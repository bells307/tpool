use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct JoinError(u32);

impl JoinError {
    pub fn new(count: u32) -> Self {
        Self(count)
    }
}

impl Display for JoinError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "detected panicked threads while join: {0}", self.0)
    }
}

impl Error for JoinError {}
