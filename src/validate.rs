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
    config: LanguageConfig,
    base: String,
    case: String,
    translation: String,
) -> Option<ValidationError> {
    None
}

//fn sanitize_string(parsed: &mut ParsedString) {
// remove trailing white-space
// remove whitespace in front of {}
// remove control chars
//}

//fn check_string(parsed: &mut ParsedString) {
// project-type, language-info, base-language
// gender-assignment in front
// all commands known and allowed in project
// plural/gender references valid
// no subindex for gender references
// no genders/cases for GS
// font-size at front
//}

//fn normalize_string(parsed: &mut ParsedString) {
// project-type
// add indexes to all parameters, including plurals/genders
//}

//fn validate_string(base: &ParsedString, trans: &ParsedString) {
// compare normalised parameters
// important parameters
// unimportant amount, but > 0
// mode-params for important parameters
//}
