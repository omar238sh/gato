use flate2::Compression;

use memmap2::Mmap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::fs::read_dir;
use std::io::Write;
use std::io::{self, Read};

use std::ops::Deref;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::add::chunker::add_as_chunk;
use crate::core::add::index::{Index, IndexEntry};
use crate::core::commit::blob::Blob;
use crate::core::config::load::load_config;

use crate::core::error::{self, GatoResult};
use crate::core::storage::StorageEngine;
use crate::core::storage::local::LocalStorage;

pub mod chunker;
pub mod index;
pub enum FileContent {
    Mmapped(Mmap),
    Loaded(Vec<u8>),
}

impl FileContent {
    pub fn to_str(&self) -> io::Result<&str> {
        match self {
            FileContent::Mmapped(m) => std::str::from_utf8(&m).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("UTF-8 error: {}", e))
            }),
            FileContent::Loaded(v) => std::str::from_utf8(&v).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("UTF-8 error: {}", e))
            }),
        }
    }
}

impl Deref for FileContent {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            FileContent::Mmapped(m) => m,
            FileContent::Loaded(v) => v,
        }
    }
}

pub fn smart_read(path: &Path) -> io::Result<FileContent> {
    let file = std::fs::File::open(path)?;
    let meta_data = file.metadata()?;
    let file_size = meta_data.len();

    if file_size > 16 * 1024 {
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(FileContent::Mmapped(mmap))
    } else {
        let mut reader = io::BufReader::new(file);
        let mut contents = Vec::with_capacity(file_size as usize);
        reader.read_to_end(&mut contents)?;
        return Ok(FileContent::Loaded(contents));
    }
}

pub fn compress_zlib(data: &[u8]) -> GatoResult<Vec<u8>> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&*data)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

fn compress_zstd(data: &[u8], level: i32) -> GatoResult<Vec<u8>> {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), level)
        .expect("Failed to create zstd encoder");
    encoder
        .write_all(data)
        .expect("Failed to write data to zstd encoder");
    let compressed_data = encoder.finish()?;
    Ok(compressed_data)
}

