use flate2::Compression;

use memmap2::Mmap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs::read_dir;
use std::io::Write;
use std::io::{self, Read};

use std::ops::Deref;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::add::index::{Index, IndexEntry};
use crate::config::load::load_config;
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

// pub fn read_buffer(path: &Path) -> io::Result<Vec<u8>> {
//     let file = std::fs::File::open(path)?;
//     let mut buf_reader = io::BufReader::new(file);
//     let mut contents: Vec<u8> = vec![];
//     buf_reader.read_to_end(&mut contents)?;
//     Ok(contents)
// }

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

pub fn compress_zlib(data: &FileContent) -> io::Result<Vec<u8>> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&*data)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

fn compress_zstd(data: &FileContent, level: i32) -> io::Result<Vec<u8>> {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), level)
        .expect("Failed to create zstd encoder");
    encoder
        .write_all(data)
        .expect("Failed to write data to zstd encoder");
    let compressed_data = encoder.finish()?;
    Ok(compressed_data)
}

pub fn decompress_zlib(data: &FileContent) -> io::Result<Vec<u8>> {
    let mut decoder = flate2::read::ZlibDecoder::new(&data[..]);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

pub fn decompress_zstd(data: &FileContent) -> io::Result<Vec<u8>> {
    let mut decoder =
        zstd::stream::read::Decoder::new(&data[..]).expect("Failed to create zstd decoder");
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .expect("Failed to read data from zstd decoder");
    Ok(decompressed_data)
}

pub fn compress(data: &FileContent) -> io::Result<Vec<u8>> {
    let config = load_config();
    match config.compression {
        Some(v) => match v.method {
            crate::config::CompressionMethod::Zlib => {
                return compress_zlib(data);
            }
            crate::config::CompressionMethod::Zstd => {
                return compress_zstd(data, v.level.unwrap_or(1));
            }
        },
        None => {
            return compress_zstd(data, 1);
        }
    }
}

pub fn decompress(data: &FileContent) -> io::Result<Vec<u8>> {
    let config = load_config();

    match config.compression {
        Some(v) => match v.method {
            crate::config::CompressionMethod::Zlib => decompress_zlib(data),
            crate::config::CompressionMethod::Zstd => decompress_zstd(data),
        },
        None => decompress_zstd(data),
    }
}

pub fn write_blob(compressed_data: Vec<u8>, outpath: &Path) -> io::Result<()> {
    if let Some(parent) = outpath.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out_file = std::fs::File::create(outpath)?;
    out_file.write_all(&compressed_data)?;
    Ok(())
}

pub fn find_files(dir_path: &Path) -> io::Result<Vec<PathBuf>> {
    let ignored = read_gatoignore();

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
                let mut nested_files = find_files(&path)?;
                files_paths.append(&mut nested_files);
            }
        }
    }
    Ok(files_paths)
}

pub fn get_file_metadata(path: &Path) -> io::Result<std::fs::Metadata> {
    std::fs::metadata(path)
}

pub fn compute_hash(data: FileContent) -> io::Result<String> {
    let hash = blake3::hash(&data);
    let hash_str = hash.to_hex().to_string();
    Ok(hash_str)
}

pub fn add_file(file_path: &Path) -> io::Result<IndexEntry> {
    let buffer = smart_read(file_path)?;
    let compressed_data = compress(&buffer)?;
    let hash_str = compute_hash(buffer)?;

    let out_path = PathBuf::from(".gato")
        .join("objects")
        .join(&hash_str[..2])
        .join(&hash_str[2..]);
    write_blob(compressed_data, out_path.as_path())?;

    let metadata = get_file_metadata(file_path)?;
    let index_entry = index::IndexEntry {
        hash: hash_str.as_bytes().to_vec(),
        size: metadata.len(),
        mtime: metadata.modified()?.elapsed().unwrap().as_secs() as u32,
        mode: metadata.permissions().mode(),
    };

    Ok(index_entry)
}

pub fn add_all(paths: Vec<PathBuf>) -> io::Result<()> {
    let mut index = Index::load().unwrap_or(Index::new());
    let new_entries: Vec<io::Result<(PathBuf, IndexEntry)>> = paths
        .par_iter()
        .map(|path| {
            let entry = add_file(&path)?;
            Ok((path.clone(), entry))
        })
        .collect();
    for result in new_entries {
        match result {
            Ok((path, entry)) => {
                index.add_entry(path, entry);
            }
            Err(e) => {
                eprintln!("Failed to process file: {}", e);
                return Err(e);
            }
        }
    }
    index.save()?;
    Ok(())
}

pub fn read_gatoignore() -> Vec<String> {
    let path = Path::new(".gatoignore");

    if !path.exists() {
        return Vec::new();
    }

    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
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

pub fn get_branch_head() -> io::Result<String> {
    let head_path = Path::new(".gato/HEAD");
    let head_content = smart_read(head_path)?;
    let head_str = std::str::from_utf8(&head_content).expect("Invalid UTF-8 in HEAD file");
    Ok(head_str.trim().to_string())
}

// #[test]
// fn compress_test() {
//     let input_file = Path::new("/home/omar/Downloads/100mb-examplefile-com.txt");
//     let out_path = Path::new("./compressed.zlib");
//     let buffer = read_buffer(input_file).unwrap();
//     let compressed_data = compress(buffer).unwrap();
//     write_blob(compressed_data, out_path).unwrap();
// }

// #[test]
// fn find_files_test() {
//     let dir_path = Path::new("/home/omar/Downloads");
//     let files = find_files(dir_path).unwrap();
//     for file in files {
//         println!("{:?}", file);
//     }
// }

// #[test]
// fn get_file_metadata_test() {
//     let file_path = Path::new("/home/omar/Downloads/100mb-examplefile-com.txt");
//     let metadata = get_file_metadata(file_path).unwrap();
//     println!("File size: {}", metadata.len());
//     println!("Is file: {}", metadata.is_file());
//     println!("Is dir: {}", metadata.is_dir());
//     println!("Readonly: {:?}", metadata.permissions());
//     // type
//     println!("Created: {:?}", metadata.created());
//     println!("Modified: {:?}", metadata.modified());
//     println!("Accessed: {:?}", metadata.accessed());
//     println!("type {:?}", metadata.file_type());
// }

#[test]
fn print_all_entries() {
    // let i = Instant::now();
    let index = Index::load().unwrap();
    // let time = i.elapsed();
    // println!("Time taken to load index: {:?}", time);
    index.debug_print();
}
