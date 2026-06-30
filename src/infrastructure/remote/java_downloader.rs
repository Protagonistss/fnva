use std::fmt;

/// Download target (file path or in-memory bytes).
pub enum DownloadTarget {
    Bytes(Vec<u8>),
    File(String),
}

/// Download error type.
#[derive(Debug)]
pub enum DownloadError {
    Network(String),
    NotFound,
    Invalid(String),
    Io(String),
    VersionParse,
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::Network(msg) => write!(f, "Network error: {msg}"),
            DownloadError::NotFound => write!(f, "Resource not found"),
            DownloadError::Invalid(msg) => write!(f, "Invalid data: {msg}"),
            DownloadError::Io(msg) => write!(f, "IO error: {msg}"),
            DownloadError::VersionParse => write!(f, "Version parse error"),
        }
    }
}

impl std::error::Error for DownloadError {}

impl From<String> for DownloadError {
    fn from(err: String) -> Self {
        DownloadError::Network(err)
    }
}
