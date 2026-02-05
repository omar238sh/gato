use thiserror::Error;
mod gc;
pub mod local;
pub trait StorageEngine: Send + Sync {
    fn get(&self, hash: &String) -> Result<Vec<u8>, StorageError>;

    fn put(&self, hash: &String, data: Vec<u8>) -> Result<(), StorageError>;

    fn exist(&self, hash: &String) -> bool;

    fn write_ref(&self, ref_name: String, hash: Vec<u8>) -> Result<(), StorageError>;

    // fn read_ref(&self, ref_name: String) -> Result<String, StorageError>;

    fn setup(&self) -> Result<(), StorageError>;

    fn new_branch(&self, name: String) -> Result<(), StorageError>;

    fn change_branch(&self, name: String) -> Result<(), StorageError>;
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Read error")]
    ReadError,

    #[error("Write error")]
    WriteError,

    // #[error("write or read ref failed")]
    // RefError,
    // #[error("uninitialized repository")]
    // UninitializedRepository,
    #[error("IO error")]
    IoError(#[from] std::io::Error),
}
