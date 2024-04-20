use regex::Regex;

#[derive(Debug, PartialEq)]
pub struct StringCommand {
    pub index: Option<usize>,
    pub name: String,
    pub case: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct GenderDefinition {
    pub gender: String,
}

#[derive(Debug, PartialEq)]
pub struct ChoiceList {
    pub name: String,
    pub indexref: Option<usize>,
    pub indexsubref: Option<usize>,
    pub choices: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum FragmentContent {
    Text(String),
    Command(StringCommand),
    Gender(GenderDefinition),
    Choice(ChoiceList),
}

#[derive(Debug, PartialEq)]
pub struct StringFragment {
    pub position: usize,
    pub content: FragmentContent,
}

#[derive(Debug, PartialEq)]
pub struct ParsedString {
    pub fragments: Vec<StringFragment>,
}

impl StringCommand {
    fn parse(string: &str) -> Option<StringCommand> {
        let pat_command =
            Regex::new(r"^\{(?:(\d+):)?(|\{|[A-Z]+[A-Z0-9]*)(?:\.(\w+))?\}$").unwrap();
        let caps = pat_command.captures(string)?;
        Some(StringCommand {
            index: caps.get(1).and_then(|v| v.as_str().parse().ok()),
            name: String::from(&caps[2]),
            case: caps.get(3).map(|v| String::from(v.as_str())),
        })
    }

    fn compile(&self) -> String {
        let mut result = String::from("{");
        if let Some(i) = self.index {
            result.push_str(&format!("{}:", i));
        }
        result.push_str(&self.name);
        if let Some(case) = &self.case {
            result.push_str(&format!(".{}", case));
        }
        result.push_str("}");
        result
    }
}

impl GenderDefinition {
    fn parse(string: &str) -> Option<GenderDefinition> {
        let pat_gender = Regex::new(r"^\{G\s*=\s*(\w+)\}$").unwrap();
        let caps = pat_gender.captures(string)?;
        Some(GenderDefinition {
            gender: String::from(&caps[1]),
        })
    }

    fn compile(&self) -> String {
        format!("{{G={}}}", self.gender)
    }
}

impl ChoiceList {
    fn parse(string: &str) -> Option<ChoiceList> {
        let pat_choice =
            Regex::new(r"^\{([PG])(?:\s+(\d+)(?::(\d+))?)?(\s+[^\s0-9].*?)\s*\}$").unwrap();
        let pat_item = Regex::new(r##"^\s+(?:([^\s"]+)|"([^"]*)")"##).unwrap();
        let caps = pat_choice.captures(string)?;
        let mut result = ChoiceList {
            name: String::from(&caps[1]),
            indexref: caps.get(2).and_then(|v| v.as_str().parse().ok()),
            indexsubref: caps.get(3).and_then(|v| v.as_str().parse().ok()),
            choices: Vec::new(),
        };
        let mut rest = &caps[4];
        while !rest.is_empty() {
            let m = pat_item.captures(rest)?;
            result
                .choices
                .push(String::from(m.get(1).or(m.get(2)).unwrap().as_str()));
            rest = &rest[m.get(0).unwrap().end()..];
        }
        return Some(result);
    }

    fn compile(&self) -> String {
        let mut result = format!("{{{}", self.name);
        if let Some(i) = self.indexref {
            result.push_str(&format!(" {}", i));
            if let Some(s) = self.indexsubref {
                result.push_str(&format!(":{}", s))
            }
        }
        for c in &self.choices {
            if c.is_empty() || c.contains(|v| char::is_ascii_whitespace(&v)) {
                result.push_str(&format!(r##" "{}""##, c));
            } else {
                result.push_str(&format!(" {}", c));
            }
        }
        result.push_str("}");
        result
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

    fn compile(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Command(command) => command.compile(),
            Self::Gender(gender) => gender.compile(),
            Self::Choice(choice) => choice.compile(),
        }
    }
}

impl ParsedString {
    fn parse(string: &str) -> Result<ParsedString, String> {
        let mut result = ParsedString {
            fragments: Vec::new(),
        };
        let mut rest: &str = string;
        let mut position: usize = 0;
        while !rest.is_empty() {
            if let Some(start) = rest.find('{') {
                if start > 0 {
                    let text: &str;
                    (text, rest) = rest.split_at(start);
                    result.fragments.push(StringFragment {
                        position: position,
                        content: FragmentContent::Text(String::from(text)),
                    });
                }
                position += start;
                if let Some(end) = rest.find('}') {
                    let text: &str;
                    (text, rest) = rest.split_at(end + 1);
                    match FragmentContent::parse(text) {
                        Ok(content) => result.fragments.push(StringFragment {
                            position: position,
                            content: content,
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
                    content: FragmentContent::Text(String::from(rest)),
                });
                break;
            }
        }
        Ok(result)
    }

    fn compile(&self) -> String {
        let mut result = String::new();
        for f in &self.fragments {
            result.push_str(&f.content.compile());
        }
        result
    }
}

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
            FragmentContent::parse("{P\na\tb}"),
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
            FragmentContent::parse("{P\t1\na\rb\n}"),
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
    fn test_compile_cmd() {
        assert_eq!(
            StringCommand {
                index: None,
                name: String::from(""),
                case: None
            }
            .compile(),
            "{}"
        );
        assert_eq!(
            StringCommand {
                index: None,
                name: String::from("{"),
                case: None
            }
            .compile(),
            "{{}"
        );
        assert_eq!(
            StringCommand {
                index: Some(1),
                name: String::from("STRING"),
                case: Some(String::from("gen"))
            }
            .compile(),
            "{1:STRING.gen}"
        );
        assert_eq!(
            GenderDefinition {
                gender: String::from("n")
            }
            .compile(),
            "{G=n}"
        );
        assert_eq!(
            ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from("a"), String::from("b")]
            }
            .compile(),
            "{P a b}"
        );
        assert_eq!(
            ChoiceList {
                name: String::from("P"),
                indexref: None,
                indexsubref: None,
                choices: vec![String::from(""), String::from(" b")]
            }
            .compile(),
            r##"{P "" " b"}"##
        );
        assert_eq!(
            ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: None,
                choices: vec![String::from("a"), String::from("b")]
            }
            .compile(),
            "{P 1 a b}"
        );
        assert_eq!(
            ChoiceList {
                name: String::from("P"),
                indexref: Some(1),
                indexsubref: Some(2),
                choices: vec![String::from("a"), String::from("b")]
            }
            .compile(),
            "{P 1:2 a b}"
        );
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
                    content: FragmentContent::Gender(GenderDefinition {
                        gender: String::from("n")
                    })
                },
                StringFragment {
                    position: 5,
                    content: FragmentContent::Command(StringCommand {
                        index: None,
                        name: String::from("ORANGE"),
                        case: None
                    })
                },
                StringFragment {
                    position: 13,
                    content: FragmentContent::Text(String::from("OpenTTD "))
                },
                StringFragment {
                    position: 21,
                    content: FragmentContent::Command(StringCommand {
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
