# Desktop App

Horn is available as a native desktop application built with [Tauri](https://tauri.app/). It provides a visual interface for validating PDFs without using the command line.

## Install

::: code-group

```bash [macOS]
curl -sL $(curl -s https://api.github.com/repos/focusring/horn/releases/latest | grep browser_download_url | grep '.dmg"' | cut -d '"' -f 4) -o Horn.dmg && open Horn.dmg
```

```powershell [Windows]
irm ((irm https://api.github.com/repos/focusring/horn/releases/latest).assets | ? { $_.name -like '*.msi' }).browser_download_url -OutFile Horn.msi; Start-Process Horn.msi
```

```bash [Linux (deb)]
curl -sL $(curl -s https://api.github.com/repos/focusring/horn/releases/latest | grep browser_download_url | grep '.deb"' | cut -d '"' -f 4) -o horn.deb && sudo dpkg -i horn.deb
```

```bash [Linux (AppImage)]
curl -sL $(curl -s https://api.github.com/repos/focusring/horn/releases/latest | grep browser_download_url | grep '.AppImage"' | cut -d '"' -f 4) -o Horn.AppImage && chmod +x Horn.AppImage && ./Horn.AppImage
```

:::

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
