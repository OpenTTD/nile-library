use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

mod validate;

#[wasm_bindgen]
pub fn validate(js_config: JsValue, base: String, case: String, translation: String) -> JsValue {
    let config: validate::LanguageConfig = serde_wasm_bindgen::from_value(js_config).unwrap();
    let response = validate::validate(config, base, case, translation);

    if let Some(error) = response {
        serde_wasm_bindgen::to_value(&error).unwrap()
    } else {
        JsValue::NULL
    }
}

#[wasm_bindgen]
pub fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}
