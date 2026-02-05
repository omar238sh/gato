use std::path::PathBuf;

use bincode::{Decode, Encode, encode_to_vec};

use crate::core::{
    add::chunker::IndexData, commit::error::CommitError, storage::local::LocalStorage,
};

#[derive(Debug, Decode, Encode)]
pub enum Blob {
    Normal(Vec<u8>),
    ChunksMap(IndexData),
}

impl Blob {
    pub fn restore(self, path: PathBuf, storage: &LocalStorage) -> Result<(), CommitError> {
        match self {
            Blob::Normal(content) => {
                let decompressed_data =
                    crate::core::add::decompress(&content, storage.work_dir()).unwrap();
                std::fs::write(&path, decompressed_data)?;
            }
            Blob::ChunksMap(index_data) => {
                index_data.restore_file(&path, storage)?;
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, CommitError> {
        let bindata = encode_to_vec(self, bincode::config::standard())?;
        Ok(bindata)
    }
}
