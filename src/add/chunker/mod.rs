use std::{
    collections::BTreeMap,
    io::{self, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use bincode::{Decode, Encode, encode_to_vec};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::add::{
    FileContent, compress, get_file_metadata, index::IndexEntry, smart_read, write_blob,
};

pub fn cut_compress(data: &FileContent) -> Vec<Vec<u8>> {
    let mut compressed_chuncks = Vec::new();
    let min_size = 1024 * 1024; // 1 MB
    let max_size = 8 * 1024 * 1024; // 8 MB
    let avg_size = 4 * 1024 * 1024; // 2 MB

    let chunker = fastcdc::v2020::FastCDC::new(data, min_size, avg_size, max_size);
    for chunk in chunker {
        let chunk_data = &data[chunk.offset as usize..(chunk.offset + chunk.length) as usize];
        let compressed_data = compress(chunk_data).expect("Compression failed");
        compressed_chuncks.push(compressed_data);
    }
    compressed_chuncks
}

// return BTreeMap of Hash -> Chunk Data
pub fn hash(chunks: Vec<Vec<u8>>) -> ChunkerResult {
    let mut data = BTreeMap::new();
    let mut ordered = Vec::new();
    for chunk in chunks {
        let hash = blake3::hash(&chunk);
        data.insert(hash.as_bytes().to_vec(), chunk);
        ordered.push(hash.as_bytes().to_vec());
    }
    ChunkerResult {
        chunks: data,
        ordered_hashes: ordered,
    }
}
#[derive(Debug, Encode, Decode)]
pub struct IndexData {
    pub path: Vec<Vec<u8>>,
}

impl IndexData {
    pub fn restore_file(self, target_path: &Path) -> io::Result<()> {
        let mut file = std::fs::File::create(target_path)?;
        for chunk_hash in &self.path {
            let hash_hex = hex::encode(chunk_hash);
            let chunk_path = PathBuf::from(".gato")
                .join("objects")
                .join(&hash_hex[..2])
                .join(&hash_hex[2..]);

            let compressed_data = smart_read(&chunk_path)?;

            let raw_data = crate::add::decompress(&compressed_data)
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
    pub fn save_chunks(&self) {
        self.chunks.par_iter().for_each(|(hash, data)| {
            let path = PathBuf::from(".gato")
                .join("objects")
                .join(hex::encode(&hash)[..2].to_string())
                .join(hex::encode(&hash)[2..].to_string());
            write_blob(data, &path).expect("failed to write chunk!");
        });
    }

    pub fn index_data(&self) -> Vec<u8> {
        let index_data = IndexData {
            path: self.ordered_hashes.clone(),
        };
        let bindata = encode_to_vec(index_data, bincode::config::standard())
            .expect("failed to encode index data");
        bindata
    }
}

pub fn add_as_chunk(path: &Path) -> io::Result<(PathBuf, IndexEntry)> {
    let buffer = smart_read(path)?;
    let chunks = cut_compress(&buffer);
    let chunker_result = hash(chunks);
    chunker_result.save_chunks();
    let file_data = chunker_result.index_data();
    let file_hash = blake3::hash(&file_data).as_bytes().to_vec();
    let target_path = PathBuf::from(".gato")
        .join("objects")
        .join(hex::encode(&file_hash)[..2].to_string())
        .join(hex::encode(&file_hash)[2..].to_string());
    write_blob(&file_data, &target_path)?;
    let metadata = get_file_metadata(path)?;
    let index = IndexEntry {
        hash: file_hash,
        size: buffer.len() as u64,
        mtime: metadata.mtime() as u32,
        mode: metadata.mode(),
    };
    Ok((path.to_owned(), index))
}
