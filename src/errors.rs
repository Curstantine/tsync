use std::borrow::Cow;
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
    pub message: Cow<'static, str>,
    pub context: Option<Cow<'static, str>>,
    pub source: Option<Box<dyn std::error::Error + Send>>,
}

impl Error {
    #[inline]
    pub fn descriptive(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            type_: ErrorType::Descriptive,
            message: message.into(),
            context: None,
            source: None,
        }
    }

    #[inline]
    pub fn with_context(mut self, context: impl Into<Cow<'static, str>>) -> Self {
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
            message: Cow::Owned(error.to_string()),
            context: None,
            source: Some(Box::new(error)),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(error: std::num::ParseIntError) -> Self {
        Self {
            type_: ErrorType::StdParseInt,
            message: Cow::Owned(error.to_string()),
            context: None,
            source: Some(Box::new(error)),
        }
    }
}
