# Horn Architecture

Horn is a PDF/UA accessibility checker based on the Matterhorn Protocol. It ships as a CLI, a browser-based web app (WASM), and a native desktop app (Tauri) — all powered by the same Rust core library.

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      horn (core library)                     │
│  validate_bytes() / validate_file() → FileReport             │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────────────┐  │
│  │ pdf_oxide│  │  lopdf   │  │ CheckRegistry (25 checks) │  │
│  │  (fast)  │  │  (lazy)  │  │ Matterhorn Protocol       │  │
│  └──────────┘  └──────────┘  └───────────────────────────┘  │
└──────────┬──────────────┬──────────────────┬────────────────┘
           │              │                  │
     ┌─────▼─────┐  ┌────▼──────┐  ┌───────▼────────┐
     │    CLI    │  │  horn-gui │  │ horn-desktop   │
     │  (Rust)  │  │  (WASM)   │  │  (Tauri)       │
     │          │  │           │  │                │
     │ parallel │  │ browser + │  │ native macOS/  │
     │ + output │  │ WebWorker │  │ Linux/Windows  │
     │ formats  │  │           │  │                │
     └──────────┘  └───────────┘  └────────────────┘
```

## Projects

| Project | Location | Purpose |
|---------|----------|---------|
| `horn` (lib + CLI) | `/horn/` | Core library + command-line tool |
| `horn-gui` | `/horn/horn-gui/` | Web app (Axum server + WASM frontend) |
| `horn-wasm` | `/horn/horn-wasm/` | WebAssembly build of the core library |
| `horn-desktop` | `/horn-desktop/` | Native desktop app (Tauri, separate repo) |

## Core Library (`horn`)

### Public API

```rust
// Validate from in-memory bytes (used by WASM + desktop)
pub fn validate_bytes(name: &str, data: Vec<u8>) -> FileReport

// Validate from a file path (used by CLI)
pub fn validate_file(path: &Path) -> FileReport

// Validate multiple files in parallel (CLI only, behind "cli" feature)
pub fn validate_files_parallel(paths: &[PathBuf], suppress_progress: bool) -> ValidationReport
```

All three targets call `validate_bytes()` — it's the universal entry point.

### HornDocument (Dual-Parser Architecture)

Horn uses **two PDF parsers** for different purposes:

- **pdf_oxide** — Structure tree, compliance validation, built-in PDF/UA-1 checks
- **lopdf** — Raw PDF object access (dictionaries, streams, metadata)

```rust
pub struct HornDocument {
    oxide: pdf_oxide::PdfDocument,       // Always parsed eagerly (~4ms)
    lopdf: OnceCell<lopdf::Document>,    // Lazy — parsed on first access
    pdf_bytes: Option<Vec<u8>>,          // Kept for lazy lopdf init
    path: PathBuf,
    standard: Standard,                  // UA-1, UA-2, or Unknown
}
```

**Why lazy lopdf?** `lopdf` decompresses all PDF streams during parsing. For a 5MB image-heavy PDF, this takes ~5s in WASM vs ~0.4s native. By deferring lopdf until a check actually needs it, initial load stays fast.

- `from_bytes()` — Parses only pdf_oxide. Lopdf deferred. (WASM/desktop)
- `open(path)` — Parses both eagerly. (CLI, native speed)

### Check System

25 checks implementing the `Check` trait, registered in `CheckRegistry`:

```rust
pub trait Check: Send + Sync {
    fn id(&self) -> &'static str;           // module id, e.g. "31-font-program"
    fn checkpoint(&self) -> u8;             // Matterhorn checkpoint (1-31)
    fn description(&self) -> &'static str;
    fn rules(&self) -> &'static [&'static str]; // Matterhorn ids this check emits
    fn supports(&self, standard: Standard) -> bool;
    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>>;
}
```

Every `CheckResult.rule_id` is an official Matterhorn Protocol 1.1 index
(`"28-010"`) or a Horn extension id (`"15-x04"`). The catalogue of all 136
conditions lives in `src/matterhorn.rs`; `CheckRegistry::implemented_rules()`
and the `horn coverage` command derive the coverage matrix from it.

Checks cover: file syntax, structure, metadata, language, fonts (dictionary
and font-program level), headings, tables, lists, images, math, notes,
annotations, optional content, embedded files, XFA, security, XObjects, and a
`human_review` check that reports the 48 human-judgment conditions applicable
to the document as `NeedsReview`.

### Supporting modules

- `src/matterhorn.rs` — the Matterhorn 1.1 failure-condition catalogue (id, checkpoint, clause, machine/human).
- `src/content/` — content-stream analysis shared by checks: which character codes each font shows (pages, Form XObjects, annotation appearances), text rendering modes, Form XObject paint counts; plus CMap parsers (encoding CMaps and ToUnicode).
- `src/fontprog/` — dependency-free parsers for embedded font programs: TrueType/OpenType (via `ttf-parser`), CFF/Type1C, Type 1 (eexec, charstrings), the Adobe Glyph List and the predefined simple-font encodings.

### Data Model

```
FileReport
├── path: PathBuf
├── standard: Standard (Ua1 | Ua2 | Unknown)
├── error: Option<String>
└── results: Vec<CheckResult>
    └── CheckResult
        ├── rule_id: String         ("28-010", Matterhorn index)
        ├── checkpoint: u8          (6)
        ├── description: String
        ├── severity: Severity      (Error | Warning | Info)
        └── outcome: CheckOutcome   (Pass | Fail | NeedsReview | NotApplicable)
