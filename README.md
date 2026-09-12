<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="docs/public/horn-dark.svg" />
    <source media="(prefers-color-scheme: dark)" srcset="docs/public/horn-light.svg" />
    <img src="docs/public/horn-dark.svg" alt="Horn logo" width="360" height="138" />
  </picture>
</p>

# Horn

Open-source PDF/UA accessibility checker based on the Matterhorn Protocol.

Horn validates PDF files against PDF/UA-1 (ISO 14289-1) and covers **every failure condition of the [Matterhorn Protocol 1.1](https://pdfa.org/resource/the-matterhorn-protocol/)** (137 index entries): the 87 machine-checkable conditions are implemented as automated checks, the 48 human-judgment conditions are reported as manual-review items, and the 2 without a defined test are reported as not applicable. Every finding carries the official Matterhorn index (e.g. `28-010`). Horn scores 100% on the [veraPDF PDF/UA-1 test corpus](https://github.com/veraPDF/veraPDF-corpus) (297 files) and passes the PDF Association's PDF/UA-1 Reference Suite. PDF/UA-2 (ISO 14289-2) documents get the same checks plus 37 PDF/UA-2-specific rules (PDF 2.0 namespaces, structure destinations, annotation artifacts, `pdfuaid:rev`) and score 100% on the veraPDF PDF/UA-2 corpus (138 files). It is designed as a cross-platform, CI/CD-ready alternative to [PAC 2024](https://pac.pdf-accessibility.org/).

## Features

- **Fast**: ~660 PDFs/second with parallel processing (release mode)
- **CI/CD native**: SARIF (GitHub Code Scanning), JUnit XML, JSON output formats
- **Cross-platform**: Linux, macOS, Windows — no JVM or GUI required
- **Complete Matterhorn coverage**: 87/87 machine-checkable conditions automated, 48/48 human-judgment conditions surfaced for manual review (`horn coverage` prints the matrix)
- **PDF/UA-1 and PDF/UA-2**: the standard is auto-detected from `pdfuaid:part`; PDF/UA-2 rules use interim `ua2:<clause>-<test>` ids until the Matterhorn Protocol 2.0 is published
- **Deep font analysis**: parses embedded TrueType, CFF and Type 1 font programs to verify glyph coverage, CharSet/CIDSet, widths, cmap subtables and Unicode mapping
- **Extensible**: `Check` trait for adding custom checks

## Installation

### Pre-built binaries

Download from [GitHub Releases](https://github.com/focusring/horn/releases).

### From source

```bash
cargo install --git https://github.com/focusring/horn.git
```

### Docker

```bash
docker build -t horn .
docker run -v $(pwd):/data horn validate /data/document.pdf
```

## Usage

```bash
# Single file
horn validate document.pdf

# Multiple files
horn validate *.pdf --format json

# Directory (recursive)
horn validate ./docs/ --recurse --format sarif -o results.sarif

# CI mode: fail pipeline if any errors
horn validate ./output/ --recurse --format junit --fail-on error -o results.xml

# Only fail on warnings or worse (ignore info-level findings)
horn validate doc.pdf --fail-on warning

# Include manual-review items (human-judgment Matterhorn conditions) in text output
horn validate doc.pdf --review

# Show the Matterhorn Protocol coverage matrix
horn coverage

# List available checks
horn list-checks

# Generate shell completions
horn completions bash > ~/.local/share/bash-completion/completions/horn
horn completions zsh > ~/.zfunc/_horn
horn completions fish > ~/.config/fish/completions/horn.fish
```

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | All files compliant |
| 1 | One or more files have findings at or above `--fail-on` severity |
| 2 | CLI or runtime error |

## GitHub Action

```yaml
- uses: focusring/horn@v0
  with:
    path: ./output/
    format: sarif
    fail-on: error
```

The action automatically uploads SARIF results to GitHub Code Scanning when `format: sarif`.

## CI/CD Examples

### GitHub Actions (manual)

```yaml
steps:
  - uses: actions/checkout@v4
  - name: Install Horn
    run: |
      curl -sL https://github.com/focusring/horn/releases/latest/download/horn-linux-x86_64 \
        -o /usr/local/bin/horn && chmod +x /usr/local/bin/horn
  - name: Validate PDFs
    run: horn validate ./docs/ --recurse --format sarif -o results.sarif
  - uses: github/codeql-action/upload-sarif@v3
    if: always()
    with:
      sarif_file: results.sarif
```

### GitLab CI

```yaml
pdf-accessibility:
  image: rust:latest
  script:
    - cargo install --git https://github.com/focusring/horn.git
    - horn validate ./docs/ --recurse --format junit -o results.xml
  artifacts:
    reports:
      junit: results.xml
```

## Checks

Horn implements checks across these Matterhorn Protocol checkpoints:

| Module | Checkpoint | What it validates |
|--------|-----------|-------------------|
| baseline | 01-31 | pdf_oxide built-in PDF/UA-1 validation |
| metadata | 06 | Document language, title display, PDF/UA identifier, XMP |
| structure | 01/02/09 | Tagged PDF, StructTreeRoot, MarkInfo, role mapping |
| fonts | 31 | Font embedding, ToUnicode CMaps, encoding |
| headings | 14 | H1-H6 hierarchy, no skipped levels |
| tables | 15 | TR/TH/TD structure, header identification |
| images | 13 | Figure alt text presence |
| annotations | 28 | Tab order, link destinations, widget accessibility |
| lists | 16 | L/LI/Lbl/LBody nesting |
| ua2/* | — | PDF/UA-2 only: identification schema, PDF 2.0 namespaces, structure destinations, annotation and form rules |

Use `horn list-checks` to see all registered checks.

## Output formats

- **text**: Human-readable terminal output (default)
- **json**: Structured JSON report
- **sarif**: [SARIF v2.1.0](https://sarifweb.azurewebsites.net/) for GitHub Code Scanning
- **junit**: JUnit XML for CI dashboards (Jenkins, GitLab, etc.)

## Test data

Horn uses the [veraPDF test corpus](https://github.com/veraPDF/veraPDF-corpus) (CC BY 4.0) as a git submodule. See [tests/fixtures/TEST_DATA.md](tests/fixtures/TEST_DATA.md) for details.

```bash
git clone --recurse-submodules https://github.com/focusring/horn.git
horn validate tests/fixtures/verapdf-corpus/PDF_UA-1/ --recurse
horn validate tests/fixtures/verapdf-corpus/PDF_UA-2/ --recurse
```

## License

Licensed under the [MIT License](LICENSE).
