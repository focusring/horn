# WebAssembly

Horn compiles to WebAssembly, allowing you to validate PDFs directly in the browser with no server required. This powers the Horn web app and can be used in your own applications.

Try the [live demo](/demo) to see it in action.

## Installation

Install from npm:

```bash
npm install @focusring/horn-wasm
```

## Building from source

Alternatively, build from source. Requires [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) or `wasm-bindgen-cli`:

```bash
# Build the WASM package
cargo build -p horn-wasm --target wasm32-unknown-unknown --release

# Generate JS bindings
wasm-bindgen \
  --target web \
  --out-dir horn-wasm/pkg \
  target/wasm32-unknown-unknown/release/horn_wasm.wasm

# Optional: optimize for size
wasm-opt -O3 --enable-bulk-memory \
  horn-wasm/pkg/horn_wasm_bg.wasm \
  -o horn-wasm/pkg/horn_wasm_bg.wasm
```

## API

The WASM build exposes a single `validate` function:

```typescript
/**
 * Validate a PDF from raw bytes.
 *
 * @param name - Display filename (e.g. "report.pdf")
 * @param data - Raw PDF bytes (Uint8Array)
 * @returns A FileReport object with validation results
 */
function validate(name: string, data: Uint8Array): FileReport
```

## Usage in the browser

```html
<script type="module">
  import init, { validate } from './pkg/horn_wasm.js'

  await init()

  const input = document.querySelector('input[type="file"]')
  input.addEventListener('change', async (e) => {
    const file = e.target.files[0]
    const bytes = new Uint8Array(await file.arrayBuffer())
    const report = validate(file.name, bytes)

    console.log(report)
    // {
    //   path: "report.pdf",
    //   standard: "pdf-ua-1",
    //   results: [...],
    //   error: null
    // }
  })
</script>
```

## Usage with a bundler

```javascript
import init, { validate } from '@focusring/horn-wasm'

await init()

const response = await fetch('/document.pdf')
const bytes = new Uint8Array(await response.arrayBuffer())
const report = validate('document.pdf', bytes)
```

## Return format

The `validate` function returns the same `FileReport` structure as the CLI's JSON output:

```json
{
  "path": "document.pdf",
  "standard": "pdf-ua-1",
  "results": [
    {
      "rule_id": "06-001",
      "checkpoint": 6,
      "description": "Document language not set",
      "severity": "error",
      "outcome": {
        "status": "Fail",
        "message": "No /Lang entry in document catalog"
      }
    }
  ],
  "error": null
}
```

## Performance notes

- The WASM build runs single-threaded — parallel processing (`rayon`) is not available
- Initial PDF structure parsing via `pdf_oxide` is fast (~4ms per file)
- The `lopdf` parser is initialized lazily and only triggered when checks need low-level PDF object access
- Complex PDFs that require `lopdf` may take longer in WASM compared to native (~3s vs ~0.4s) due to single-threaded decompression

## Differences from the CLI

| Feature | CLI | WASM |
|---------|-----|------|
| Parallel processing | Yes (Rayon) | No (single-threaded) |
| File system access | Yes | No (bytes only) |
| Progress bars | Yes | No |
| Output formatting | text, JSON, SARIF, JUnit | Raw `FileReport` object |
| Shell completions | Yes | N/A |
