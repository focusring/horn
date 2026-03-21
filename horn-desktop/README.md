<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="../docs/public/horn-dark.svg" />
    <source media="(prefers-color-scheme: dark)" srcset="../docs/public/horn-light.svg" />
    <img src="../docs/public/horn-dark.svg" alt="Horn logo" width="360" height="138" />
  </picture>
</p>

# Horn Desktop

Native desktop app for [Horn](https://github.com/focusring/horn), the open-source PDF/UA accessibility checker based on the Matterhorn Protocol.

Built with [Tauri 2](https://tauri.app/) — a lightweight native shell around the Horn validation engine with a file picker dialog for selecting PDFs.

## Prerequisites

- Rust 2024 edition (1.85+)
- [Tauri CLI](https://tauri.app/start/)

## Development

```sh
cargo tauri dev -p horn-desktop
```

## Build

```sh
cargo tauri build -p horn-desktop
```

## License

MIT
