use std::error::Error as StdError;

#[derive(Debug, Clone, Copy)]
pub enum ErrorType {
    InvalidIpFormat
}

#[derive(Debug)]
pub struct Error {
    pub error_type: ErrorType,
    pub message: &'static str,
    pub cause: Option<&'static StdError>
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.message
    }

    fn cause(&self) -> Option<&dyn StdError> {
        self.cause
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.cause {
            Some(cause) => write!(f, "[{:?}]: {}\nCaused by: {}",
                                  self.error_type,
                                  self.message,
                                  cause),
            None => write!(f, "[{:?}]: {}", self.error_type, self.message)
        }
    }
}