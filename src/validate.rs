use crate::commands::{CommandInfo, Dialect, Occurence, COMMANDS};
use crate::parser::{FragmentContent, ParsedString};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Deserialize, Debug)]
pub struct LanguageConfig {
    pub dialect: String, //< "newgrf", "game-script", "openttd"
    pub cases: Vec<String>,
    pub genders: Vec<String>,
    pub plural_count: usize,
}

#[derive(Debug, PartialEq)]
pub enum Severity {
    Error,   //< translation is broken, do not commit.
    Warning, //< translation has minor issues, but is probably better than no translation.
}

#[derive(Serialize, Debug, PartialEq)]
pub struct ValidationError {
    pub severity: Severity,
    pub pos_begin: Option<usize>, //< codepoint offset in input string
    pub pos_end: Option<usize>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub normalized: Option<String>,
}

impl LanguageConfig {
    fn get_dialect(&self) -> Dialect {
        self.dialect.as_str().into()
    }

    pub fn allow_cases(&self) -> bool {
        self.get_dialect() != Dialect::GAMESCRIPT
    }

    fn allow_genders(&self) -> bool {
        self.get_dialect() != Dialect::GAMESCRIPT
    }
}

impl Serialize for Severity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
        })
    }
}

/**
 * Validate whether a base string is valid.
 *
 * @param config The language configuration of the base language. (dialect and plural form)
 * @param base The base string to validate.
 *
 * @returns A normalized form of the base string for translators, and a list of error messages, if the base is invalid.
 */
pub fn validate_base(config: &LanguageConfig, base: &String) -> ValidationResult {
    let mut base = match ParsedString::parse(&base) {
        Err(err) => {
            return ValidationResult {
                errors: vec![ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(err.pos_begin),
                    pos_end: err.pos_end,
                    message: err.message,
                    suggestion: None,
                }],
                normalized: None,
            };
        }
        Ok(parsed) => parsed,
    };
    let errs = validate_string(&config, &base, None);
    if errs.iter().any(|e| e.severity == Severity::Error) {
        ValidationResult {
            errors: errs,
            normalized: None,
        }
    } else {
        sanitize_whitespace(&mut base);
        normalize_string(&config.get_dialect(), &mut base);
        ValidationResult {
            errors: errs,
            normalized: Some(base.compile()),
        }
    }
}

/**
 * Validate whether a translation is valid for the given base string.
 *
 * @param config The language configuration to validate against.
 * @param base The base string to validate against.
 * @param case The case of the translation. Use "default" for the default case.
 * @param translation The translation to validate.
 *
 * @returns A normalized form of the translation, and a list of error messages, if the translation is invalid.
 */
pub fn validate_translation(
    config: &LanguageConfig,
    base: &String,
    case: &String,
    translation: &String,
) -> ValidationResult {
    let base = match ParsedString::parse(&base) {
        Err(_) => {
            return ValidationResult {
                errors: vec![ValidationError {
                    severity: Severity::Error,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("Base language text is invalid."),
                    suggestion: Some(String::from("This is a bug; wait until it is fixed.")),
                }],
                normalized: None,
            };
        }
        Ok(parsed) => parsed,
    };
    if case != "default" {
        if !config.allow_cases() {
            return ValidationResult {
                errors: vec![ValidationError {
                    severity: Severity::Error,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("No cases allowed."),
                    suggestion: None,
                }],
                normalized: None,
            };
        } else if !config.cases.contains(&case) {
            return ValidationResult {
                errors: vec![ValidationError {
                    severity: Severity::Error,
                    pos_begin: None,
                    pos_end: None,
                    message: format!("Unknown case '{}'.", case),
                    suggestion: Some(format!("Known cases are: '{}'", config.cases.join("', '"))),
                }],
                normalized: None,
            };
        }
    }
    let mut translation = match ParsedString::parse(&translation) {
        Err(err) => {
            return ValidationResult {
                errors: vec![ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(err.pos_begin),
                    pos_end: err.pos_end,
                    message: err.message,
                    suggestion: None,
                }],
                normalized: None,
            };
        }
        Ok(parsed) => parsed,
    };
    let errs = validate_string(&config, &translation, Some(&base));
    if errs.iter().any(|e| e.severity == Severity::Error) {
        ValidationResult {
            errors: errs,
            normalized: None,
        }
    } else {
        sanitize_whitespace(&mut translation);
        normalize_string(&config.get_dialect(), &mut translation);
        ValidationResult {
            errors: errs,
            normalized: Some(translation.compile()),
        }
    }
}

fn remove_ascii_ctrl(t: &mut String) {
    *t = t.replace(|c| char::is_ascii_control(&c), " ");
}

