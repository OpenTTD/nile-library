use regex::Regex;

#[derive(Debug, PartialEq)]
struct StringCommand {
    index: Option<usize>,
    name: String,
    case: Option<String>,
}

#[derive(Debug, PartialEq)]
struct GenderDefinition {
    gender: String,
}

#[derive(Debug, PartialEq)]
struct ChoiceList {
    name: String,
    indexref: Option<usize>,
    indexsubref: Option<usize>,
    choices: Vec<String>,
}

#[derive(Debug, PartialEq)]
enum FragmentContent {
    Text(String),
    Command(StringCommand),
    Gender(GenderDefinition),
    Choice(ChoiceList),
}

#[derive(Debug, PartialEq)]
struct StringFragment {
    position: usize,
    fragment: FragmentContent,
}

#[derive(Debug, PartialEq)]
struct ParsedString {
    fragments: Vec<StringFragment>,
}

impl StringCommand {
    fn parse(string: &str) -> Option<StringCommand> {
        let pat_command =
            Regex::new(r"^\{(?:(\d+):)?(|\{|[A-Z]+[A-Z0-9]*)(?:\.(\w+))?\}$").unwrap();
        if let Some(caps) = pat_command.captures(string) {
            let result = StringCommand {
                index: caps.get(1).and_then(|v| v.as_str().parse().ok()),
                name: String::from(&caps[2]),
                case: caps.get(3).map(|v| String::from(v.as_str())),
            };
            return Some(result);
        }
        None
    }
}

impl GenderDefinition {
    fn parse(string: &str) -> Option<GenderDefinition> {
        let pat_gender = Regex::new(r"^\{G *= *(\w+) *\}$").unwrap();
        if let Some(caps) = pat_gender.captures(string) {
            let result = GenderDefinition {
                gender: String::from(&caps[1]),
            };
            return Some(result);
        }
        None
    }
}

