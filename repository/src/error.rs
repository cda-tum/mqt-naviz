use std::str::Utf8Error;

use naviz_parser::{ParseError, ParseErrorInner};

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    UTF8Error(Utf8Error),
    IdError,
    LexError(usize, ParseErrorInner),
    ParseError(usize, ParseErrorInner),
    ConfigReadError(naviz_parser::config::error::Error),
    NotRemovableError,
}

impl Error {
    /// Creates a [Error::LexError] from a [ParseError]
    pub fn lex_error<I>(error: ParseError<I>) -> Self {
        Self::LexError(error.offset(), error.into_inner())
    }

    /// Creates a [Error::ParseError] from a [ParseError]
    pub fn parse_error<I>(error: ParseError<I>) -> Self {
        Self::ParseError(error.offset(), error.into_inner())
    }
}

/// A Result with the defined [Error]-type
pub type Result<T> = std::result::Result<T, Error>;
