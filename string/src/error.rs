use std::error::Error;
use std::fmt;

/// InvalidUTF8Error indicates that the string contains invalid UTF-8 and could not be
/// represented as a Rust string.
#[derive(Eq, PartialEq, Debug)]
pub struct InvalidUTF8Error;

impl fmt::Display for InvalidUTF8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value contains invalid UTF-8 bytes")
    }
}

impl Error for InvalidUTF8Error {}

/// EmbeddedNulError indicates that the string contains embedded NUL bytes and
/// could not be represented as a C string.
#[derive(Eq, PartialEq, Debug)]
pub struct EmbeddedNulError;

impl fmt::Display for EmbeddedNulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value contains embedded NUL bytes")
    }
}

impl Error for EmbeddedNulError {}