impl ChoiceList {
    fn parse(string: &str) -> Option<ChoiceList> {
        let pat_choice =
            Regex::new(r"^\{([PG])(?: +(\d+)(?::(\d+))?)?( +[^ 0-9].*?) *\}$").unwrap();
        let pat_item = Regex::new(r##"^ +(?:([^ "]+)|"([^"]*)")"##).unwrap();
        if let Some(caps) = pat_choice.captures(string) {
            let mut result = ChoiceList {
                name: String::from(&caps[1]),
                indexref: caps.get(2).and_then(|v| v.as_str().parse().ok()),
                indexsubref: caps.get(3).and_then(|v| v.as_str().parse().ok()),
                choices: Vec::new(),
            };
            let mut rest = &caps[4];
            while !rest.is_empty() {
                if let Some(m) = pat_item.captures(rest) {
                    result
                        .choices
                        .push(String::from(m.get(1).or(m.get(2)).unwrap().as_str()));
                    rest = &rest[m.get(0).unwrap().end()..];
                } else {
                    return None;
                }
            }
            return Some(result);
        }
        None
    }
}

impl FragmentContent {
    fn parse(string: &str) -> Result<FragmentContent, String> {
        if let Some(command) = StringCommand::parse(string) {
            Ok(FragmentContent::Command(command))
        } else if let Some(gender) = GenderDefinition::parse(string) {
            Ok(FragmentContent::Gender(gender))
        } else if let Some(choice) = ChoiceList::parse(string) {
            Ok(FragmentContent::Choice(choice))
        } else {
            Err(format!("Invalid string command: '{}'", string))
        }
    }
}

impl ParsedString {
    fn parse(string: &str) -> Result<ParsedString, String> {
        let mut result = ParsedString {
            fragments: Vec::new(),
        };
        let mut rest: &str = string.trim_end(); // TODO what to do with \t, \r, \n, ... inside the string?
        let mut position: usize = 0;
        while !rest.is_empty() {
            if let Some(start) = rest.find('{') {
                if start > 0 {
                    let text: &str;
                    (text, rest) = rest.split_at(start);
                    result.fragments.push(StringFragment {
                        position: position,
                        fragment: FragmentContent::Text(String::from(text)),
                    });
                }
                position += start;
                if let Some(end) = rest.find('}') {
                    let text: &str;
                    (text, rest) = rest.split_at(end + 1);
                    match FragmentContent::parse(text) {
                        Ok(fragment) => result.fragments.push(StringFragment {
                            position: position,
                            fragment: fragment,
                        }),
                        Err(message) => return Err(message),
                    };
                    position += end + 1
                } else {
                    return Err(String::from("Unterminated string command, '}' expected."));
                }
            } else {
                result.fragments.push(StringFragment {
                    position: position,
                    fragment: FragmentContent::Text(String::from(rest)),
                });
                break;
            }
        }
        Ok(result)
    }
}

//fn compile_string(parsed: &ParsedString) -> String {
//}

//fn check_string(parsed: &mut ParsedString) {
// project-type, language-info, base-language
// gender-assignment in front
// all commands known and allowed in project
// plural/gender references valid
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
    fn test_parse_cmd_ok() {
        assert_eq!(
            FragmentContent::parse("{}"),
            Ok(FragmentContent::Command(StringCommand {
                index: None,
                name: String::from(""),
                case: None
            }))
        );
        assert_eq!(
            FragmentContent::parse("{{}"),
            Ok(FragmentContent::Command(StringCommand {
                index: None,
                name: String::from("{"),
                case: None
            }))
        );
        assert_eq!(
            FragmentContent::parse("{NUM}"),
            Ok(FragmentContent::Command(StringCommand {
                index: None,
                name: String::from("NUM"),
                case: None
            }))
        );
        assert_eq!(
            FragmentContent::parse("{1:RED}"),
            Ok(FragmentContent::Command(StringCommand {
                index: Some(1),
                name: String::from("RED"),
                case: None
            }))
        );
        assert_eq!(
            FragmentContent::parse("{STRING.gen}"),
            Ok(FragmentContent::Command(StringCommand {
                index: None,
                name: String::from("STRING"),
                case: Some(String::from("gen"))
            }))
        );
        assert_eq!(
            FragmentContent::parse("{1:STRING.gen}"),
            Ok(FragmentContent::Command(StringCommand {
                index: Some(1),
                name: String::from("STRING"),
                case: Some(String::from("gen"))
            }))
        );
        assert_eq!(
            FragmentContent::parse("{G=n}"),
            Ok(FragmentContent::Gender(GenderDefinition {
                gender: String::from("n")
            }))
        );
        assert_eq!(
            FragmentContent::parse("{G = n}"),
            Ok(FragmentContent::Gender(GenderDefinition {
                gender: String::from("n")
            }))
        );
        assert_eq!(
            FragmentContent::parse("{P a b}"),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from("a"), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P "" b}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from(""), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P "a b" "c"}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from("a b"), String::from("c")]
            }))
        );
        assert_eq!(
            FragmentContent::parse("{P 1 a b}"),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: None,
                choices: vec![String::from("a"), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1 "" b}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: None,
                choices: vec![String::from(""), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1 "a b" "c"}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: None,
                choices: vec![String::from("a b"), String::from("c")]
            }))
        );
        assert_eq!(
            FragmentContent::parse("{P 1:2 a b}"),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: Some(2),
                choices: vec![String::from("a"), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1:2 "" b}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: Some(2),
                choices: vec![String::from(""), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1:2 "a b" "c"}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: Some(2),
                choices: vec![String::from("a b"), String::from("c")]
            }))
        );

        assert_eq!(
            FragmentContent::parse("{P a b c}"),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from("a"), String::from("b"), String::from("c")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P "" "" b}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from(""), String::from(""), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P a ""}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from("a"), String::from("")]
            }))
        );
        assert_eq!(
            FragmentContent::parse("{P 1 a b c}"),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: None,
                choices: vec![String::from("a"), String::from("b"), String::from("c")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1 "" "" b}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: None,
                choices: vec![String::from(""), String::from(""), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1 a ""}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: None,
                choices: vec![String::from("a"), String::from("")]
            }))
        );
        assert_eq!(
            FragmentContent::parse("{P 1:2 a b c}"),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: Some(2),
                choices: vec![String::from("a"), String::from("b"), String::from("c")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1:2 "" "" b}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: Some(2),
                choices: vec![String::from(""), String::from(""), String::from("b")]
            }))
        );
        assert_eq!(
            FragmentContent::parse(r##"{P 1:2 a ""}"##),
            Ok(FragmentContent::Choice(ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: Some(2),
                choices: vec![String::from("a"), String::from("")]
            }))
        );
    }

    #[test]
    fn test_parse_cmd_err() {
        assert!(FragmentContent::parse("{1}").is_err());
        assert!(FragmentContent::parse("{1:1}").is_err());
        assert!(FragmentContent::parse("{1:1 NUM}").is_err());
        assert!(FragmentContent::parse("{NUM=a}").is_err());
        assert!(FragmentContent::parse(r##"{P " a}"##).is_err());
        assert!(FragmentContent::parse(r##"{P 1.a a b}"##).is_err());
        assert!(FragmentContent::parse(r##"{P 1:a a b}"##).is_err());
    }

    #[test]
    fn test_parse_str_empty() {
        let case1 = ParsedString::parse("");
        assert!(case1.is_ok());
        let case1 = case1.unwrap();
        assert!(case1.fragments.is_empty());
    }

    #[test]
    fn test_parse_str_ok() {
        let case1 = ParsedString::parse("{G=n}{ORANGE}OpenTTD {STRING}");
        assert!(case1.is_ok());
        let case1 = case1.unwrap();
        assert_eq!(
            case1.fragments,
            vec![
                StringFragment {
                    position: 0,
                    fragment: FragmentContent::Gender(GenderDefinition {
                        gender: String::from("n")
                    })
                },
                StringFragment {
                    position: 5,
                    fragment: FragmentContent::Command(StringCommand {
                        index: None,
                        name: String::from("ORANGE"),
                        case: None
                    })
                },
                StringFragment {
                    position: 13,
                    fragment: FragmentContent::Text(String::from("OpenTTD "))
                },
                StringFragment {
                    position: 21,
                    fragment: FragmentContent::Command(StringCommand {
                        index: None,
                        name: String::from("STRING"),
                        case: None
                    })
                },
            ]
        );
    }

    #[test]
    fn test_parse_str_err() {
        let case1 = ParsedString::parse("{G=n}{ORANGE OpenTTD");
        assert!(case1.is_err());
    }
}
