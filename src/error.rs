use std::panic::Location;

pub type Result<T> = std::result::Result<T, Error>;

pub trait Context<T> {
    fn context(self, context: &str) -> Result<T>;
    fn ok_or_log(self, context: &str) -> Option<T>
    where
        Self: Sized,
    {
        match self.context(context) {
            Ok(value) => Some(value),
            Err(error) => {
                log::error!("{error}");
                None
            }
        }
    }
}

impl<T, E: std::error::Error> Context<T> for std::result::Result<T, E> {
    #[track_caller]
    #[inline]
    fn context(self, context: &str) -> Result<T> {
        self.map_err(|e| Error::new(format!("{context}: {e}")))
    }
}

#[derive(Debug)]
pub struct Error {
    message: String,
    location: &'static Location<'static>,
}

impl Error {
    #[track_caller]
    pub fn new(message: String) -> Self {
        Self {
            message,
            location: Location::caller(),
        }
    }
    #[track_caller]
    pub fn no_command() -> Self {
        Self::new("No dmypy command found".to_string())
    }
}

impl From<log::ParseLevelError> for Error {
    #[track_caller]
    fn from(error: log::ParseLevelError) -> Self {
        Self {
            message: format!("log level error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<Error> for tower_lsp::jsonrpc::Error {
    #[track_caller]
    fn from(error: Error) -> Self {
        tower_lsp::jsonrpc::Error {
            code: tower_lsp::jsonrpc::ErrorCode::InternalError,
            message: error.message.into(),
            data: None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [location={}]", self.message, self.location)
    }
}

impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error>> for Error {
    #[track_caller]
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Self {
            message: format!("dyn error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<regex::Error> for Error {
    #[track_caller]
    fn from(error: regex::Error) -> Self {
        Self {
            message: format!("regex error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<serde_yml::Error> for Error {
    #[track_caller]
    fn from(error: serde_yml::Error) -> Self {
        Self {
            message: format!("yaml error: {error:?}"),
            location: Location::caller(),
        }
    }
}
impl From<serde_json::Error> for Error {
    #[track_caller]
    fn from(error: serde_json::Error) -> Self {
        Self {
            message: format!("json error: {error:?}"),
            location: Location::caller(),
        }
    }
}
impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        Self {
            message: format!("io error: {error:?}"),
            location: Location::caller(),
        }
    }
}

/*
impl From<toml::de::Error> for Error {
    #[track_caller]
    fn from(error: toml::de::Error) -> Self {
        Self {
            message: format!("toml error: {error:?}"),
            location: Location::caller(),
        }
    }
}
*/

impl From<String> for Error {
    #[track_caller]
    fn from(error: String) -> Self {
        Self {
            message: format!("error: {error}"),
            location: Location::caller(),
        }
    }
}

impl From<&str> for Error {
    #[track_caller]
    fn from(error: &str) -> Self {
        Self {
            message: format!("error: {error}"),
            location: Location::caller(),
        }
    }
}