```

### Feature Flags

```toml
[features]
default = ["cli"]
cli = ["rayon", "indicatif", "walkdir", "clap", "env_logger", "lopdf/rayon", ...]
```

- **`cli` (default)** — Enables parallel validation, progress bars, directory walking, CLI parsing
- **Without `cli`** — Minimal library for WASM/desktop (no rayon, no clap, no terminal deps)

### Output Formats

The CLI supports four output formats: `text`, `json`, `sarif`, `junit`.

## Data Flow Per Target

### CLI

```
$ horn validate *.pdf -f json

Files/directories (clap)
  → collect_pdf_paths() [walkdir if --recurse]
  → validate_files_parallel() [rayon par_iter]
    → HornDocument::open(path)      # eager, both parsers
    → CheckRegistry::run_all()      # 25 checks
    → FileReport
  → output::write_report()          # text/json/sarif/junit
  → stdout or file
  → exit code (0=compliant, 1=failures)
```

### Web GUI (WASM)

```
Browser: user drops PDF
  → file.arrayBuffer()
  → postMessage to Web Worker
    → WASM: validate(name, Uint8Array)
      → horn::validate_bytes()
        → HornDocument::from_bytes()  # pdf_oxide eager, lopdf lazy
        → CheckRegistry::run_all()
        → FileReport → serde_wasm_bindgen → JsValue
    → postMessage back to main thread
  → JS renders results as HTML
```

```
┌────────────┐     postMessage      ┌────────────┐
│ Main Thread│ ──────────────────► │ Web Worker │
│            │                     │            │
│ index.html │ ◄────────────────── │ worker.js  │
│ (renders)  │     postMessage      │ (WASM)     │
└────────────┘                     └────────────┘
```

The Axum server (`horn-gui`) only serves static files — no server-side validation.

### Desktop (Tauri)

```
User clicks "Choose & Validate PDFs"
  → JS: invoke('pick_and_validate')
  → Rust: native file picker dialog
  → Rust: std::fs::read(path) for each file
  → Rust: horn::validate_bytes(name, data)
    → HornDocument::from_bytes()
    → CheckRegistry::run_all()
    → FileReport
  → Rust → JS: serde_json::Value (IPC)
  → JS renders results as HTML
```

```
┌─────────────────┐    invoke()     ┌──────────────────┐
│ System Webview  │ ─────────────► │ Tauri Backend    │
│                 │                │ (native Rust)    │
│ src/index.html  │ ◄───────────── │ horn::validate() │
│ (renders)       │    JSON IPC    │ + file dialog    │
└─────────────────┘                └──────────────────┘
```

## Performance Characteristics

| Target | "Home V1.pdf" (5.3MB) | Simple PDF | Why |
|--------|----------------------|------------|-----|
| CLI (native, release) | ~0.4s | ~0.07s | Native Rust, parallel, SIMD zlib |
| Desktop (Tauri, release) | ~0.4s | ~0.07s | Same — direct Rust call |
| Web (WASM) | ~3s | ~0.07s | zlib decompression slow in WASM |

The WASM bottleneck is `lopdf`'s eager stream decompression running in single-threaded WASM without SIMD. The lazy lopdf optimization helps by deferring this cost, but it's still triggered when lopdf-dependent checks run (23 of 25 checks).

## File Structure

```
/horn/                              # Cargo workspace
├── Cargo.toml                      # Workspace: [., horn-gui, horn-wasm, horn-desktop/src-tauri]; [workspace.package] holds the shared version
├── src/
│   ├── lib.rs                      # Public API: validate_bytes, validate_file
│   ├── main.rs                     # CLI binary (clap, output formatting)
│   ├── document.rs                 # HornDocument (dual parser, lazy lopdf, cached content usage)
│   ├── model.rs                    # Standard, Severity, CheckResult, FileReport
│   ├── matterhorn.rs               # Matterhorn 1.1 condition catalogue
│   ├── content/                    # Content-stream font usage + CMap parsers
│   ├── fontprog/                   # TrueType / CFF / Type 1 font program parsers, AGL, encodings
│   ├── checks/
│   │   ├── mod.rs                  # Check trait + CheckRegistry
│   │   ├── baseline.rs             # pdf_oxide built-in UA-1 validation
│   │   ├── fonts.rs                # Font dictionaries: embedding, CMaps, encodings
│   │   ├── font_program.rs         # Font programs: glyphs, CharSet/CIDSet, widths, cmaps, ToUnicode
│   │   ├── human_review.rs         # Manual-review items for human-judgment conditions
│   │   ├── tables.rs               # Table structure (TH/TD/Headers/Scope/grid)
│   │   ├── ... (19 more)
│   │   └── xobjects.rs             # Form XObject paint counts
│   └── output/
│       ├── mod.rs                  # OutputFormat enum
│       ├── text.rs                 # Human-readable output
│       ├── sarif.rs                # SARIF format
│       └── junit.rs                # JUnit XML format
│
├── horn-gui/                       # Web app
│   ├── Cargo.toml                  # axum, tokio, tower-http
│   ├── src/main.rs                 # Static file server (port 3000)
│   └── static/
│       ├── index.html              # SPA frontend + client-side rendering
│       ├── style.css               # Theming (light/dark/system)
│       └── worker.js               # Web Worker ↔ WASM bridge
│
└── horn-wasm/                      # WASM build
    ├── Cargo.toml                  # wasm-bindgen, serde-wasm-bindgen
    ├── src/lib.rs                  # #[wasm_bindgen] pub fn validate()
    └── pkg/                        # Build output (JS glue + .wasm binary)

