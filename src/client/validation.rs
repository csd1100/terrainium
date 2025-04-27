use crate::client::types::command::{Command, CommandsType, OperationType};
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Formatter;
use tracing::{event, Level};

#[allow(dead_code)]
#[derive(Debug, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum ValidationMessageLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl std::fmt::Display for ValidationMessageLevel {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
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

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub(crate) enum Target<'a> {
    Env(&'a str),
    Alias(&'a str),
    ForegroundConstructor(&'a Command),
    BackgroundConstructor(&'a Command),
    ForegroundDestructor(&'a Command),
    BackgroundDestructor(&'a Command),
}

impl<'a> Target<'a> {
    pub(crate) fn from_identifier(identifier: &IdentifierType, value: &'a str) -> Self {
        match identifier {
            IdentifierType::Env => Target::Env(value),
            IdentifierType::Alias => Target::Alias(value),
            IdentifierType::Identifier => panic!("did not expect identifier without type"),
        }
    }

    pub(crate) fn from_command(
        commands_type: &CommandsType,
        operation_type: &OperationType,
        command: &'a Command,
    ) -> Self {
        match operation_type {
            OperationType::Constructor => match commands_type {
                CommandsType::Foreground => Target::ForegroundConstructor(command),
                CommandsType::Background => Target::BackgroundConstructor(command),
            },
            OperationType::Destructor => match commands_type {
                CommandsType::Foreground => Target::ForegroundDestructor(command),
                CommandsType::Background => Target::BackgroundDestructor(command),
            },
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) enum ValidationFixAction<'a> {
    None,
    Trim {
        biome_name: &'a str,
        target: Target<'a>,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ValidationResult<'a> {
    pub(crate) level: ValidationMessageLevel,
    pub(crate) message: String,
    pub(crate) r#for: String,
    pub(crate) fix_action: ValidationFixAction<'a>,
}

impl ValidationResult<'_> {
    pub fn level(&self) -> &ValidationMessageLevel {
        &self.level
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn target(&self) -> &str {
        &self.r#for
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValidationResults<'a> {
    fixable: bool,
    results: HashSet<ValidationResult<'a>>,
}

impl<'a> ValidationResults<'a> {
    pub(crate) fn new(fixable: bool, results: HashSet<ValidationResult<'a>>) -> Self {
        Self { fixable, results }
    }

    pub fn is_fixable(&self) -> bool {
        self.fixable
    }

    pub fn results_ref(&self) -> &HashSet<ValidationResult> {
        &self.results
    }

    pub(crate) fn append(&mut self, other: ValidationResults<'a>) {
        if other.fixable {
            self.fixable = true;
        }
        self.results.extend(other.results);
    }

    pub fn print_validation_message(&self) {
        let messages = self.results.clone();

        messages.iter().for_each(|message| {
            let target = format!("terrain_validation({})", message.r#for);
            match message.level {
                ValidationMessageLevel::Debug => {
                    event!(Level::DEBUG, r#for = target, "{:?}", message.message);
                }
                ValidationMessageLevel::Info => {
                    event!(Level::INFO, r#for = target, "{:?}", message.message);
                }
                ValidationMessageLevel::Warn => {
                    event!(Level::WARN, r#for = target, "{:?}", message.message);
                }
                ValidationMessageLevel::Error => {
                    event!(Level::ERROR, r#for = target, "{:?}", message.message);
                }
            }
        })
    }

    pub fn results(self) -> HashSet<ValidationResult<'a>> {
        self.results
    }
}

#[derive(Debug, Clone)]
pub struct ValidationError<'a> {
    pub(crate) results: ValidationResults<'a>,
}

impl ValidationError<'_> {
    pub fn results(&self) -> &ValidationResults {
        &self.results
    }
}

impl std::fmt::Display for ValidationError<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.results
            .results_ref()
            .iter()
            .try_for_each(|message| writeln!(f, "{} - {}", message.level, message.message))
    }
}

pub(crate) enum IdentifierType {
    Env,
    Alias,
    Identifier,
}

impl std::fmt::Display for IdentifierType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentifierType::Env => {
                write!(f, "env")
            }
            IdentifierType::Alias => {
                write!(f, "alias")
            }
            IdentifierType::Identifier => {
                write!(f, "env or alias")
            }
        }
    }
}

pub(crate) fn validate_identifiers<'a>(
    data_type: IdentifierType,
    data: &'a BTreeMap<String, String>,
    biome_name: &'a str,
) -> ValidationResults<'a> {
    let mut fixable = false;
    let mut messages = HashSet::new();

    let starting_with_num = Regex::new(r"^[0-9]").unwrap();
    let invalid_identifier = Regex::new(r"[^a-zA-Z0-9_]").unwrap();

    data.iter().for_each(|(k, _v)| {
        let mut k = k.as_str();

        if k.is_empty() {
            messages.insert(ValidationResult {
                level: ValidationMessageLevel::Error,
                message:
                "empty identifier is not allowed".to_string(),
                r#for: format!("{biome_name}({data_type})"),
                fix_action: ValidationFixAction::None,
            });
        } else {
            if k.starts_with(" ") || k.ends_with(" ") {
                fixable = true;
                messages.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!(
                        "trimming spaces from identifier: '{k}'"
                    ),
                    r#for: format!("{biome_name}({data_type})"),
                    fix_action: ValidationFixAction::Trim { biome_name, target: Target::from_identifier(&data_type, k) },
                });
            }

            // trim leading and trailing spaces for further validation
            k = k.trim();

            if k.contains(" ") {
                messages.insert(ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: format!(
                        "identifier '{k}' is invalid as it contains spaces",
                    ),
                    r#for: format!("{biome_name}({data_type})"),
                    fix_action: ValidationFixAction::None,
                });
            }

            if starting_with_num.is_match(k) {
                messages.insert(ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: format!(
                        "identifier '{k}' cannot start with number",
                    ),
                    r#for: format!("{biome_name}({data_type})"),
                    fix_action: ValidationFixAction::None,
                });
            }

            if invalid_identifier.is_match(k) {
                messages.insert(ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: format!("identifier '{k}' contains invalid characters. identifier name can only include [a-zA-Z0-9_] characters."),
                    r#for: format!("{biome_name}({data_type})"),
                    fix_action: ValidationFixAction::None,
                });
            }
        }
    });
    ValidationResults::new(fixable, messages)
}
