# Getting Started

## Installation

### Pre-built binaries

Install with a single command:

::: code-group

```bash [macOS (Apple Silicon)]
curl -sL https://github.com/focusring/horn/releases/latest/download/horn-macos-aarch64 \
  -o /usr/local/bin/horn && chmod +x /usr/local/bin/horn
```

```bash [macOS (Intel)]
curl -sL https://github.com/focusring/horn/releases/latest/download/horn-macos-x86_64 \
  -o /usr/local/bin/horn && chmod +x /usr/local/bin/horn
```

```bash [Linux (x86_64)]
curl -sL https://github.com/focusring/horn/releases/latest/download/horn-linux-x86_64 \
  -o /usr/local/bin/horn && chmod +x /usr/local/bin/horn
```

```bash [Linux (aarch64)]
curl -sL https://github.com/focusring/horn/releases/latest/download/horn-linux-aarch64 \
  -o /usr/local/bin/horn && chmod +x /usr/local/bin/horn
```

```powershell [Windows]
Invoke-WebRequest -Uri https://github.com/focusring/horn/releases/latest/download/horn-windows-x86_64.exe -OutFile horn.exe
```

:::

Or download manually from [GitHub Releases](https://github.com/focusring/horn/releases/latest).

### From source

Requires [Rust](https://rustup.rs/) 1.85+ (edition 2024):

```bash
cargo install --git https://github.com/focusring/horn.git
```

### Docker

```bash
docker build -t horn .
docker run -v $(pwd):/data horn validate /data/document.pdf
```

### Desktop App

Download the native desktop app from [GitHub Releases](https://github.com/focusring/horn/releases/latest):

| Platform | Format |
|----------|--------|
| macOS | `.dmg` |
| Windows | `.msi` |
| Linux | `.deb` / `.AppImage` |

See the [Desktop App guide](/guide/desktop-app) for more details.

### WebAssembly (npm)

Use Horn in the browser or Node.js via the WASM package:

```bash
npm install @focusring/horn-wasm
```

See the [WebAssembly guide](/guide/wasm) for API details and usage examples.

### GitHub Action

Add PDF accessibility checks to your CI pipeline:

```yaml
- uses: focusring/horn@v1
  with:
    path: ./output/
```

See [CI/CD Integration](/guide/ci-cd) and [GitHub Action Reference](/reference/github-action) for all options.

## Quick start

Validate a single PDF:

```bash
horn validate document.pdf
```

Validate a directory of PDFs:

```bash
horn validate ./pdfs/ --recurse
```

Get JSON output:

```bash
horn validate document.pdf --format json
```

See all available checks:

```bash
horn list-checks
```

## Shell completions

Generate completions for your shell:

```bash
# Bash
horn completions bash > ~/.local/share/bash-completion/completions/horn

# Zsh
horn completions zsh > ~/.zfunc/_horn

# Fish
horn completions fish > ~/.config/fish/completions/horn.fish
```

## Next steps

- [Validating PDFs](/guide/validating-pdfs) — learn about all validation options
- [Output Formats](/guide/output-formats) — choose the right format for your use case
- [CI/CD Integration](/guide/ci-cd) — automate accessibility checks in your pipeline
