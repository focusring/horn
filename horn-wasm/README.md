<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="../docs/public/horn-dark.svg" />
    <source media="(prefers-color-scheme: dark)" srcset="../docs/public/horn-light.svg" />
    <img src="../docs/public/horn-dark.svg" alt="Horn logo" width="360" height="138" />
  </picture>
</p>

# Horn WASM

WebAssembly build of [Horn](https://github.com/focusring/horn), the open-source PDF/UA accessibility checker based on the Matterhorn Protocol.

Exposes a single `validate(name, data)` function that takes a filename and raw PDF bytes, returning the full Horn report as a JavaScript object.

## Build

```sh
wasm-pack build horn-wasm --target web
```

The output is written to `horn-wasm/pkg/`.

## Usage

```js
import init, { validate } from './pkg/horn_wasm.js'

await init()

const buffer = await file.arrayBuffer()
const report = validate(file.name, new Uint8Array(buffer))
```

## License

MIT
