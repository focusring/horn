<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="../docs/public/horn-dark.svg" />
    <source media="(prefers-color-scheme: dark)" srcset="../docs/public/horn-light.svg" />
    <img src="../docs/public/horn-dark.svg" alt="Horn logo" width="360" height="138" />
  </picture>
</p>

# Horn GUI

Web-based GUI for [Horn](https://github.com/focusring/horn), the open-source PDF/UA accessibility checker based on the Matterhorn Protocol.

Built with **Axum** + **htmx** + **Askama** templates — lightweight, server-rendered, no JavaScript framework required.

## Features

- Drag-and-drop or file-picker upload for one or many PDFs
- Single PDF: shows results directly with charts and findings table
- Multiple PDFs: shows a summary list with per-file links to detailed results
- Visual stats: outcome donut chart and severity breakdown bars
- JSON API endpoint for programmatic access

## Getting Started

### Prerequisites

- Rust 2024 edition (1.85+)
- Cargo

### Run

```sh
cargo run -p horn-gui
```

The server starts at **http://127.0.0.1:3000**.

### Development

```
horn-gui/
  src/main.rs        # Axum routes, validation logic, state management
  templates/          # Askama HTML templates
    index.html        # Upload page
    results_detail.html   # Single-file detail view with charts
    results_summary.html  # Multi-file batch summary
  static/
    style.css         # All styles (pure CSS, no build step)
    htmx.min.js       # htmx library
```

Templates use [Askama](https://github.com/djc/askama) (compile-time checked Jinja2-like syntax). Changes to templates require a recompile.

### API

**POST** `/api/validate` — multipart file upload, returns JSON:

```json
{
  "files": [
    {
      "filename": "doc.pdf",
      "compliant": false,
      "passed": 20,
      "failed": 3,
      "needs_review": 1,
      "findings": [...]
    }
  ]
}
```

## License

MIT