fn remove_trailing_blanks(t: &mut String) {
    t.truncate(t.trim_end().len());
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
    parameters: HashMap<usize, (&'static CommandInfo<'static>, usize)>,
    nonpositional_count: BTreeMap<String, (Occurence, usize)>,
    // TODO track color/lineno/colorstack for positional parameters
}

fn get_signature(
    dialect: &Dialect,
    base: &ParsedString,
) -> Result<StringSignature, Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut signature = StringSignature {
        parameters: HashMap::new(),
        nonpositional_count: BTreeMap::new(),
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
                            severity: Severity::Error,
                            pos_begin: Some(fragment.pos_begin),
                            pos_end: Some(fragment.pos_end),
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
                    if let Some(existing) = signature.parameters.get_mut(&pos) {
                        existing.1 += 1;
                    } else {
                        signature.parameters.insert(pos, (info, 1));
                    }
                    pos += 1;
                }
            } else {
                errors.push(ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(fragment.pos_begin),
                    pos_end: Some(fragment.pos_end),
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

fn validate_string(
    config: &LanguageConfig,
    test: &ParsedString,
    base: Option<&ParsedString>,
) -> Vec<ValidationError> {
    let dialect = config.get_dialect();
    let signature: StringSignature;
    match get_signature(&dialect, base.unwrap_or(test)) {
        Ok(sig) => signature = sig,
        Err(msgs) => {
            if base.is_some() {
                return vec![ValidationError {
                    severity: Severity::Error,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("Base language text is invalid."),
                    suggestion: Some(String::from("This is a bug; wait until it is fixed.")),
                }];
            } else {
                return msgs;
            }
        }
    }

    let mut errors = Vec::new();
    let mut positional_count: HashMap<usize, usize> = HashMap::new();
    let mut nonpositional_count: BTreeMap<String, (Occurence, usize)> = BTreeMap::new();
    let mut pos = 0;
    let mut front = 0;
    for fragment in &test.fragments {
        match &fragment.content {
            FragmentContent::Command(cmd) => {
                let opt_expected = signature
                    .parameters
                    .get(&cmd.index.unwrap_or(pos))
                    .map(|v| (*v).0);
                let opt_info =
                    opt_expected
                        .filter(|ex| ex.get_norm_name() == cmd.name)
                        .or(COMMANDS
                            .into_iter()
                            .find(|ci| ci.name == cmd.name && ci.dialects.contains(&dialect)));
                if let Some(info) = opt_info {
                    if let Some(c) = &cmd.case {
                        if !config.allow_cases() {
                            errors.push(ValidationError {
                                severity: Severity::Error,
                                pos_begin: Some(fragment.pos_begin),
                                pos_end: Some(fragment.pos_end),
                                message: String::from("No case selections allowed."),
                                suggestion: Some(format!("Remove '.{}'.", c)),
                            });
                        } else if !info.allow_case {
                            errors.push(ValidationError {
                                severity: Severity::Error,
                                pos_begin: Some(fragment.pos_begin),
                                pos_end: Some(fragment.pos_end),
                                message: format!(
                                    "No case selection allowed for '{{{}}}'.",
                                    cmd.name
                                ),
                                suggestion: Some(format!("Remove '.{}'.", c)),
                            });
                        } else if !config.cases.contains(&c) {
                            errors.push(ValidationError {
                                severity: Severity::Error,
                                pos_begin: Some(fragment.pos_begin),
                                pos_end: Some(fragment.pos_end),
                                message: format!("Unknown case '{}'.", c),
                                suggestion: Some(format!(
                                    "Known cases are: '{}'",
                                    config.cases.join("', '")
                                )),
                            });
                        }
                    }

                    if info.parameters.is_empty() {
                        if let Some(index) = cmd.index {
                            errors.push(ValidationError {
                                severity: Severity::Error,
                                pos_begin: Some(fragment.pos_begin),
                                pos_end: Some(fragment.pos_end),
                                message: format!(
                                    "Command '{{{}}}' cannot have a position reference.",
                                    cmd.name
                                ),
                                suggestion: Some(format!("Remove '{}:'.", index)),
                            });
                        }

                        let norm_name = String::from(info.get_norm_name());
                        if let Some(existing) = nonpositional_count.get_mut(&norm_name) {
                            existing.1 += 1;
                        } else {
                            nonpositional_count.insert(norm_name, (info.occurence.clone(), 1));
                        }
                    } else {
                        if let Some(index) = cmd.index {
                            pos = index;
                        }

                        if let Some(expected) = opt_expected {
                            if expected.get_norm_name() == info.get_norm_name() {
                                if let Some(existing) = positional_count.get_mut(&pos) {
                                    *existing += 1;
                                } else {
                                    positional_count.insert(pos, 1);
                                }
                            } else {
                                errors.push(ValidationError {
                                    severity: Severity::Error,
                                    pos_begin: Some(fragment.pos_begin),
                                    pos_end: Some(fragment.pos_end),
                                    message: format!(
                                        "Expected '{{{}:{}}}', found '{{{}}}'.",
                                        pos, expected.name, cmd.name
                                    ),
                                    suggestion: None,
                                })
                            }
                        } else {
                            errors.push(ValidationError {
                                severity: Severity::Error,
                                pos_begin: Some(fragment.pos_begin),
                                pos_end: Some(fragment.pos_end),
                                message: format!(
                                    "There is no parameter in position {}, found '{{{}}}'.",
                                    pos, cmd.name
                                ),
                                suggestion: None,
                            });
                        }

                        pos += 1;
                    }
                } else {
                    errors.push(ValidationError {
                        severity: Severity::Error,
                        pos_begin: Some(fragment.pos_begin),
                        pos_end: Some(fragment.pos_end),
                        message: format!("Unknown string command '{{{}}}'.", cmd.name),
                        suggestion: None,
                    });
                }
                front = 2;
            }
            FragmentContent::Gender(g) => {
                if !config.allow_genders() || config.genders.len() < 2 {
                    errors.push(ValidationError {
                        severity: Severity::Error,
                        pos_begin: Some(fragment.pos_begin),
                        pos_end: Some(fragment.pos_end),
                        message: String::from("No gender definitions allowed."),
                        suggestion: Some(String::from("Remove '{G=...}'.")),
                    });
                } else if front == 2 {
                    errors.push(ValidationError {
                        severity: Severity::Warning,
                        pos_begin: Some(fragment.pos_begin),
                        pos_end: Some(fragment.pos_end),
                        message: String::from("Gender definitions must be at the front."),
                        suggestion: Some(String::from(
                            "Move '{G=...}' to the front of the translation.",
                        )),
                    });
                } else if front == 1 {
                    errors.push(ValidationError {
                        severity: Severity::Warning,
                        pos_begin: Some(fragment.pos_begin),
                        pos_end: Some(fragment.pos_end),
                        message: String::from("Duplicate gender definition."),
                        suggestion: Some(String::from("Remove the second '{G=...}'.")),
                    });
                } else {
                    front = 1;
                    if !config.genders.contains(&g.gender) {
                        errors.push(ValidationError {
                            severity: Severity::Error,
                            pos_begin: Some(fragment.pos_begin),
                            pos_end: Some(fragment.pos_end),
                            message: format!("Unknown gender '{}'.", g.gender),
                            suggestion: Some(format!(
                                "Known genders are: '{}'",
                                config.genders.join("', '")
                            )),
                        });
                    }
                }
            }
            FragmentContent::Choice(cmd) => {
                let opt_ref_pos = match cmd.name.as_str() {
                    "P" => {
                        if pos == 0 {
                            None
                        } else {
                            Some(pos - 1)
                        }
                    }
                    "G" => Some(pos),
                    _ => panic!(),
                };
                let opt_ref_pos = cmd.indexref.or(opt_ref_pos);
                if cmd.name == "G" && (!config.allow_genders() || config.genders.len() < 2) {
                    errors.push(ValidationError {
                        severity: Severity::Error,
                        pos_begin: Some(fragment.pos_begin),
                        pos_end: Some(fragment.pos_end),
                        message: String::from("No gender choices allowed."),
                        suggestion: Some(String::from("Remove '{G ...}'.")),
                    });
                } else if cmd.name == "P" && config.plural_count < 2 {
                    errors.push(ValidationError {
                        severity: Severity::Error,
                        pos_begin: Some(fragment.pos_begin),
                        pos_end: Some(fragment.pos_end),
                        message: String::from("No plural choices allowed."),
                        suggestion: Some(String::from("Remove '{P ...}'.")),
                    });
                } else {
                    match cmd.name.as_str() {
                        "P" => {
                            if cmd.choices.len() != config.plural_count {
                                errors.push(ValidationError {
                                    severity: Severity::Error,
                                    pos_begin: Some(fragment.pos_begin),
                                    pos_end: Some(fragment.pos_end),
                                    message: format!(
                                        "Expected {} plural choices, found {}.",
                                        config.plural_count,
                                        cmd.choices.len()
                                    ),
                                    suggestion: None,
                                });
                            }
                        }
                        "G" => {
                            if cmd.choices.len() != config.genders.len() {
                                errors.push(ValidationError {
                                    severity: Severity::Error,
                                    pos_begin: Some(fragment.pos_begin),
                                    pos_end: Some(fragment.pos_end),
                                    message: format!(
                                        "Expected {} gender choices, found {}.",
                                        config.genders.len(),
                                        cmd.choices.len()
                                    ),
                                    suggestion: None,
                                });
                            }
                        }
                        _ => panic!(),
                    };

                    if let Some(ref_info) = opt_ref_pos
                        .and_then(|ref_pos| signature.parameters.get(&ref_pos).map(|v| v.0))
                    {
                        let ref_pos = opt_ref_pos.unwrap();
                        let ref_norm_name = ref_info.get_norm_name();
                        let ref_subpos = match cmd.name.as_str() {
                            "P" => cmd
                                .indexsubref
                                .or(ref_info.def_plural_subindex)
                                .unwrap_or(0),
                            "G" => cmd.indexsubref.unwrap_or(0),
                            _ => panic!(),
                        };
                        if let Some(par_info) = ref_info.parameters.get(ref_subpos) {
                            match cmd.name.as_str() {
                                "P" => {
                                    if !par_info.allow_plural {
                                        errors.push(ValidationError{
                                            severity: Severity::Error,
                                            pos_begin: Some(fragment.pos_begin),
                                            pos_end: Some(fragment.pos_end),
                                            message: format!(
                                                "'{{{}}}' references position '{}:{}', but '{{{}:{}}}' does not allow plurals.",
                                                cmd.name, ref_pos, ref_subpos, ref_pos, ref_norm_name
                                            ),
                                            suggestion: None,
                                        });
                                    }
                                }
                                "G" => {
                                    if !par_info.allow_gender {
                                        errors.push(ValidationError{
                                            severity: Severity::Error,
                                            pos_begin: Some(fragment.pos_begin),
                                            pos_end: Some(fragment.pos_end),
                                            message: format!(
                                                "'{{{}}}' references position '{}:{}', but '{{{}:{}}}' does not allow genders.",
                                                cmd.name, ref_pos, ref_subpos, ref_pos, ref_norm_name
                                            ),
                                            suggestion: None,
                                        });
                                    }
                                }
                                _ => panic!(),
                            };
                        } else {
                            errors.push(ValidationError{
                                severity: Severity::Error,
                                pos_begin: Some(fragment.pos_begin),
                                pos_end: Some(fragment.pos_end),
                                message: format!(
                                    "'{{{}}}' references position '{}:{}', but '{{{}:{}}}' only has {} subindices.",
                                    cmd.name, ref_pos, ref_subpos, ref_pos, ref_norm_name, ref_info.parameters.len()
                                ),
                                suggestion: None,
                            });
                        }
                    } else {
                        errors.push(ValidationError {
                            severity: Severity::Error,
                            pos_begin: Some(fragment.pos_begin),
                            pos_end: Some(fragment.pos_end),
                            message: format!(
                                "'{{{}}}' references position '{}', which has no parameter.",
                                cmd.name,
                                opt_ref_pos
                                    .and_then(|v| isize::try_from(v).ok())
                                    .unwrap_or(-1)
                            ),
                            suggestion: if cmd.indexref.is_none() {
                                Some(String::from("Add a position reference."))
                            } else {
                                None
                            },
                        });
                    }
                }
                front = 2;
            }
            FragmentContent::Text(_) => {
                front = 2;
            }
        }
    }

    for (pos, (info, ex_count)) in &signature.parameters {
        let norm_name = info.get_norm_name();
        let found_count = positional_count.get(pos).cloned().unwrap_or(0);
        if info.occurence != Occurence::ANY && found_count == 0 {
            errors.push(ValidationError {
                severity: Severity::Error,
                pos_begin: None,
                pos_end: None,
                message: format!("String command '{{{}:{}}}' is missing.", pos, norm_name),
                suggestion: None,
            });
        } else if info.occurence == Occurence::EXACT && *ex_count != found_count {
            errors.push(ValidationError {
                severity: Severity::Warning,
                pos_begin: None,
                pos_end: None,
                message: format!(
                    "String command '{{{}:{}}}': expected {} times, found {} times.",
                    pos, norm_name, ex_count, found_count
                ),
                suggestion: None,
            });
        }
    }

    for (norm_name, (occurence, ex_count)) in &signature.nonpositional_count {
        let found_count = nonpositional_count.get(norm_name).map(|v| v.1).unwrap_or(0);
        if *occurence != Occurence::ANY && found_count == 0 {
            errors.push(ValidationError {
                severity: Severity::Warning,
                pos_begin: None,
                pos_end: None,
                message: format!("String command '{{{}}}' is missing.", norm_name),
                suggestion: None,
            });
        } else if *occurence == Occurence::EXACT && *ex_count != found_count {
            errors.push(ValidationError {
                severity: Severity::Warning,
                pos_begin: None,
                pos_end: None,
                message: format!(
                    "String command '{{{}}}': expected {} times, found {} times.",
                    norm_name, ex_count, found_count
                ),
                suggestion: None,
            });
        }
    }
    for (norm_name, (occurence, _)) in &nonpositional_count {
        if *occurence != Occurence::ANY && signature.nonpositional_count.get(norm_name).is_none() {
            errors.push(ValidationError {
                severity: Severity::Warning,
                pos_begin: None,
                pos_end: None,
                message: format!("String command '{{{}}}' is unexpected.", norm_name),
                suggestion: Some(String::from("Remove this command.")),
            });
        }
    }

    errors
}

fn normalize_string(dialect: &Dialect, parsed: &mut ParsedString) {
    let mut parameters = HashMap::new();

    let mut pos = 0;
    for fragment in &mut parsed.fragments {
        match &mut fragment.content {
            FragmentContent::Command(cmd) => {
                if let Some(info) = COMMANDS
                    .into_iter()
                    .find(|ci| ci.name == cmd.name && ci.dialects.contains(&dialect))
                {
                    if let Some(norm_name) = info.norm_name {
                        // normalize name
                        cmd.name = String::from(norm_name);
                    }
                    if !info.parameters.is_empty() {
                        if let Some(index) = cmd.index {
                            pos = index;
                        } else {
                            // add missing indices
                            cmd.index = Some(pos);
                        }
                        parameters.insert(pos, info);
                        pos += 1;
                    }
                }
            }
            FragmentContent::Choice(cmd) => {
                match cmd.name.as_str() {
                    "P" => {
                        if cmd.indexref.is_none() && pos > 0 {
                            // add missing indices
                            cmd.indexref = Some(pos - 1);
                        }
                    }
                    "G" => {
                        if cmd.indexref.is_none() {
                            // add missing indices
                            cmd.indexref = Some(pos);
                        }
                    }
                    _ => panic!(),
                };
            }
            _ => (),
        }
    }

    for fragment in &mut parsed.fragments {
        if let FragmentContent::Choice(cmd) = &mut fragment.content {
            if let Some(ref_info) = cmd.indexref.and_then(|pos| parameters.get(&pos)) {
                if cmd.indexsubref == ref_info.def_plural_subindex.or(Some(0)) {
                    // remove subindex, if default
                    cmd.indexsubref = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize() {
        let mut s1 = String::from("");
        let mut s2 = String::from(" a b c ");
        let mut s3 = String::from("\0a\tb\rc\r\n");
        let mut s4 = String::from("abc\u{b3}");
        remove_ascii_ctrl(&mut s1);
        remove_ascii_ctrl(&mut s2);
        remove_ascii_ctrl(&mut s3);
        remove_ascii_ctrl(&mut s4);
        assert_eq!(s1, String::from(""));
        assert_eq!(s2, String::from(" a b c "));
        assert_eq!(s3, String::from(" a b c  "));
        assert_eq!(s4, String::from("abc\u{b3}"));
        remove_trailing_blanks(&mut s1);
        remove_trailing_blanks(&mut s2);
        remove_trailing_blanks(&mut s3);
        remove_trailing_blanks(&mut s4);
        assert_eq!(s1, String::from(""));
        assert_eq!(s2, String::from(" a b c"));
        assert_eq!(s3, String::from(" a b c"));
        assert_eq!(s4, String::from("abc\u{b3}"));
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
        let parsed = ParsedString::parse("{P a b}{RED}{NUM}{NBSP}{MONO_FONT}{5:STRING.foo}{RED}{2:STRING3.bar}{RAW_STRING}{3:RAW_STRING}{G c d}").unwrap();
        let sig = get_signature(&Dialect::OPENTTD, &parsed).unwrap();
        assert_eq!(sig.parameters.len(), 4);
        assert_eq!(sig.parameters.get(&0).unwrap().0.name, "NUM");
        assert_eq!(sig.parameters.get(&0).unwrap().1, 1);
        assert_eq!(sig.parameters.get(&5).unwrap().0.name, "STRING");
        assert_eq!(sig.parameters.get(&5).unwrap().1, 1);
        assert_eq!(sig.parameters.get(&2).unwrap().0.name, "STRING3");
        assert_eq!(sig.parameters.get(&2).unwrap().1, 1);
        assert_eq!(sig.parameters.get(&3).unwrap().0.name, "RAW_STRING");
        assert_eq!(sig.parameters.get(&3).unwrap().1, 2);
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
    fn test_signature_dialect() {
        let parsed = ParsedString::parse("{RAW_STRING}").unwrap();

        let sig = get_signature(&Dialect::OPENTTD, &parsed).unwrap();
        assert_eq!(sig.parameters.len(), 1);
        assert_eq!(sig.parameters.get(&0).unwrap().0.name, "RAW_STRING");
        assert_eq!(sig.parameters.get(&0).unwrap().1, 1);
        assert_eq!(sig.nonpositional_count.len(), 0);

        let err = get_signature(&Dialect::NEWGRF, &parsed).err().unwrap();
        assert_eq!(err.len(), 1);
        assert_eq!(
            err[0],
            ValidationError {
                severity: Severity::Error,
                pos_begin: Some(0),
                pos_end: Some(12),
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
                severity: Severity::Error,
                pos_begin: Some(0),
                pos_end: Some(8),
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
                severity: Severity::Error,
                pos_begin: Some(0),
                pos_end: Some(7),
                message: String::from("Command '{RED}' cannot have a position reference."),
                suggestion: Some(String::from("Remove '1:'.")),
            }
        );
    }

    #[test]
    fn test_validate_empty() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![],
            genders: vec![],
            plural_count: 0,
        };
        let base = ParsedString::parse("").unwrap();

        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        let val_trans = validate_string(&config, &base, Some(&base));
        assert_eq!(val_trans.len(), 0);
    }

    #[test]
    fn test_validate_invalid() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![],
            genders: vec![],
            plural_count: 0,
        };
        let base = ParsedString::parse("{FOOBAR}").unwrap();

        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 1);
        assert_eq!(
            val_base[0],
            ValidationError {
                severity: Severity::Error,
                pos_begin: Some(0),
                pos_end: Some(8),
                message: String::from("Unknown string command '{FOOBAR}'."),
                suggestion: None,
            }
        );

        let val_trans = validate_string(&config, &base, Some(&base));
        assert_eq!(val_trans.len(), 1);
        assert_eq!(
            val_trans[0],
            ValidationError {
                severity: Severity::Error,
                pos_begin: None,
                pos_end: None,
                message: String::from("Base language text is invalid."),
                suggestion: Some(String::from("This is a bug; wait until it is fixed.")),
            }
        );
    }

    #[test]
    fn test_validate_positional() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![],
            genders: vec![],
            plural_count: 0,
        };
        let base = ParsedString::parse("{NUM}").unwrap();
        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        {
            let trans = ParsedString::parse("{0:NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("{FOOBAR}{NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 1);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(0),
                    pos_end: Some(8),
                    message: String::from("Unknown string command '{FOOBAR}'."),
                    suggestion: None,
                }
            );
        }
        {
            let trans = ParsedString::parse("{1:NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 2);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(0),
                    pos_end: Some(7),
                    message: String::from("There is no parameter in position 1, found '{NUM}'."),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("String command '{0:NUM}' is missing."),
                    suggestion: None,
                }
            );
        }
        {
            let trans = ParsedString::parse("{COMMA}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 2);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(0),
                    pos_end: Some(7),
                    message: String::from("Expected '{0:NUM}', found '{COMMA}'."),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("String command '{0:NUM}' is missing."),
                    suggestion: None,
                }
            );
        }
        {
            let trans = ParsedString::parse("{0:NUM}{0:NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 1);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from(
                        "String command '{0:NUM}': expected 1 times, found 2 times."
                    ),
                    suggestion: None,
                }
            );
        }
    }

    #[test]
    fn test_validate_front() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![],
            genders: vec![String::from("a"), String::from("b")],
            plural_count: 0,
        };
        let base = ParsedString::parse("{BIG_FONT}foo{NUM}").unwrap();
        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        {
            let trans = ParsedString::parse("{G=a}{BIG_FONT}bar{NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("{G=a}{G=a}{BIG_FONT}bar{NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 1);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: Some(5),
                    pos_end: Some(10),
                    message: String::from("Duplicate gender definition."),
                    suggestion: Some(String::from("Remove the second '{G=...}'.")),
                }
            );
        }
        {
            let trans = ParsedString::parse("{BIG_FONT}{G=a}bar{NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 1);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: Some(10),
                    pos_end: Some(15),
                    message: String::from("Gender definitions must be at the front."),
                    suggestion: Some(String::from(
                        "Move '{G=...}' to the front of the translation."
                    )),
                }
            );
        }
        {
            let trans = ParsedString::parse("foo{BIG_FONT}bar{NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("foo{G=a}bar{NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 2);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: Some(3),
                    pos_end: Some(8),
                    message: String::from("Gender definitions must be at the front."),
                    suggestion: Some(String::from(
                        "Move '{G=...}' to the front of the translation."
                    )),
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("String command '{BIG_FONT}' is missing."),
                    suggestion: None,
                }
            );
        }
    }

    #[test]
    fn test_validate_position_references() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![String::from("x"), String::from("y")],
            genders: vec![String::from("a"), String::from("b")],
            plural_count: 2,
        };
        let base = ParsedString::parse("{RED}{NUM}{STRING3}").unwrap();
        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        {
            let trans = ParsedString::parse("{RED}{1:STRING.x}{0:NUM}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("{2:RED}{1:STRING.z}{0:NUM.x}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 3);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(0),
                    pos_end: Some(7),
                    message: String::from("Command '{RED}' cannot have a position reference."),
                    suggestion: Some(String::from("Remove '2:'.")),
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(7),
                    pos_end: Some(19),
                    message: String::from("Unknown case 'z'."),
                    suggestion: Some(String::from("Known cases are: 'x', 'y'")),
                }
            );
            assert_eq!(
                val_trans[2],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(19),
                    pos_end: Some(28),
                    message: String::from("No case selection allowed for '{NUM}'."),
                    suggestion: Some(String::from("Remove '.x'.")),
                }
            );
        }
        {
            let trans = ParsedString::parse("{RED}{NUM}{G i j}{P i j}{STRING.y}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("{RED}{NUM}{G 0 i j}{P 1 i j}{STRING.y}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 2);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(10),
                    pos_end: Some(19),
                    message: String::from(
                        "'{G}' references position '0:0', but '{0:NUM}' does not allow genders."
                    ),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(19),
                    pos_end: Some(28),
                    message: String::from(
                        "'{P}' references position '1:0', but '{1:STRING}' does not allow plurals."
                    ),
                    suggestion: None,
                }
            );
        }
        {
            let trans = ParsedString::parse("{RED}{NUM}{G 1:1 i j}{P 1:3 i j}{STRING.y}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("{RED}{NUM}{G 1:4 i j}{P 1:4 i j}{STRING.y}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 2);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(10),
                    pos_end: Some(21),
                    message: String::from(
                        "'{G}' references position '1:4', but '{1:STRING}' only has 4 subindices."
                    ),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(21),
                    pos_end: Some(32),
                    message: String::from(
                        "'{P}' references position '1:4', but '{1:STRING}' only has 4 subindices."
                    ),
                    suggestion: None,
                }
            );
        }
        {
            let trans = ParsedString::parse("{RED}{NUM}{G 2 i j}{P 2 i j}{STRING.y}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 2);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(10),
                    pos_end: Some(19),
                    message: String::from("'{G}' references position '2', which has no parameter."),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(19),
                    pos_end: Some(28),
                    message: String::from("'{P}' references position '2', which has no parameter."),
                    suggestion: None,
                }
            );
        }
        {
            let trans = ParsedString::parse("{RED}{P i j}{NUM}{STRING.y}{G i j}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 2);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(5),
                    pos_end: Some(12),
                    message: String::from(
                        "'{P}' references position '-1', which has no parameter."
                    ),
                    suggestion: Some(String::from("Add a position reference.")),
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(27),
                    pos_end: Some(34),
                    message: String::from("'{G}' references position '2', which has no parameter."),
                    suggestion: Some(String::from("Add a position reference.")),
                }
            );
        }
    }

    #[test]
    fn test_validate_nochoices() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![],
            genders: vec![],
            plural_count: 1,
        };
        let base = ParsedString::parse("{NUM}{STRING3}").unwrap();
        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        {
            let trans = ParsedString::parse("{G=a}{NUM}{P a}{G a}{STRING}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 3);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(0),
                    pos_end: Some(5),
                    message: String::from("No gender definitions allowed."),
                    suggestion: Some(String::from("Remove '{G=...}'.")),
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(10),
                    pos_end: Some(15),
                    message: String::from("No plural choices allowed."),
                    suggestion: Some(String::from("Remove '{P ...}'.")),
                }
            );
            assert_eq!(
                val_trans[2],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(15),
                    pos_end: Some(20),
                    message: String::from("No gender choices allowed."),
                    suggestion: Some(String::from("Remove '{G ...}'.")),
                }
            );
        }
    }

    #[test]
    fn test_validate_gschoices() {
        let config = LanguageConfig {
            dialect: String::from("game-script"),
            cases: vec![String::from("x"), String::from("y")],
            genders: vec![String::from("a"), String::from("b")],
            plural_count: 2,
        };
        let base = ParsedString::parse("{NUM}{STRING3}").unwrap();
        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        {
            let trans = ParsedString::parse("{G=a}{NUM}{P a b}{G a b}{STRING.x}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 3);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(0),
                    pos_end: Some(5),
                    message: String::from("No gender definitions allowed."),
                    suggestion: Some(String::from("Remove '{G=...}'.")),
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(17),
                    pos_end: Some(24),
                    message: String::from("No gender choices allowed."),
                    suggestion: Some(String::from("Remove '{G ...}'.")),
                }
            );
            assert_eq!(
                val_trans[2],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(24),
                    pos_end: Some(34),
                    message: String::from("No case selections allowed."),
                    suggestion: Some(String::from("Remove '.x'.")),
                }
            );
        }
    }

    #[test]
    fn test_validate_choices() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![String::from("x"), String::from("y")],
            genders: vec![String::from("a"), String::from("b")],
            plural_count: 2,
        };
        let base = ParsedString::parse("{NUM}{STRING3}").unwrap();
        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        {
            let trans = ParsedString::parse("{G=a}{NUM}{P a b}{G a b}{STRING.x}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("{G=c}{NUM}{P a b c}{G a b c}{STRING.z}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 4);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(0),
                    pos_end: Some(5),
                    message: String::from("Unknown gender 'c'."),
                    suggestion: Some(String::from("Known genders are: 'a', 'b'")),
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(10),
                    pos_end: Some(19),
                    message: String::from("Expected 2 plural choices, found 3."),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[2],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(19),
                    pos_end: Some(28),
                    message: String::from("Expected 2 gender choices, found 3."),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[3],
                ValidationError {
                    severity: Severity::Error,
                    pos_begin: Some(28),
                    pos_end: Some(38),
                    message: String::from("Unknown case 'z'."),
                    suggestion: Some(String::from("Known cases are: 'x', 'y'")),
                }
            );
        }
    }

    #[test]
    fn test_validate_nonpositional() {
        let config = LanguageConfig {
            dialect: String::from("openttd"),
            cases: vec![],
            genders: vec![],
            plural_count: 0,
        };
        let base = ParsedString::parse("{RED}{NBSP}{}{GREEN}{NBSP}{}{RED}{TRAIN}").unwrap();
        let val_base = validate_string(&config, &base, None);
        assert_eq!(val_base.len(), 0);

        {
            let trans = ParsedString::parse("{RED}{}{GREEN}{}{RED}{TRAIN}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans = ParsedString::parse("{RED}{}{GREEN}{NBSP}{RED}{NBSP}{GREEN}{}{RED}{TRAIN}")
                .unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 0);
        }
        {
            let trans =
                ParsedString::parse("{RED}{}{RED}{TRAIN}{BLUE}{TRAIN}{RIGHT_ARROW}{SHIP}").unwrap();
            let val_trans = validate_string(&config, &trans, Some(&base));
            assert_eq!(val_trans.len(), 4);
            assert_eq!(
                val_trans[0],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("String command '{GREEN}' is missing."),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[1],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from(
                        "String command '{TRAIN}': expected 1 times, found 2 times."
                    ),
                    suggestion: None,
                }
            );
            assert_eq!(
                val_trans[2],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("String command '{BLUE}' is unexpected."),
                    suggestion: Some(String::from("Remove this command.")),
                }
            );
            assert_eq!(
                val_trans[3],
                ValidationError {
                    severity: Severity::Warning,
                    pos_begin: None,
                    pos_end: None,
                    message: String::from("String command '{SHIP}' is unexpected."),
                    suggestion: Some(String::from("Remove this command.")),
                }
            );
        }
    }

    #[test]
    fn test_normalize_cmd() {
        let mut parsed =
            ParsedString::parse("{RED}{NBSP}{2:RAW_STRING}{0:STRING5}{COMMA}").unwrap();
        normalize_string(&Dialect::OPENTTD, &mut parsed);
        let result = parsed.compile();
        assert_eq!(result, "{RED}{NBSP}{2:STRING}{0:STRING}{1:COMMA}");
    }

    #[test]
    fn test_normalize_ref() {
        let mut parsed = ParsedString::parse("{RED}{NBSP}{P a b}{2:STRING}{P 1 a b}{G 0:1 a b}{0:STRING}{G 0 a b}{P 0:1 a b}{COMMA}{P a b}{G a b}").unwrap();
        normalize_string(&Dialect::OPENTTD, &mut parsed);
        let result = parsed.compile();
        assert_eq!(result, "{RED}{NBSP}{P a b}{2:STRING}{P 1 a b}{G 0:1 a b}{0:STRING}{G 0 a b}{P 0:1 a b}{1:COMMA}{P 1 a b}{G 2 a b}");
    }

    #[test]
    fn test_normalize_subref() {
        let mut parsed = ParsedString::parse(
            "{NUM}{P 0:0 a b}{G 1:0 a b}{G 1:1 a b}{STRING}{P 1:2 a b}{CARGO_LONG}{P 2:1 a b}",
        )
        .unwrap();
        normalize_string(&Dialect::OPENTTD, &mut parsed);
        let result = parsed.compile();
        assert_eq!(
            result,
            "{0:NUM}{P 0 a b}{G 1 a b}{G 1:1 a b}{1:STRING}{P 1:2 a b}{2:CARGO_LONG}{P 2 a b}"
        );
    }
}
