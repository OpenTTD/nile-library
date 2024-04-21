use crate::commands::{CommandInfo, Dialect, Occurence, COMMANDS};
use crate::parser::{FragmentContent, ParsedString, StringCommand};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Deserialize, Debug)]
pub struct LanguageConfig {
    pub dialect: String, //< "newgrf", "game-script", "openttd"
    pub cases: Vec<String>,
    pub genders: Vec<String>,
    pub plural_count: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct ValidationError {
    pub critical: bool, //< true: translation is broken, do not commit. false: translation has minor issues, but is probably better than no translation
    pub position: Option<usize>, //< byte offset in input string
    pub message: String,
    pub suggestion: Option<String>,
}

impl LanguageConfig {
    fn get_dialect(&self) -> Dialect {
        match self.dialect.as_str() {
            "newgrf" => Dialect::NEWGRF,
            "game-script" => Dialect::GAMESCRIPT,
            "openttd" => Dialect::OPENTTD,
            _ => panic!(),
        }
    }
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

struct StringSignature {
    parameters: HashMap<usize, &'static CommandInfo<'static>>,
    nonpositional_count: HashMap<String, (Occurence, usize)>,
    // TODO track color/lineno/colorstack for positional parameters
}

fn get_signature(
    dialect: &Dialect,
    base: &ParsedString,
) -> Result<StringSignature, Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut signature = StringSignature {
        parameters: HashMap::new(),
        nonpositional_count: HashMap::new(),
    };

    let mut pos = 0;
    for fragment in &base.fragments {
        if let FragmentContent::Command(cmd) = &fragment.content {
            if let Some(info) = COMMANDS
                .into_iter()
                .find(|ci| ci.name == cmd.name && ci.dialects.contains(&dialect))
            {
                if info.parameters.is_empty() {
                    if let Some(index) = cmd.index {
                        errors.push(ValidationError {
                            critical: true,
                            position: Some(fragment.position),
                            message: format!(
                                "Command '{{{}}}' cannot have a position reference.",
                                cmd.name
                            ),
                            suggestion: Some(format!("Remove '{}:'.", index)),
                        });
                    }
                    let norm_name = String::from(info.get_norm_name());
                    if let Some(existing) = signature.nonpositional_count.get_mut(&norm_name) {
                        existing.1 += 1;
                    } else {
                        signature
                            .nonpositional_count
                            .insert(norm_name, (info.occurence.clone(), 1));
                    }
                } else {
                    if let Some(index) = cmd.index {
                        pos = index;
                    }
                    if let Some(existing) = signature.parameters.insert(pos, info) {
                        errors.push(ValidationError {
                            critical: true,
                            position: Some(fragment.position),
                            message: format!(
                                "Command '{{{}:{}}}' references the same position as '{{{}:{}}}' before.",
                                pos, cmd.name, pos, existing.name
                            ),
                            suggestion: Some(String::from("Assign unique position references.")),
                        });
                    }
                    pos += 1;
                }
            } else {
                errors.push(ValidationError {
                    critical: true,
                    position: Some(fragment.position),
                    message: format!("Unknown string command '{{{}}}'.", cmd.name),
                    suggestion: None,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(signature)
    } else {
        Err(errors)
    }
}

//fn normalize_string(parsed: &mut ParsedString) {
// project-type
// add indexes to all parameters, including plurals/genders
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

    #[test]
    fn test_signature_empty() {
        let parsed = ParsedString::parse("").unwrap();
        let sig = get_signature(&Dialect::OPENTTD, &parsed).unwrap();
        assert!(sig.parameters.is_empty());
        assert!(sig.nonpositional_count.is_empty());
    }

    #[test]
    fn test_signature_pos() {
        let parsed = ParsedString::parse("{P a b}{RED}{NUM}{NBSP}{MONO_FONT}{5:STRING.foo}{RED}{2:STRING3.bar}{RAW_STRING}{G c d}").unwrap();
        let sig = get_signature(&Dialect::OPENTTD, &parsed).unwrap();
        assert_eq!(sig.parameters.len(), 4);
        assert_eq!(sig.parameters.get(&0).unwrap().name, "NUM");
        assert_eq!(sig.parameters.get(&5).unwrap().name, "STRING");
        assert_eq!(sig.parameters.get(&2).unwrap().name, "STRING3");
        assert_eq!(sig.parameters.get(&3).unwrap().name, "RAW_STRING");
        assert_eq!(sig.nonpositional_count.len(), 3);
        assert_eq!(
            sig.nonpositional_count.get("RED"),
            Some(&(Occurence::NONZERO, 2))
        );
        assert_eq!(
            sig.nonpositional_count.get("MONO_FONT"),
            Some(&(Occurence::EXACT, 1))
        );
        assert_eq!(
            sig.nonpositional_count.get("NBSP"),
            Some(&(Occurence::ANY, 1))
        );
    }

    #[test]
    fn test_signature_dup() {
        let parsed = ParsedString::parse("{NUM}{0:COMMA}").unwrap();
        let err = get_signature(&Dialect::OPENTTD, &parsed).err().unwrap();
        assert_eq!(err.len(), 1);
        assert_eq!(
            err[0],
            ValidationError {
                critical: true,
                position: Some(5),
                message: String::from(
                    "Command '{0:COMMA}' references the same position as '{0:NUM}' before."
                ),
                suggestion: Some(String::from("Assign unique position references.")),
            }
        );
    }

    #[test]
    fn test_signature_dialect() {
        let parsed = ParsedString::parse("{RAW_STRING}").unwrap();

        let sig = get_signature(&Dialect::OPENTTD, &parsed).unwrap();
        assert_eq!(sig.parameters.len(), 1);
        assert_eq!(sig.parameters.get(&0).unwrap().name, "RAW_STRING");
        assert_eq!(sig.nonpositional_count.len(), 0);

        let err = get_signature(&Dialect::NEWGRF, &parsed).err().unwrap();
        assert_eq!(err.len(), 1);
        assert_eq!(
            err[0],
            ValidationError {
                critical: true,
                position: Some(0),
                message: String::from("Unknown string command '{RAW_STRING}'."),
                suggestion: None,
            }
        );
    }

    #[test]
    fn test_signature_unknown() {
        let parsed = ParsedString::parse("{FOOBAR}").unwrap();
        let err = get_signature(&Dialect::OPENTTD, &parsed).err().unwrap();
        assert_eq!(err.len(), 1);
        assert_eq!(
            err[0],
            ValidationError {
                critical: true,
                position: Some(0),
                message: String::from("Unknown string command '{FOOBAR}'."),
                suggestion: None,
            }
        );
    }

    #[test]
    fn test_signature_nonpos() {
        let parsed = ParsedString::parse("{1:RED}").unwrap();
        let err = get_signature(&Dialect::OPENTTD, &parsed).err().unwrap();
        assert_eq!(err.len(), 1);
        assert_eq!(
            err[0],
            ValidationError {
                critical: true,
                position: Some(0),
                message: String::from("Command '{RED}' cannot have a position reference."),
                suggestion: Some(String::from("Remove '1:'.")),
            }
        );
    }
}
