use crate::core::vfs::error;

#[derive(thiserror::Error, Debug)]
pub enum VFSError {
    #[error("Internal synchronization error: Lock is poisoned")]
    LockPoisoned,

    #[error("Node not loaded in map")]
    NodeNotLoaded,

    #[error("Tree not found: {0}")]
    TreeNotFound(String),

    #[error("Gato error {0}")]
    GatoError(String),

    #[error("not a file")]
    NotAFile,
}
pub type VFSResult<T> = Result<T, VFSError>;
