use crate::Pos;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Parse color error: {:?}", _0.0)]
    ParseColorError(ParseColorError),
    #[error("GetPx command parameters outside frame: {0:?}")]
    GetPxOutside(Pos),
    #[error("Unknown command: {0:?}")]
    UnknownCommand(String),
    #[error("Io Error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug)]
pub struct ParseColorError(String);

impl ParseColorError {
    pub fn new<S: ToString>(s: S) -> Self {
        Self(s.to_string())
    }
}

impl From<io::ErrorKind> for MyError {
    fn from(kind: io::ErrorKind) -> Self {
        Self::Io(kind.into())
    }
}

pub type MyResult<T = ()> = Result<T, MyError>;
