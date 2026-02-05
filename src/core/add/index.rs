use std::{collections::BTreeMap, path::PathBuf};

use bincode::{
    Decode, Encode,
    config::{self},
};

use crate::core::storage::local::LocalStorage;

#[derive(Encode, Decode, Debug, Clone)]
pub struct IndexEntry {
    pub hash: Vec<u8>,
    pub size: u64,
    pub mtime: u32,
    pub mode: u32,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct Index {
    pub entries: BTreeMap<PathBuf, IndexEntry>,
    pub dependencies: Vec<String>,
}

impl Index {
    pub fn new() -> Self {
        Index {
            entries: BTreeMap::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn index_file_path(storage: &LocalStorage) -> PathBuf {
        storage.repo_path().join("index")
    }

    // pub fn add_dependency(&mut self, dependency: String) {
    //     if !self.dependencies.contains(&dependency) {
    //         self.dependencies.push(dependency);
    //     }
    // }

    pub fn add_entry(&mut self, path: PathBuf, entry: IndexEntry) {
        self.entries.insert(path, entry);
    }

    // pub fn get_entry(&self, path: &PathBuf) -> Option<&IndexEntry> {
    //     self.entries.get(path)
    // }

    pub fn load(storage: &LocalStorage) -> std::io::Result<Self> {
        let data = std::fs::read(Self::index_file_path(&storage))?;
        let (index, _): (Index, usize) =
            bincode::decode_from_slice(&data.as_slice(), config::standard())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(index)
    }

    pub fn save(&self, storage: &LocalStorage) -> std::io::Result<()> {
        let encoded: Vec<u8> =
            bincode::encode_to_vec(self, config::standard()).expect("Encoding failed");
        std::fs::write(Self::index_file_path(storage), encoded)?;
        Ok(())
    }

    // pub fn debug_print(&self) {
    //     for (path, entry) in &self.entries {
    //         println!("{:?} => {:?}", path, entry);
    //     }
    // }
}
