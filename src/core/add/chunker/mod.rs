use std::{
    collections::BTreeMap,
    io::{self, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use bincode::{Decode, Encode};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::core::{
    add::{FileContent, compress, compute_hash, get_file_metadata, index::IndexEntry, smart_read},
    commit::{blob::Blob, error::CommitError},
    error::GatoResult,
    storage::{StorageEngine, StorageError, local::LocalStorage},
};

pub fn cut(data: &FileContent) -> Vec<&[u8]> {
    let min_size = 1024 * 1024; // 1 MB
    let max_size = 8 * 1024 * 1024; // 8 MB
    let avg_size = 4 * 1024 * 1024; // 4 MB

    let chunker = fastcdc::v2020::FastCDC::new(data, min_size, avg_size, max_size);
    chunker
        .map(|chunk| &data[chunk.offset as usize..(chunk.offset + chunk.length) as usize])
        .collect()
}

pub fn process_chunk(chunks: Vec<&[u8]>, storage: &LocalStorage) -> ChunkerResult {
    let mut data = BTreeMap::new();

    let mut ordered_hash = Vec::new();
    let a: Vec<(Vec<u8>, Option<Vec<u8>>)> = chunks
        .par_iter()
        .map(|chunk| {
            let hash = compute_hash(chunk).to_vec();
            if !storage.exist(&hex::encode(&hash)) {
                let compressed_data =
                    compress(chunk, storage.work_dir()).expect("failed to compress chunk");
                (hash, Some(compressed_data))
            } else {
                (hash, None)
            }
        })
        .collect();

    for (hash, compressed_opt) in a {
        ordered_hash.push(hash.clone());

        if let Some(compressed) = compressed_opt {
            data.insert(hash, compressed);
        }
    }

    ChunkerResult {
        chunks: data,
        ordered_hashes: ordered_hash,
    }
}
// return BTreeMap of Hash -> Chunk Data
// pub fn hash(chunks: Vec<Vec<u8>>) -> ChunkerResult {
//     let mut data = BTreeMap::new();
//     let mut ordered = Vec::new();
//     for chunk in chunks {
//         let hash = blake3::hash(&chunk);
//         data.insert(hash.as_bytes().to_vec(), chunk);
//         ordered.push(hash.as_bytes().to_vec());
//     }
//     ChunkerResult {
//         chunks: data,
//         ordered_hashes: ordered,
//     }
// }
#[derive(Debug, Encode, Decode)]
pub struct IndexData {
    pub path: Vec<Vec<u8>>,
}

impl IndexData {
    pub fn restore_file(
        self,
        target_path: &Path,
        storage: &LocalStorage,
    ) -> Result<(), StorageError> {
        let mut file = std::fs::File::create(target_path)?;
        for chunk_hash in &self.path {
            let hash_hex = hex::encode(chunk_hash);

            let compressed_data = storage.get(&hash_hex)?;

            let raw_data = crate::core::add::decompress(&compressed_data)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Decompression failed"))?;

            file.write_all(&raw_data)?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkerResult {
    pub chunks: BTreeMap<Vec<u8>, Vec<u8>>,
    pub ordered_hashes: Vec<Vec<u8>>,
}

impl ChunkerResult {
    pub fn save_chunks(&self, storage: &impl StorageEngine) {
        self.chunks.par_iter().for_each(|(hash, data)| {
            match storage.put(&hex::encode(hash), data.to_vec()) {
                Ok(_) => {}
                Err(e) => println!("{e}"),
            }
        });
    }

    pub fn index_data(&self) -> Result<Vec<u8>, CommitError> {
        let index_data = IndexData {
            path: self.ordered_hashes.clone(),
        };
        let blob_data = Blob::ChunksMap(index_data);
        let bindata = blob_data.encode()?;
        Ok(bindata)
    }
}

pub fn add_as_chunk(
    path: &Path,
    storage: &LocalStorage,
) -> Result<(PathBuf, IndexEntry, Vec<String>), CommitError> {
    let buffer = smart_read(path)?;

    let chunker_result = process_chunk(cut(&buffer), storage);
    let mut hashs: Vec<String> = chunker_result
        .ordered_hashes
        .clone()
        .iter()
        .map(|e| hex::encode(e))
        .collect();
    chunker_result.save_chunks(storage);
    let file_data = chunker_result.index_data()?;
    let file_hash = blake3::hash(&file_data).as_bytes().to_vec();
    let hash_str = hex::encode(file_hash.clone());

    storage.put(&hash_str, file_data)?;
    hashs.push(hash_str);
    let metadata = get_file_metadata(path)?;
    let index = IndexEntry {
        hash: file_hash,
        size: buffer.len() as u64,
        mtime: metadata.mtime() as u32,
        mode: metadata.mode(),
    };
    Ok((path.to_owned(), index, hashs))
}

pub fn get_dry_chunck_hash(path: &Path, storage: &LocalStorage) -> GatoResult<String> {
    let buffer = smart_read(path)?;

    let chunker_result = process_chunk(cut(&buffer), storage);

    // chunker_result.save_chunks(storage);
    let file_data = chunker_result.index_data()?;
    let file_hash = blake3::hash(&file_data).as_bytes().to_vec();
    let hash_str = hex::encode(file_hash.clone());

    Ok(hex::encode(hash_str))
}
