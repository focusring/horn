# Desktop App

Horn is available as a native desktop application built with [Tauri](https://tauri.app/). It provides a visual interface for validating PDFs without using the command line.

## Download

Download the latest installer for your platform from [GitHub Releases](https://github.com/focusring/horn/releases/latest):

[macOS (.dmg)](https://github.com/focusring/horn/releases/latest) · [Windows (.msi)](https://github.com/focusring/horn/releases/latest) · [Linux (.deb / .AppImage)](https://github.com/focusring/horn/releases/latest)

## Features

- Drag-and-drop or file picker to select PDFs
- Visual validation results with findings breakdown
- No terminal or command-line knowledge required
- Works offline — all validation runs locally

## How it differs from the CLI

The desktop app is ideal for occasional use or for users who prefer a graphical interface. For batch processing, CI/CD integration, or scripting, use the [CLI](/guide/getting-started) instead.

| | Desktop App | CLI |
|---|-------------|-----|
| Interface | Graphical (native window) | Terminal |
| Batch processing | Single/few files | Hundreds per second |
| CI/CD integration | No | Yes |
| Output formats | Visual report | text, JSON, SARIF, JUnit |
| Parallel processing | No | Yes (Rayon) |

## Building from source

Requires [Rust](https://rustup.rs/) 1.85+ and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
cd horn-desktop
cargo tauri build
```

The built installer will be in `horn-desktop/src-tauri/target/release/bundle/`.
