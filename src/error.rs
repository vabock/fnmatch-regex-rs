//! Common error definitions for the fnmatch crate.

use std::error;
use std::fmt;

/// An error that occurred during the processing of a pattern.
#[derive(Debug)]
pub struct Error {
    /// The error message.
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for Error {}

impl Error {
    /// Return an error with the specified message.
    pub fn new(msg: String) -> Self {
        Self { msg }
    }

    /// Return a boxed error with the specified message.
    pub fn boxed(msg: String) -> Box<Self> {
        Box::new(Self { msg })
    }
}

/// Report an error that occurred during the parsing of a pattern.
pub fn parse_error(msg: String) -> Box<Error> {
    Error::boxed(msg)
}
