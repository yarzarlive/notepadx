use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotepadxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File is not valid UTF-8: {0}")]
    InvalidUtf8(PathBuf),

    #[error("File appears to be binary and not a plain text file: {0}")]
    BinaryFile(PathBuf),

    #[error("Recovery serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),
}

pub type Result<T> = std::result::Result<T, NotepadxError>;
