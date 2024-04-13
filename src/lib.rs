use wasm_bindgen::prelude::*;

mod validate;

#[wasm_bindgen]
pub fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}
