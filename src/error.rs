use std;
use serde_json as json;
use serde;
use url;
use git2;
use notify;
use tempfile;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Json(json::error::Error),
    Io(std::io::Error),
    Url(url::ParseError),
    Git(git2::Error),
    Deserialize(serde::de::value::Error),
    Notify(notify::Error),
    Tempfile(tempfile::PersistError),
    Nom,
    MissingParameter(String),
    InvalidTargetDir,
    RepositoryBare,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Json(ref e) => e.fmt(f),
            Error::Io(ref e) => e.fmt(f),
            Error::Deserialize(ref e) => e.fmt(f),
            Error::Url(ref e) => e.fmt(f),
            Error::Git(ref e) => e.fmt(f),
            Error::Notify(ref e) => e.fmt(f),
            Error::Tempfile(ref e) => e.fmt(f),
            Error::Nom => write!(f, "Parse error"),
            Error::MissingParameter(ref p) => write!(f, "Missing parameter: {}", p),
            Error::InvalidTargetDir => write!(f, "Target directory is invalid"),
            Error::RepositoryBare => write!(f, "Git repository is bare"),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Json(ref e) => e.description(),
            Error::Io(ref e) => e.description(),
            Error::Deserialize(ref e) => e.description(),
            Error::Url(ref e) => e.description(),
            Error::Git(ref e) => e.description(),
            Error::Notify(ref e) => e.description(),
            Error::Tempfile(ref e) => e.description(),
            Error::Nom => "Parse Error",
            Error::MissingParameter(_) => "Missing parameter",
            Error::InvalidTargetDir => "Target directory is invalid",
            Error::RepositoryBare => "Git repository is bare",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::Json(ref e) => Some(e),
            Error::Io(ref e) => Some(e),
            Error::Deserialize(ref e) => Some(e),
            Error::Url(ref e) => Some(e),
            Error::Git(ref e) => Some(e),
            Error::Notify(ref e) => Some(e),
            Error::Tempfile(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<json::error::Error> for Error {
    fn from(e: json::error::Error) -> Self {
        Error::Json(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::Url(e)
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Error::Git(e)
    }
}

impl From<notify::Error> for Error {
    fn from(e: notify::Error) -> Self {
        Error::Notify(e)
    }
}

impl From<tempfile::PersistError> for Error {
    fn from(e: tempfile::PersistError) -> Self {
        Error::Tempfile(e)
    }
}
