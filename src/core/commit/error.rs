use std::io;

use thiserror::Error;

use crate::core::storage::StorageError;

#[derive(Debug, Error)]
pub enum CommitError {
    #[error("Storage interaction failed: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization failed: {0}")]
    EncodingError(#[from] bincode::error::EncodeError),

    #[error("Deserialization failed: {0}")]
    DecodingError(#[from] bincode::error::DecodeError),

    #[error("Storage Error {0}")]
    StorageError(#[from] StorageError),
    // #[error("Corrupt or missing index file")]
    // IndexLoadError,

    // #[error("Could not resolve HEAD ref (are you in a detached state?)")]
    // HeadResolutionError,

    // #[error("Object not found with hash: {0}")]
    // ObjectNotFound(String),

    // #[error("Invalid object format for hash: {0}")]
    // InvalidObjectFormat(String),
}
