# What is Horn?

Horn is an open-source PDF/UA accessibility checker based on the [Matterhorn Protocol 1.1](https://pdfa.org/resource/the-matterhorn-protocol/). It validates PDF files against PDF/UA-1 (ISO 14289-1), targeting the machine-checkable failure conditions defined in the protocol.

Horn is designed as a cross-platform, CI/CD-ready alternative to [PAC 2024](https://pac.pdf-accessibility.org/).

## Why Horn?

Most PDF accessibility checkers are slow, GUI-only, or platform-locked. Horn takes a different approach:

- **Built in Rust** for raw speed — validate 500+ PDFs per second with parallel processing
- **Single binary** with zero runtime dependencies — no JVM, no GUI frameworks
- **CI/CD first** — SARIF, JUnit XML, and JSON outputs integrate directly into your pipeline
- **Cross-platform** — Linux, macOS, and Windows from the same codebase

## How it works

Horn uses a dual-parser architecture for optimal performance:

1. **pdf_oxide** parses the PDF structure tree eagerly (~4ms per file) and runs built-in PDF/UA-1 compliance checks
2. **lopdf** provides raw PDF object access and is initialized lazily — only when checks need low-level access

This means simple PDFs are validated extremely quickly, while complex documents still get thorough checking.

## Matterhorn Protocol

The [Matterhorn Protocol](https://pdfa.org/resource/the-matterhorn-protocol/) defines 136 failure conditions across 31 checkpoints for PDF/UA-1. Horn implements 21 check modules covering these checkpoints, including:

- Document metadata and language
- Structure tree and tagging
- Font embedding and encoding
- Heading hierarchy
- Table structure
- Image alt text
- Annotation accessibility
- List nesting
- And more

Run `horn list-checks` to see all registered checks.

## Platforms

Horn is available as:

- **CLI** — the primary interface, suitable for local use and CI/CD
- **GitHub Action** — drop-in action with automatic SARIF upload
- **Web app** — browser-based validation via WebAssembly
- **Desktop app** — native app built with Tauri

## License

Horn is dual-licensed under MIT and Apache 2.0 — use whichever fits your project.
