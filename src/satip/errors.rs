use std::error::Error as StdError;

#[derive(Debug, Clone, Copy)]
pub enum ErrorType {
    InvalidIpFormat
}

#[derive(Debug)]
pub struct Error {
    pub error_type: ErrorType,
    pub message: &'static str,
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.message
    }

    fn cause(&self) -> Option<&dyn StdError> {
        None
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{:?}]: {}", self.error_type, self.message)
    }
}