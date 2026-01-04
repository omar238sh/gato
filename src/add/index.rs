#![allow(dead_code)]

use std::{collections::BTreeMap, path::PathBuf};

use bincode::{
    Decode, Encode,
    config::{self},
};

const INDEX_FILE: &str = "./.gato/index";

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
}

impl Index {
    pub fn new() -> Self {
        Index {
            entries: BTreeMap::new(),
        }
    }

    pub fn add_entry(&mut self, path: PathBuf, entry: IndexEntry) {
        self.entries.insert(path, entry);
    }

    pub fn get_entry(&self, path: &PathBuf) -> Option<&IndexEntry> {
        self.entries.get(path)
    }

    pub fn load() -> std::io::Result<Self> {
        let data = std::fs::read(INDEX_FILE)?;
        let (index, _): (Index, usize) =
            bincode::decode_from_slice(&data.as_slice(), config::standard())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(index)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let encoded: Vec<u8> =
            bincode::encode_to_vec(self, config::standard()).expect("Encoding failed");
        std::fs::write(INDEX_FILE, encoded)?;
        Ok(())
    }

    pub fn debug_print(&self) {
        for (path, entry) in &self.entries {
            println!("{:?} => {:?}", path, entry);
        }
    }
}
