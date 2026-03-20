use crate::model::Standard;
use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
use std::cell::OnceCell;
use std::path::{Path, PathBuf};

/// Unified PDF document wrapper combining `pdf_oxide` (structure tree, compliance)
/// and `lopdf` (raw object access) parsers.
///
/// When loaded via `from_bytes`, `lopdf` parsing is deferred until first access.
/// This makes the initial load fast (pdf_oxide only), which is critical for WASM
/// where lopdf's eager stream decompression is very slow.
pub struct HornDocument {
    oxide: pdf_oxide::PdfDocument,
    lopdf: OnceCell<lopdf::Document>,
    /// Raw PDF bytes kept for lazy lopdf init and for checks that need raw access.
    pdf_bytes: Option<Vec<u8>>,
    path: PathBuf,
    standard: Standard,
}

impl HornDocument {
    /// Open a PDF file with both parsers (eagerly).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(path: &Path) -> Result<Self> {
        let path_str = path.to_str().context("Path contains invalid UTF-8")?;

        let oxide = pdf_oxide::PdfDocument::open(path_str)
            .map_err(|e| anyhow::anyhow!("pdf_oxide failed to open: {e}"))?;

        let lopdf = lopdf::Document::load(path)
            .with_context(|| format!("lopdf failed to open: {}", path.display()))?;

        let standard = detect_standard_from_xmp(&lopdf);

        let cell = OnceCell::new();
        cell.set(lopdf).ok();

        Ok(Self {
            oxide,
            lopdf: cell,
            pdf_bytes: None,
            path: path.to_path_buf(),
            standard,
        })
    }

    /// Load a PDF from in-memory bytes.
    ///
    /// Only `pdf_oxide` is parsed eagerly (~4ms). `lopdf` parsing is deferred
    /// until a check calls `doc.lopdf()`, avoiding the expensive stream
    /// decompression cost upfront (critical for WASM performance).
    pub fn from_bytes(name: String, bytes: Vec<u8>) -> Result<Self> {
        // Detect standard by scanning raw bytes for XMP pdfuaid:part string.
        // This avoids needing lopdf for standard detection.
        let standard = detect_standard_from_raw(&bytes);

        let oxide = pdf_oxide::PdfDocument::from_bytes(bytes.clone())
            .map_err(|e| anyhow::anyhow!("pdf_oxide failed to parse: {e}"))?;

        Ok(Self {
            oxide,
            lopdf: OnceCell::new(),
            pdf_bytes: Some(bytes),
            path: PathBuf::from(name),
            standard,
        })
    }

    /// The detected PDF/UA standard (UA-1, UA-2, or Unknown).
    pub fn standard(&self) -> Standard {
        self.standard
    }

    /// Get the file path (or synthetic name when loaded from bytes).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Access the raw PDF bytes, if loaded via `from_bytes`.
    pub fn raw_bytes(&self) -> Option<&[u8]> {
        self.pdf_bytes.as_deref()
    }

    /// Access the `pdf_oxide` document for structure tree and compliance checks.
    pub fn oxide(&mut self) -> &mut pdf_oxide::PdfDocument {
        &mut self.oxide
    }

    /// Access the `lopdf` document for raw PDF object inspection.
    ///
    /// When loaded via `from_bytes`, this triggers lazy parsing on first call.
    pub fn lopdf(&self) -> &lopdf::Document {
        self.lopdf.get_or_init(|| {
            if let Some(bytes) = &self.pdf_bytes {
                lopdf::Document::load_mem(bytes)
                    .expect("lopdf failed to parse PDF bytes (lazy init)")
            } else {
                panic!("lopdf not initialized and no bytes available")
            }
        })
    }

    /// Get the document catalog dictionary via lopdf.
    pub fn raw_catalog(&self) -> Result<&lopdf::Dictionary> {
        self.lopdf()
            .catalog()
            .map_err(|e| anyhow::anyhow!("Failed to read catalog: {e}"))
    }
}

/// Detect the PDF/UA standard by scanning raw bytes for the XMP `pdfuaid:part` string.
///
/// This avoids needing to parse the PDF with lopdf just for standard detection.
fn detect_standard_from_raw(bytes: &[u8]) -> Standard {
    let text = String::from_utf8_lossy(bytes);
    detect_standard_from_xmp_string(&text)
}

/// Detect the PDF/UA standard from the document's XMP metadata via lopdf.
#[cfg(not(target_arch = "wasm32"))]
fn detect_standard_from_xmp(doc: &lopdf::Document) -> Standard {
    let Ok(catalog) = doc.catalog() else {
        return Standard::Unknown;
    };

    let meta_ref = match catalog.get(b"Metadata") {
        Ok(obj) => match obj.as_reference() {
            Ok(r) => r,
            Err(_) => return Standard::Unknown,
        },
        Err(_) => return Standard::Unknown,
    };

    let meta_obj = match doc.get_object(meta_ref) {
        Ok(obj) => obj,
        Err(_) => return Standard::Unknown,
    };

    let stream = match meta_obj.as_stream() {
        Ok(s) => s,
        Err(_) => return Standard::Unknown,
    };

    let content = match stream.get_plain_content() {
        Ok(c) => c,
        Err(_) => return Standard::Unknown,
    };

    let xmp = String::from_utf8_lossy(&content);
    detect_standard_from_xmp_string(&xmp)
}

/// Parse the pdfuaid:part value from XMP metadata string.
///
/// Handles both element syntax (`<pdfuaid:part>2</pdfuaid:part>`) and
/// attribute syntax (`pdfuaid:part="2"`), as used by UA-1 and UA-2 respectively.
fn detect_standard_from_xmp_string(xmp: &str) -> Standard {
    // Try element syntax first: <pdfuaid:part>N</pdfuaid:part>
    let elem_start = "<pdfuaid:part>";
    let elem_end = "</pdfuaid:part>";
    if let Some(start_idx) = xmp.find(elem_start) {
        let value_start = start_idx + elem_start.len();
        if let Some(end_idx) = xmp[value_start..].find(elem_end) {
            let value = xmp[value_start..value_start + end_idx].trim();
            return match value {
                "1" => Standard::Ua1,
                "2" => Standard::Ua2,
                _ => Standard::Unknown,
            };
        }
    }

    // Try attribute syntax: pdfuaid:part="N"
    if let Some(idx) = xmp.find("pdfuaid:part=\"") {
        let value_start = idx + "pdfuaid:part=\"".len();
        if let Some(end_idx) = xmp[value_start..].find('"') {
            let value = xmp[value_start..value_start + end_idx].trim();
            return match value {
                "1" => Standard::Ua1,
                "2" => Standard::Ua2,
                _ => Standard::Unknown,
            };
        }
    }

    Standard::Unknown
}
