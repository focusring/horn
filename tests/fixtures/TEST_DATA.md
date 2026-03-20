# Test Data for Horn

## veraPDF Corpus (Git Submodule)

The primary test corpus is the [veraPDF test corpus](https://github.com/veraPDF/veraPDF-corpus),
included as a git submodule at `tests/fixtures/verapdf-corpus/`.

**License:** Creative Commons Attribution 4.0 International (CC BY 4.0)

### Setup

Clone horn with the test data:

```bash
git clone --recurse-submodules https://github.com/focusring/horn.git
```

If you already cloned without `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

To update the corpus to the latest upstream version:

```bash
git submodule update --remote tests/fixtures/verapdf-corpus
```

### Corpus Structure

The corpus contains **434 atomic PDF test files** across two PDF/UA standards:

#### PDF/UA-1 (ISO 14289-1) — 296 files (141 pass, 155 fail)

| Section | Description | Files |
|---------|-------------|-------|
| 5 | Version identification | 10 |
| 7.1 | General (structure, metadata, tagged PDF) | 30 |
| 7.2 | Text (natural language, ActualText, Unicode) | 110 |
| 7.3 | Graphics | 5 |
| 7.4 | Headings (numbered and unnumbered) | 14 |
| 7.5 | Tables | 8 |
| 7.7 | Mathematical expressions | 5 |
| 7.9 | Notes and references | 5 |
| 7.10 | Optional content | 5 |
| 7.11 | Embedded files | 3 |
| 7.15 | XFA | 1 |
| 7.16 | Security | 2 |
| 7.18 | Annotations (forms, links, media, attachments) | 48 |
| 7.20 | XObjects | 4 |
| 7.21 | Fonts (encoding, embedding, metrics, CID, CMaps) | 46 |

#### PDF/UA-2 (ISO 14289-2) — 138 files (54 pass, 84 fail)

| Section | Description | Files |
|---------|-------------|-------|
| 5 | Version identification | 7 |
| 8.2 | Logical structure (headings, lists, tables, figures) | 49 |
| 8.4 | Text representation (language, fonts, encoding) | 67 |
| 8.6 | Text string objects | 1 |
| 8.7 | Optional content | 2 |
| 8.8 | Intra-document destinations | 2 |
| 8.9 | Annotations | 2 |
| 8.10 | Forms | 2 |
| 8.11 | Metadata | 5 |
| 8.14 | Use of embedded files | 1 |

### File Naming Convention

All test files follow this pattern:

```
{section}-t{test_number}-{pass|fail}-{variant}.pdf
```

- **section**: ISO clause number (e.g., `7.1`, `7.21.4.1`)
- **test_number**: Test case number within that clause (e.g., `t01`, `t02`)
- **pass/fail**: Expected result — `pass` means the PDF conforms, `fail` means it violates
- **variant**: Alphabetic variant (e.g., `a`, `b`, `c`) for multiple files testing the same rule

**Examples:**
- `7.1-t01-pass-a.pdf` — Section 7.1, test 1, should PASS, variant a
- `7.21.6-t02-fail-d.pdf` — Section 7.21.6, test 2, should FAIL, variant d

### Mapping to Matterhorn Protocol

The veraPDF sections map directly to Matterhorn Protocol checkpoints:

| Matterhorn Checkpoint | veraPDF Section (UA-1) | Area |
|-----------------------|------------------------|------|
| 01 | 7.1 | General / Document |
| 02 | 7.2 | Text |
| 06 | 7.1 (tagged PDF) | Tagged PDF |
| 07 | 7.4 | Headings |
| 09 | 7.5 | Tables |
| 11 | 7.3 | Graphics |
| 13 | 7.18 | Annotations |
| 14 | 7.21 | Fonts |
| 26 | 7.10 | Optional content |
| 28 | 5 | Version identification |

### Using the Test Data in Horn

#### Running all tests against the corpus

```bash
# Test against all PDF/UA-1 pass files (should report 0 violations)
find tests/fixtures/verapdf-corpus/PDF_UA-1 -name "*-pass-*.pdf" | \
  xargs -I {} cargo run -- check {}

# Test against all PDF/UA-1 fail files (should report violations)
find tests/fixtures/verapdf-corpus/PDF_UA-1 -name "*-fail-*.pdf" | \
  xargs -I {} cargo run -- check {}
```

#### Testing a specific Matterhorn checkpoint

```bash
# Test heading checks (Section 7.4 -> Matterhorn 07)
cargo run -- check "tests/fixtures/verapdf-corpus/PDF_UA-1/7.4 Headings/"

# Test font checks (Section 7.21 -> Matterhorn 14)
cargo run -- check "tests/fixtures/verapdf-corpus/PDF_UA-1/7.21 Fonts/"
```

#### In Rust integration tests

```rust
use std::path::Path;

/// Helper to collect test PDFs by section and expected result
fn corpus_pdfs(standard: &str, section: &str, expected: &str) -> Vec<std::path::PathBuf> {
    let base = Path::new("tests/fixtures/verapdf-corpus").join(standard);
    walkdir::WalkDir::new(&base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            name.ends_with(".pdf")
                && name.contains(&format!("-{}-", expected))
                && name.starts_with(section)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

#[test]
fn heading_checks_detect_failures() {
    let fails = corpus_pdfs("PDF_UA-1", "7.4", "fail");
    assert!(!fails.is_empty(), "No test files found");
    for pdf in fails {
        let result = horn::check(&pdf).unwrap();
        assert!(
            !result.violations.is_empty(),
            "Expected violations in {:?}",
            pdf
        );
    }
}

#[test]
fn heading_checks_pass_valid() {
    let passes = corpus_pdfs("PDF_UA-1", "7.4", "pass");
    assert!(!passes.is_empty(), "No test files found");
    for pdf in passes {
        let result = horn::check(&pdf).unwrap();
        assert!(
            result.violations.is_empty(),
            "Unexpected violations in {:?}: {:?}",
            pdf,
            result.violations
        );
    }
}
```

#### CI/CD integration

In your GitHub Actions workflow:

```yaml
steps:
  - uses: actions/checkout@v4
    with:
      submodules: recursive

  - name: Run corpus tests
    run: cargo test --test corpus
```

### Key Properties of the Test Files

- **Atomic**: Each file tests exactly one condition
- **Self-documented**: Document outlines explain what each file tests
- **Deterministic**: Pass/fail expectation encoded in the filename
- **Comprehensive**: Covers all machine-checkable Matterhorn Protocol failure conditions

## PDF/UA Reference Suite (Local Fixtures)

The [PDF/UA-1 Reference Suite 1.1](https://pdfa.org/resource/pdfua-reference-suite/)
from the PDF Association is included at `tests/fixtures/pdfua-reference-suite/`.

These PDF documents are the **gold standard** for PDF/UA-1 conformance. The 9
PDFUA-Ref files were created by the PDF Association's PDF/UA Competence Center as
reference material for developers and practitioners, following the
[Tagged PDF Best Practice Guide](https://pdfa.org/resource/tagged-pdf-best-practice-guide-syntax/).
The Matterhorn Protocol PDF is itself a reference-quality PDF/UA file.

**All 10 files must pass horn validation with zero errors.**

### Files

| File | Description | Pages | Size |
|------|-------------|-------|------|
| PDFUA-Ref-2-01_Magazine-danish.pdf | Danish magazine | 127 | 12.3 MB |
| PDFUA-Ref-2-02_Invoice.pdf | Invoice | 1 | 61 KB |
| PDFUA-Ref-2-03_AcademicAbstract.pdf | Academic abstract | 3 | 88 KB |
| PDFUA-Ref-2-04_Presentation.pdf | Presentation slides | 13 | 1.2 MB |
| PDFUA-Ref-2-05_BookChapter-german.pdf | German book chapter | 8 | 626 KB |
| PDFUA-Ref-2-06_Brochure.pdf | Brochure | 17 | 1.6 MB |
| PDFUA-Ref-2-08_BookChapter.pdf | Book chapter | 23 | 2.2 MB |
| PDFUA-Ref-2-09_Scanned.pdf | Scanned document (OCR) | 104 | 10.1 MB |
| PDFUA-Ref-2-10_Form.pdf | Interactive form | 1 | 57 KB |
| Matterhorn-Protocol-1-1.pdf | Matterhorn Protocol spec | 10 | 311 KB |

### What These Files Exercise

These documents cover real-world document types and exercise features that
simple atomic test files often miss:

- **Number tree `/Kids` hierarchies** in ParentTree (large documents)
- **Form field `/Parent` inheritance** for `/T` and `/TU` attributes
- **`xmlns:pdfuaid` namespace declarations** (without `pdfaExtension:schemas`)
- **`THead`/`TBody` table structure** and `/Scope` attributes on TH cells
- **Multiple languages** (Danish, German, English)
- **Scanned/OCR content** with proper tagging
- **Complex layouts** (magazines, brochures, presentations)

### Running

```bash
# Validate all reference suite files (should report 0 errors, 9/9 compliant)
cargo run --release -- validate -r tests/fixtures/pdfua-reference-suite/
```

## pdfcheck Examples (Git Submodule)

The [pdfcheck](https://github.com/jsnmrs/pdfcheck) repository is included as a git
submodule at `tests/fixtures/pdfcheck/`.

**License:** MIT

This is a browser-based PDF accessibility screening tool. Its `examples/` directory
contains 12 PDFs covering a spectrum from completely inaccessible to fully compliant,
with filenames that encode the expected outcome.

### Files

| File | Tagged | UA | Expected |
|------|--------|----|----------|
| `tagged-with-UA.pdf` | ✅ | ✅ | **pass** |
| `tagged-PAC2-pass.pdf` | ✅ | ✅ | **pass** |
| `tagged-HTML-headings-PAC-2024-pass.pdf` | ✅ | ✅ | **pass** |
| `tagged-no-UA.pdf` | ✅ | ❌ | fail (missing pdfuaid) |
| `tagged-no-UA-with-filename.pdf` | ✅ | ❌ | fail (missing pdfuaid, DisplayDocTitle) |
| `tagged-HTML-headings-chrome.pdf` | ✅ | ❌ | fail (unembedded fonts, list structure) |
| `tagged-HTML-headings-chrome-espanol.pdf` | ✅ | ❌ | fail (unembedded fonts, list structure) |
| `not-tagged.pdf` | ❌ | ❌ | fail (no tags, no UA) |
| `not-tagged-with-UA.pdf` | ❌ | ✅ | fail (no tags despite UA claim) |
| `not-tagged-with-doctitle.pdf` | ❌ | ❌ | fail (no tags, no UA) |
| `not-tagged-with-filename.pdf` | ❌ | ❌ | fail (no tags, no UA) |
| `not-tagged-with-language.pdf` | ❌ | ❌ | fail (no tags, no UA) |

### Setup

Included with the other submodules:

```bash
git submodule update --init --recursive
```

### Running

```bash
# Should pass — fully compliant files
cargo run --release -- validate tests/fixtures/pdfcheck/examples/tagged-with-UA.pdf
cargo run --release -- validate tests/fixtures/pdfcheck/examples/tagged-PAC2-pass.pdf

# Should fail — intentionally non-compliant files
cargo run --release -- validate tests/fixtures/pdfcheck/examples/not-tagged.pdf
```

### Adding More Test Data

Additional test PDFs can be placed in `tests/fixtures/` alongside the corpus.
Follow this convention for custom test files:

```
tests/fixtures/custom/{category}/{description}-{pass|fail}.pdf
```
