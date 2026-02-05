use thiserror::Error;

use crate::core::{commit::error::CommitError, storage::StorageError};

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    StorageError(#[from] StorageError),

    #[error("{0}")]
    CommitError(#[from] CommitError),

    #[error("{0}")]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    DeserialzeError(#[from] toml::de::Error),

    #[error("{0}")]
    DecodeError(#[from] bincode::error::DecodeError),

    #[error("{0}")]
    EncodeError(#[from] bincode::error::EncodeError),

    #[error("There are staged files that have not been committed yet")]
    GcError,

    #[error("Cannot delete the active branch")]
    ActiveBranchDeletionError,
}

pub type GatoResult<T> = std::result::Result<T, Error>;
