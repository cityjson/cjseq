//! WASM bindings for CityJSONSeq operations
//!
//! This module provides WebAssembly bindings for the cjseq library,
//! allowing JavaScript/TypeScript applications to process CityJSONSeq
//! files directly in the browser or Node.js.

use crate::{cjseq_to_cj, CityJSON, CityJSONFeature};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

/// Initialize the WASM module with panic hook for better error messages
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Convert a base CityJSON metadata and array of CityJSONFeatures into a complete CityJSON object
///
/// # Arguments
/// * `base_cj` - The base CityJSON metadata object (typically first line of CityJSONSeq)
/// * `features` - Array of CityJSONFeature objects
///
/// # Returns
/// * `Result<JsValue, JsValue>` - Complete CityJSON object or error
///
/// # Example
/// ```javascript
/// import init, { cjseqToCj } from './cjseq.js';
///
/// await init();
/// const result = cjseqToCj(baseCityJSON, featuresArray);
/// ```
#[wasm_bindgen(js_name = cjseqToCj)]
pub fn cjseq_to_cj_wasm(base_cj: JsValue, features: JsValue) -> Result<JsValue, JsValue> {
    let base_cj: CityJSON = match from_value(base_cj) {
        Ok(cj) => cj,
        Err(e) => {
            web_sys::console::error_1(&format!("failed to deserialize base_cj: {}", e).into());
            return Err(JsValue::from_str(&format!(
                "failed to parse base_cj: {}",
                e
            )));
        }
    };

    let features: Vec<CityJSONFeature> = match from_value(features) {
        Ok(f) => f,
        Err(e) => {
            web_sys::console::error_1(&format!("failed to deserialize features: {}", e).into());
            return Err(JsValue::from_str(&format!(
                "failed to parse features: {}",
                e
            )));
        }
    };

    let cj = cjseq_to_cj(base_cj, features);

    match to_value(&cj) {
        Ok(js_val) => Ok(js_val),
        Err(e) => {
            web_sys::console::error_1(&format!("failed to serialize cj: {}", e).into());
            Err(JsValue::from_str(&format!("failed to serialize cj: {}", e)))
        }
    }
}
