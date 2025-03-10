use std::error::Error;
use std::fmt;

/// A custom error type for configuration parsing errors.
#[derive(Debug)]
pub struct ConfigParseError {
    details: String,
}

impl ConfigParseError {
    /// Creates a new `ConfigParseError` with the given error message.
    pub fn new(msg: &str) -> ConfigParseError {
        ConfigParseError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ConfigParseError: {}", self.details)
    }
}

impl Error for ConfigParseError {
    fn description(&self) -> &str {
        &self.details
    }
}
