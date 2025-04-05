use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Error types for the cjseq library
#[derive(Error, Debug)]
pub enum CjseqError {
    #[error("CityJSON error: {0}")]
    CityJsonError(String),

    /// Error when parsing JSON
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),

    /// Error when performing I/O operations
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// Error when a file is not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Error when a required field is missing
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Error when a value is invalid
    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    /// Error related to HTTP requests
    #[cfg(feature = "http")]
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Generic error with custom message
    #[error("{0}")]
    Generic(String),
}

// // Helper conversion methods for easier error handling
// impl From<&str> for CjseqError {
//     fn from(s: &str) -> Self {
//         CjseqError::Generic(s.to_string())
//     }
// }

// impl From<String> for CjseqError {
//     fn from(s: String) -> Self {
//         CjseqError::Generic(s)
//     }
// }

/// A specialized Result type for cjseq operations
pub type Result<T> = std::result::Result<T, CjseqError>;
