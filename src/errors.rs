use std::fmt::{Debug, Display};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ErrorType {
    Abort,

    StdIo,
    StdParseInt,
    Descriptive,
    Serde,
}

#[derive(Debug)]
pub struct Error {
    pub type_: ErrorType,
    pub message: String,
    pub context: Option<String>,
    pub source: Option<Box<dyn std::error::Error + Send>>,
}

impl Error {
    pub fn descriptive(message: impl Into<String>) -> Self {
        Self {
            type_: ErrorType::Descriptive,
            message: message.into(),
            context: None,
            source: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(context) = &self.context {
            write!(f, "{}: {}", self.message, context)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|boxed| boxed.as_ref() as _)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self {
            type_: ErrorType::StdIo,
            message: error.to_string(),
            context: None,
            source: Some(Box::new(error)),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(error: std::num::ParseIntError) -> Self {
        Self {
            type_: ErrorType::StdParseInt,
            message: error.to_string(),
            context: None,
            source: Some(Box::new(error)),
        }
    }
}
