use wasm_bindgen::prelude::*;

/// Validate a PDF from bytes, returning the report as a JS object.
///
/// `name` is the display filename (e.g. "report.pdf").
/// `data` is the raw PDF bytes.
#[wasm_bindgen]
pub fn validate(name: &str, data: &[u8]) -> JsValue {
    let report = horn::validate_bytes(name, data.to_vec());
    serde_wasm_bindgen::to_value(&report).unwrap_or(JsValue::NULL)
}
