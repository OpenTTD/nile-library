use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

mod commands;
mod parser;
mod validate;

#[wasm_bindgen]
pub fn validate_base(js_config: JsValue, base: String) -> JsValue {
    let config: validate::LanguageConfig = serde_wasm_bindgen::from_value(js_config).unwrap();
    let response = validate::validate_base(config, base);
    serde_wasm_bindgen::to_value(&response).unwrap()
}

#[wasm_bindgen]
pub fn validate_translation(
    js_config: JsValue,
    base: String,
    case: String,
    translation: String,
) -> JsValue {
    let config: validate::LanguageConfig = serde_wasm_bindgen::from_value(js_config).unwrap();
    let response = validate::validate_translation(config, base, case, translation);
    serde_wasm_bindgen::to_value(&response).unwrap()
}

#[wasm_bindgen]
pub fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}