/horn-desktop/                      # Separate project
├── src-tauri/
│   ├── Cargo.toml                  # tauri 2, horn (path dep), dialog plugin
│   ├── src/main.rs                 # pick_and_validate command
│   └── tauri.conf.json             # App config, bundling
└── src/
    ├── index.html                  # Frontend (Tauri invoke instead of WASM)
    └── style.css                   # Same styles as horn-gui
```

## Build Commands

```bash
# CLI
cargo build --release                    # → target/release/horn

# WASM (for web GUI)
cargo build -p horn-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir horn-wasm/pkg target/wasm32-unknown-unknown/release/horn_wasm.wasm
wasm-opt -O3 --enable-bulk-memory -o horn-wasm/pkg/horn_wasm_bg.wasm horn-wasm/pkg/horn_wasm_bg.wasm

# Web GUI
cargo run -p horn-gui                    # → http://127.0.0.1:3000

# Desktop (dev)
cd /horn-desktop && cargo tauri dev --release

# Desktop (distributable)
cd /horn-desktop && cargo tauri build    # → .app / .dmg / .msi / .AppImage
```

## Versioning and Releases

All four crates share one version number, declared once in `[workspace.package]`
in the root `Cargo.toml` and inherited with `version.workspace = true`. Non-Cargo
files that embed the version are derived from it:

| File | Used by |
|---|---|
| `horn-desktop/src-tauri/tauri.conf.json` | Tauri bundle version → installer names (`Horn_0.3.0_amd64.deb`, …) |
| `docs/package.json` | VitePress nav (`v0.3.0`) |
| `horn-desktop/src/index.html`, `horn-gui/static/index.html` | GUI footer |
| `horn-wasm/pkg/package.json` | Generated by `wasm-pack`; the release workflow sets it from the tag |
| `docs/public/wasm/` | Not committed: `docs/scripts/sync-wasm.mjs` downloads the released `@focusring/horn-wasm` matching `docs/package.json` before every docs build, so the live demo runs the documented release |

`scripts/sync-version.sh` propagates the workspace version into the first three
rows and `scripts/check-version.sh [expected]` verifies that they all agree with
Cargo. The `horn-wasm/pkg/package.json` row is different: `wasm-pack` generates
it during the release build, and the release workflow stamps the tag's version
into it before publishing.
CI runs the check on every push and pull request; the release workflow runs it
against the tag before building anything.

### Cutting a release

1. Bump `[workspace.package].version` in `Cargo.toml` and run
   `scripts/sync-version.sh` (or let `cargo release <level>` do both — it is
   configured in `[workspace.metadata.release]` with the sync script as its
   pre-release hook and never publishes to crates.io).
2. Commit (`release: vX.Y.Z`), merge to `main`, and push the tag `vX.Y.Z`.
3. `.github/workflows/release.yml` then:
   - verifies the tag against `scripts/check-version.sh`;
   - runs the full CI workflow (lint, tests, corpus validation);
   - builds the CLI for Linux (x86_64, aarch64 musl), macOS (x86_64, aarch64)
     and Windows (x86_64);
   - builds the WASM package with `wasm-pack` and publishes
     `@focusring/horn-wasm@X.Y.Z` to npm (skipped if that version exists);
   - builds the Tauri desktop app (`.dmg`, `.app.tar.gz`, `.deb`, `.AppImage`,
     `.msi`);
   - fails if any of those assets is missing, otherwise creates the GitHub
     Release with generated notes;
   - force-moves the major tag (`v0`, later `v1`) to the release so
     `uses: focusring/horn@v0` in the GitHub Action always resolves to the
     latest release of that major line.

`horn-gui` is a development server for the WASM front-end and is not
distributed; the docs site is deployed by Vercel from `main`.
