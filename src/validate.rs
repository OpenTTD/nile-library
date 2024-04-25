use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct LanguageConfig {
    pub cases: Vec<String>,
    pub genders: Vec<String>,
    pub plural_count: u32,
}

#[derive(Serialize, Debug)]
pub struct ValidationError {
    pub message: String,
    pub suggestion: Option<String>,
}

/**
 * Validate whether a translation is valid for the given base string.
 *
 * @param config The language configuration to validate against.
 * @param base The base string to validate against.
 * @param case The case of the translation.
 * @param translation The translation to validate.
 *
 * @returns A clear and specific error message if the translation is invalid. None otherwise.
 */
pub fn validate(
    _config: LanguageConfig,
    _base: String,
    _case: String,
    _translation: String,
) -> Option<ValidationError> {
    None
}
