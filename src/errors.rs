use std::{
    fmt::{Display, Formatter},
    num::ParseIntError,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io {
        source: std::io::Error,
        /// Include additional context information about the error, like the path to the file that couldn't be opened.
        context: Option<String>,
    },
    Descriptive(String),
    Abort,
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            source: error,
            context: None,
        }
    }
}

impl From<inquire::InquireError> for Error {
    fn from(value: inquire::InquireError) -> Self {
        use inquire::InquireError;

        match value {
            InquireError::OperationInterrupted | InquireError::OperationCanceled => Error::Abort,
            InquireError::IO(io_error) => Error::Io {
                source: io_error,
                context: None,
            },
            _ => panic!("Unhandled error: {:#?}", value),
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(error: ParseIntError) -> Self {
        Self::Descriptive(error.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Abort => write!(f, "Aborted."),
            Self::Descriptive(message) => write!(f, "{}", message),
            Self::Io { source, context } => {
                if let Some(context) = context {
                    write!(f, "IO error: {}\nContext: {}", source, context)
                } else {
                    write!(f, "IO error: {}", source)
                }
            }
        }
    }
}