pub fn decompress_zlib(data: &[u8]) -> GatoResult<Vec<u8>> {
    let mut decoder = flate2::read::ZlibDecoder::new(&data[..]);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

pub fn decompress_zstd(data: &[u8]) -> GatoResult<Vec<u8>> {
    let mut decoder =
        zstd::stream::read::Decoder::new(&data[..]).expect("Failed to create zstd decoder");
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .expect("Failed to read data from zstd decoder");
    Ok(decompressed_data)
}

pub fn compress(data: &[u8], work_dir: &PathBuf) -> GatoResult<Vec<u8>> {
    let config = load_config(work_dir)?;
    match config.compression {
        Some(v) => match v.method {
            crate::core::config::CompressionMethod::Zlib => {
                return compress_zlib(data);
            }
            crate::core::config::CompressionMethod::Zstd => {
                return compress_zstd(data, v.level.unwrap_or(1));
            }
        },
        None => {
            return compress_zstd(data, 1);
        }
    }
}

pub fn decompress(data: &[u8], work_dir: &PathBuf) -> GatoResult<Vec<u8>> {
    let config = load_config(work_dir)?;

    match config.compression {
        Some(v) => match v.method {
            crate::core::config::CompressionMethod::Zlib => decompress_zlib(data),
            crate::core::config::CompressionMethod::Zstd => decompress_zstd(data),
        },
        None => decompress_zstd(data),
    }
}

// pub fn write_blob(compressed_data: &Vec<u8>, outpath: &Path) -> io::Result<()> {
//     let exist = outpath.exists();
//     if exist {
//         Ok(())
//     } else {
//         if let Some(parent) = outpath.parent() {
//             std::fs::create_dir_all(parent)?;
//         }
//         let mut out_file = std::fs::File::create(outpath)?;
//         out_file.write_all(&compressed_data)?;
//         Ok(())
//     }
// }

pub fn find_files(dir_path: &Path, storage: &LocalStorage) -> GatoResult<Vec<PathBuf>> {
    let ignored = read_gatoignore(storage)?;

    let mut files_paths: Vec<PathBuf> = Vec::new();
    if dir_path.is_dir() {
        let mut entryies = read_dir(dir_path)?.into_iter();
        while let Some(Ok(entry)) = entryies.next() {
            let path = entry.path();
            if is_ignored(&path, &ignored) {
                continue;
            };
            if path.is_file() {
                files_paths.push(path);
            } else if path.is_dir() {
                let mut nested_files = find_files(&path, storage)?;
                files_paths.append(&mut nested_files);
            }
        }
    }
    Ok(files_paths)
}

pub fn get_file_metadata(path: &Path) -> io::Result<std::fs::Metadata> {
    std::fs::metadata(path)
}

pub fn compute_hash(data: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(&data);
    let hash = hash.as_bytes();
    *hash
}

pub fn add_file(file_path: &Path, storage: &LocalStorage) -> GatoResult<index::IndexEntry> {
    let buffer = smart_read(file_path)?;
    let hash = compute_hash(&buffer);
    let hash_str = hex::encode(hash);

    if !storage.exist(&hash_str) {
        let compressed_data = compress(&buffer, storage.work_dir())?;
        let data = Blob::Normal(compressed_data);

        storage.put(&hash_str, data.encode()?)?;
    }

    let metadata = get_file_metadata(file_path)?;
    let index_entry = index::IndexEntry {
        hash: hash.to_vec(),
        size: metadata.len(),
        mtime: metadata.modified()?.elapsed().unwrap().as_secs() as u32,
        #[cfg(unix)]
        mode: metadata.permissions().mode(),
        #[cfg(not(unix))]
        mode: 0,
    };

    Ok(index_entry)
}

pub fn add_all(paths: Vec<PathBuf>, storage: Arc<LocalStorage>) -> GatoResult<()> {
    let mut index = Index::load(storage.as_ref()).unwrap_or(Index::new());
    let new_entries: Vec<Result<(PathBuf, IndexEntry, Vec<String>), error::Error>> = paths
        .par_iter()
        .map(|path| {
            let file_len = get_file_metadata(&storage.work_dir().join(path))?.len();
            if file_len < 1024 * 1024 * 8 {
                let storage_clone = Arc::clone(&storage);
                let entry = add_file(&storage_clone.work_dir().join(path), storage_clone.as_ref())?;
                let deps = vec![hex::encode(&entry.hash)];
                Ok((path.clone(), entry, deps))
            } else {
                let storage_clone = Arc::clone(&storage);
                let (_, entry, hashs) =
                    add_as_chunk(&storage_clone.work_dir().join(path), storage_clone.as_ref())?;

                Ok((path.clone(), entry, hashs))
            }
        })
        .collect();
    for result in new_entries {
        match result {
            Ok((path, entry, deps)) => {
                index.add_entry(path, entry.clone());
                index.dependencies.extend(deps);
            }
            Err(e) => {
                eprintln!("Failed to process file: {}", e);
                return Err(e);
            }
        }
    }
    index.save(storage.as_ref())?;
    Ok(())
}

pub fn read_gatoignore(storage: &LocalStorage) -> GatoResult<Vec<String>> {
    Ok(load_config(storage.work_dir())?.ignored())
}

pub fn is_ignored(path: &Path, ignored_patterns: &[String]) -> bool {
    for component in path.components() {
        if let Some(comp_str) = component.as_os_str().to_str() {
            for pattern in ignored_patterns {
                if comp_str == pattern {
                    return true;
                }
            }
        }
    }
    false
}

// pub fn get_branch_head(storage: &LocalStorage) -> io::Result<String> {
//     let head_path = storage.repo_path().join("HEAD");
//     let head_content = smart_read(&head_path)?;
//     let head_str = String::from_utf8(head_content.to_vec()).expect("Invalid UTF-8 in HEAD file");
//     Ok(head_str.trim().to_string())
// }
