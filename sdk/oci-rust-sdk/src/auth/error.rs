use std::fmt;

#[derive(Debug)]
pub enum AuthError {
    ConfigNotFound(String),
    InvalidConfig(String),
    IoError(std::io::Error),
    CryptoError(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::ConfigNotFound(msg) => write!(f, "Config not found: {}", msg),
            AuthError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            AuthError::IoError(err) => write!(f, "IO error: {}", err),
            AuthError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<std::io::Error> for AuthError {
    fn from(err: std::io::Error) -> Self {
        AuthError::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, AuthError>;
