
use std::num::ParseIntError;
use std::env::VarError;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    Ocotocrab(octocrab::Error),
    ParseInt(ParseIntError),
    EnvVar(VarError),
    StdIO(std::io::Error),
    SerdeJson(serde_json::Error),
}

impl From<VarError> for self::Error {
    fn from(err: VarError) -> Self {
        Self::EnvVar(err)
    }
}

impl From<ParseIntError> for self::Error {
    fn from(err: ParseIntError) -> Self {
        Self::ParseInt(err)
    }
}

impl From<std::io::Error> for self::Error {
    fn from(err: std::io::Error) -> Self {
        Self::StdIO(err)
    }
}

impl From<serde_json::Error> for self::Error {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeJson(err)
    }
}
impl From<octocrab::Error> for self::Error {
    fn from(err: octocrab::Error) -> Self {
        Error::Ocotocrab(err)
    }
}

pub type Result<T, E = self::Error> = std::result::Result<T, E>;

/// This just logs the error and consumes the result
/// Intended for actions which shouldn't crash the application
pub fn log_if_error<T>(result: Result<T>) {
    use log::error;

    if let Err(e) = result {
        error!("{:?}", e);
    }
}
