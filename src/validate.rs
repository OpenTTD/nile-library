use crate::parser::{FragmentContent, ParsedString, StringCommand};
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

fn remove_ascii_ctrl(t: &mut String) {
    *t = t.replace(|c| char::is_ascii_control(&c), " ");
}

fn remove_trailing_blanks(t: &mut String) {
    if let Some(last) = t.rfind(|c| c != ' ') {
        t.truncate(last + 1);
    }
}

/// Replace all ASCII control codes with blank.
/// Remove trailing blanks at end of each line.
fn sanitize_whitespace(parsed: &mut ParsedString) {
    let mut is_eol = true;
    for i in (0..parsed.fragments.len()).rev() {
        let mut is_nl = false;
        match &mut parsed.fragments[i].content {
            FragmentContent::Text(t) => {
                remove_ascii_ctrl(t);
                if is_eol {
                    remove_trailing_blanks(t);
                }
            }
            FragmentContent::Choice(c) => {
                for t in &mut c.choices {
                    remove_ascii_ctrl(t);
                }
            }
            FragmentContent::Command(c) => {
                is_nl = c.name.is_empty();
            }
            _ => (),
        }
        is_eol = is_nl;
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize() {
        let mut s1 = String::from("");
        let mut s2 = String::from(" a b c ");
        let mut s3 = String::from("\0a\tb\rc\r\n");
        remove_ascii_ctrl(&mut s1);
        remove_ascii_ctrl(&mut s2);
        remove_ascii_ctrl(&mut s3);
        assert_eq!(s1, String::from(""));
        assert_eq!(s2, String::from(" a b c "));
        assert_eq!(s3, String::from(" a b c  "));
        remove_trailing_blanks(&mut s1);
        remove_trailing_blanks(&mut s2);
        remove_trailing_blanks(&mut s3);
        assert_eq!(s1, String::from(""));
        assert_eq!(s2, String::from(" a b c"));
        assert_eq!(s3, String::from(" a b c"));
    }
}
