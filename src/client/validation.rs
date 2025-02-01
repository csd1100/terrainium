use regex::Regex;
use std::collections::BTreeMap;
use std::fmt::Formatter;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub(crate) enum ValidationMessageLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for ValidationMessageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationMessageLevel::Debug => {
                write!(f, "debug")
            }
            ValidationMessageLevel::Info => {
                write!(f, "info")
            }
            ValidationMessageLevel::Warn => {
                write!(f, "warn")
            }
            ValidationMessageLevel::Error => {
                write!(f, "error")
            }
        }
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub(crate) struct ValidationMessage {
    pub(crate) level: ValidationMessageLevel,
    pub(crate) message: String,
    pub(crate) target: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ValidationError {
    pub(crate) messages: Vec<ValidationMessage>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.messages
            .iter()
            .try_for_each(|message| writeln!(f, "{} - {}", message.level, message.message))
    }
}

pub(crate) enum IdentifierType {
    Env,
    Alias,
}

impl std::fmt::Display for IdentifierType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentifierType::Env => {
                write!(f, "environment variable")
            }
            IdentifierType::Alias => {
                write!(f, "alias")
            }
        }
    }
}

pub(crate) fn validate_identifiers(
    data_type: IdentifierType,
    data: &BTreeMap<String, String>,
    target: &str,
) -> Vec<ValidationMessage> {
    let mut messages = vec![];

    let starting_with_num = Regex::new(r"^[0-9]").unwrap();
    let invalid_identifier = Regex::new(r"[^a-zA-Z0-9_]").unwrap();

    data.iter().for_each(|(k, _v)| {
        let mut k = k.as_str();

        if k.is_empty() {
            messages.push(ValidationMessage {
                level: ValidationMessageLevel::Error,
                message:
                format!("empty {} identifier is not allowed", data_type),
                target: target.to_string(),
            })
        } else {
            if k.starts_with(" ") || k.ends_with(" ") {
                messages.push(ValidationMessage {
                    level: ValidationMessageLevel::Info,
                    message: format!(
                        "trimming spaces from {} identifier: `{}`",
                        data_type, k
                    ),
                    target: target.to_string(),
                })
            }

            // trim leading and trailing spaces for further validation
            k = k.trim();

            if k.contains(" ") {
                messages.push(ValidationMessage {
                    level: ValidationMessageLevel::Error,
                    message: format!(
                        "{} identifier `{}` is invalid as it contains spaces",
                        data_type, k
                    ),
                    target: target.to_string(),
                })
            }

            if starting_with_num.is_match(k) {
                messages.push(ValidationMessage {
                    level: ValidationMessageLevel::Error,
                    message: format!(
                        "{} identifier `{}` cannot start with number",
                        data_type, k
                    ),
                    target: target.to_string(),
                })
            }

            if invalid_identifier.is_match(k) {
                messages.push(ValidationMessage {
                    level: ValidationMessageLevel::Error,
                    message: format!("{} identifier `{}` contains invalid characters. {} name can only include [a-zA-Z0-9_] characters.", data_type, k, data_type),
                    target: target.to_string(),
                })
            }
        }
    });
    messages
}
