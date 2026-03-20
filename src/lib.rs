pub mod checks;
pub mod document;
pub mod model;
pub mod output;

use crate::checks::CheckRegistry;
use crate::document::HornDocument;
use crate::model::{FileReport, Standard};
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use crate::model::ValidationReport;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

/// Validate a PDF from in-memory bytes, auto-detecting the PDF/UA standard.
///
/// `name` is used as the display name in the report (e.g. the original filename).
pub fn validate_bytes(name: &str, data: Vec<u8>) -> FileReport {
    let mut doc = match HornDocument::from_bytes(name.to_string(), data) {
        Ok(doc) => doc,
        Err(e) => {
            return FileReport {
                path: PathBuf::from(name),
                standard: Standard::Unknown,
                results: Vec::new(),
                error: Some(format!("{e:#}")),
            };
        }
    };

    let standard = doc.standard();
    let registry = CheckRegistry::new();
    let results = registry.run_all(&mut doc, standard);

    FileReport {
        path: PathBuf::from(name),
        standard,
        results,
        error: None,
    }
}

/// Validate a single PDF file, auto-detecting the PDF/UA standard.
#[cfg(not(target_arch = "wasm32"))]
pub fn validate_file(path: &Path) -> FileReport {
    let mut doc = match HornDocument::open(path) {
        Ok(doc) => doc,
        Err(e) => {
            return FileReport {
                path: path.to_path_buf(),
                standard: Standard::Unknown,
                results: Vec::new(),
                error: Some(format!("{e:#}")),
            };
        }
    };

    let standard = doc.standard();
    let registry = CheckRegistry::new();
    let results = registry.run_all(&mut doc, standard);

    FileReport {
        path: path.to_path_buf(),
        standard,
        results,
        error: None,
    }
}

/// Validate multiple PDF files sequentially.
#[cfg(not(target_arch = "wasm32"))]
pub fn validate_files(paths: &[&Path]) -> ValidationReport {
    let files = paths.iter().map(|p| validate_file(p)).collect();
    ValidationReport { files }
}

/// Validate multiple PDF files in parallel using rayon.
///
/// When `show_progress` is false and stderr is a terminal, displays a progress bar.
#[cfg(feature = "cli")]
pub fn validate_files_parallel(paths: &[PathBuf], suppress_progress: bool) -> ValidationReport {
    use indicatif::{ProgressBar, ProgressStyle};
    use rayon::prelude::*;
    use std::io::IsTerminal;

    let show_bar = !suppress_progress && std::io::stderr().is_terminal();

    let pb = if show_bar {
        let bar = ProgressBar::new(paths.len() as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("=> "),
        );
        Some(bar)
    } else {
        None
    };

    let files: Vec<FileReport> = paths
        .par_iter()
        .map(|path| {
            let report = validate_file(path);
            if let Some(ref bar) = pb {
                bar.inc(1);
            }
            report
        })
        .collect();

    if let Some(bar) = pb {
        bar.finish_and_clear();
    }

    ValidationReport { files }
}
