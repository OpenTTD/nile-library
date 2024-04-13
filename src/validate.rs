use wasm_bindgen::prelude::*;

/**
 * Validate whether a translation is valid for the given base string.
 *
 * @param base The base string to validate against.
 * @param case The case of the translation.
 * @param translation The translation to validate.
 *
 * @returns A clear and specific error message if the translation is invalid. None otherwise.
 */
#[wasm_bindgen]
pub fn validate(base: String, case: String, translation: String) -> Option<String> {
    None
}
