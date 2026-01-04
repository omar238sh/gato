use std::{
    collections::BTreeMap,
    fmt::Display,
    fs::{self, write},
    path::{Path, PathBuf},
};

use bincode::{Decode, Encode, config, encode_to_vec};
use blake3::hash;

use crate::add::{get_branch_head, index::Index};

#[derive(Encode, Decode, Debug, Clone)]
pub struct Commit {
    message: String,
    author: String,
    timestamp: u64,
    email: String,
    tree_hash: Vec<u8>,
    parent_hash: Option<Vec<u8>>,
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parent_hash_str = match &self.parent_hash {
            Some(hash) => hex::encode(hash),
            None => "None".to_string(),
        };
        write!(
            f,
            "Commit Message: {}\nAuthor: {}\nEmail: {}\nTimestamp: {}\nTree Hash: {}\nParent Hash: {}\n",
            self.message,
            self.author,
            self.email,
            self.timestamp,
            hex::encode(&self.tree_hash),
            parent_hash_str
        )
    }
}

impl Commit {
    pub fn save(&self) {
        let data = encode_to_vec(self, config::standard()).expect("Encoding failed");
        let branch = get_branch_head().expect("Failed to get branch head");
        let hash = hash(&data);
        let hash_hex = hash.to_hex().to_string();
        let hash_bytes = hash.as_bytes().to_vec();

        let dir_path = format!(".gato/objects/{}", hash_hex[..2].to_string());
        let file_path = format!("{}/{}", dir_path, hash_hex[2..].to_string());
        std::fs::create_dir_all(dir_path).expect("Failed to create objects directory");
        write(file_path, data).expect("Failed to write commit object");
        write(
            PathBuf::from(".gato")
                .join("refs")
                .join("heads")
                .join(branch),
            hash_bytes,
        )
        .unwrap();
    }

    pub fn get_parent_hash() -> Option<Vec<u8>> {
        let branch = get_branch_head().expect("Failed to get branch head");
        let hash = fs::read(
            PathBuf::from(".gato")
                .join("refs")
                .join("heads")
                .join(branch),
        )
        .ok();
        hash
    }

    pub fn new(message: String, author: String) -> Self {
        let tree_hash = Tree::create_from_index(Index::load().expect("Failed to load index"));
        let parent_hash = Self::get_parent_hash();
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let email = format!("{}@gato.com", author);

        Commit {
            message,
            author,
            timestamp,
            email,
            tree_hash,
            parent_hash,
        }
    }

    pub fn load(hash: String) -> Self {
        let path = format!(
            ".gato/objects/{}/{}",
            hash[..2].to_string(),
            hash[2..].to_string()
        );
        let data = std::fs::read(path).expect("cannot open file!");
        let commit: Commit = bincode::decode_from_slice(&data, config::standard())
            .expect("Decoding failed")
            .0;
        commit
    }

    pub fn get_last_commit_hash() -> Option<String> {
        let hash_bytes = Self::get_parent_hash()?;
        let hash_str = hex::encode(hash_bytes);
        Some(hash_str)
    }

    pub fn get_hash_from_index(index: usize) -> Option<String> {
        let mut current_hash = Self::get_last_commit_hash()?;
        for _ in 0..index {
            let commit = Commit::load(current_hash);
            match commit.parent_hash {
                Some(parent_hash) => {
                    current_hash = hex::encode(parent_hash);
                }
                None => return None,
            }
        }
        Some(current_hash)
    }

    pub fn load_by_index(index: usize) -> Option<Self> {
        let hash = Self::get_hash_from_index(index)?;
        let commit = Commit::load(hash);
        Some(commit)
    }

    pub fn write_tree(&self, out_path: &Path) {
        let tree_hash_hex = hex::encode(&self.tree_hash);
        let tree = Tree::load(tree_hash_hex);
        for entry in tree.entries {
            entry.write(out_path);
        }
    }
}

#[derive(Encode, Decode, Debug, Clone)]
enum TreeEntry {
    Blob(String, Vec<u8>), // hash of the blob
    Tree(String, Vec<u8>), // hash of the tree
}

impl TreeEntry {
    fn write(&self, parent_path: &Path) {
        match self {
            TreeEntry::Blob(name, hash) => {
                let hash_hex = String::from_utf8(hash.clone()).expect("Invalid UTF-8 in hash");
                let compressed_file_path = PathBuf::new().join(format!(
                    ".gato/objects/{}/{}",
                    &hash_hex[..2],
                    &hash_hex[2..]
                ));

                let path = parent_path.join(name);
                let blob = crate::add::smart_read(&compressed_file_path);
                match blob {
                    Ok(v) => {
                        let decompressed_data = crate::add::decompress(&v).unwrap();
                        write(&path, decompressed_data)
                            .expect(&format!("cannot write file: {name}"));
                    }
                    Err(_) => {
                        panic!("cannot read file: {}", name)
                    }
                }
            }
            TreeEntry::Tree(name, items) => {
                let tree_hash_hex = hex::encode(items);
                let tree = Tree::load(tree_hash_hex);
                let dir_path = parent_path.join(name);
                fs::create_dir_all(&dir_path).expect("cannot create directory!");
                for entry in tree.entries {
                    entry.write(&dir_path);
                }
            }
        }
    }
}

#[derive(Encode, Decode, Debug, Clone)]
struct Tree {
    name: String,
    entries: Vec<TreeEntry>, // name , entry
}

// README.md
// src/main.rs
// src/model/user.rs
//
// Tree root { <README.md , Blob(hash)> }
// Tree root { <src , Tree(hash)> , <README.md , Blob(hash)> } => Tree src {<main.rs , Blob(hash)>}
// Tree root { <src , Tree(hash)> , <README.md , Tree(hash)> } => Tree src {<main.rs , Blob(hash)> , <model , Tree(hash)>} => Tree model {<user.rs , Blob(hash)>}

impl Tree {
    fn new(name: String) -> Self {
        Tree {
            name: name,
            entries: Vec::new(),
        }
    }

    fn add_entry(&mut self, entry: TreeEntry) {
        self.entries.push(entry);
    }

    fn into_entry(&self) -> TreeEntry {
        TreeEntry::Tree(self.name.clone(), self.hash())
    }

    // encode Object to bincode bytes
    fn tree_bytes(&self) -> Vec<u8> {
        let tree_data = encode_to_vec(self, config::standard()).expect("Encoding failed");
        tree_data
    }

    // hash the tree object
    fn hash_str(&self) -> String {
        let hash = hash(&self.tree_bytes());
        let hash_hex = hash.to_hex().to_string();
        hash_hex
    }

    fn hash(&self) -> Vec<u8> {
        let hash = hash(&self.tree_bytes());
        // let hash_hex = hash;
        hash.as_bytes().to_vec()
    }

    // save the tree object to .gato/objects/<first 2 chars>/<rest chars>
    fn save(&self) {
        let tree_hash = self.hash_str();
        let tree_data = self.tree_bytes();
        let dir_path = format!(".gato/objects/{}", tree_hash[..2].to_string());
        let file_path = format!("{}/{}", dir_path, tree_hash[2..].to_string());
        std::fs::create_dir_all(dir_path).expect("Failed to create objects directory");
        std::fs::write(file_path, tree_data).expect("Failed to write tree object");
    }
    // load tree object from .gato/objects/<first 2 chars>/<rest chars>
    fn load(hash: String) -> Self {
        let path = format!(
            ".gato/objects/{}/{}",
            hash[..2].to_string(),
            hash[2..].to_string()
        );
        let data = std::fs::read(path).expect("cannot open file!");
        let tree: Tree = bincode::decode_from_slice(&data, config::standard())
            .expect("Decoding failed")
            .0;
        tree
    }
    // return hash of the root tree created from index
    pub fn create_from_index(index: Index) -> Vec<u8> {
        let entries: Vec<(PathBuf, Vec<u8>)> = index
            .entries
            .into_iter()
            .map(|(path, entry)| (path, entry.hash))
            .collect();

        let root_tree_entry = Self::build_recursive_tree(entries, "root".to_string());

        match root_tree_entry {
            TreeEntry::Tree(_, hash) => hash,
            _ => panic!("Root is not a tree!"),
        }
    }

    // recursively build tree from entries
    fn build_recursive_tree(entries: Vec<(PathBuf, Vec<u8>)>, name: String) -> TreeEntry {
        let mut current_tree = Tree::new(name.clone());

        let mut groups: BTreeMap<String, Vec<(PathBuf, Vec<u8>)>> = BTreeMap::new();

        for (path, hash) in entries {
            let mut components = path.components();

            if let Some(component) = components.next() {
                let component_str = component.as_os_str().to_string_lossy().to_string();
                let remaining_path: PathBuf = components.as_path().to_path_buf();

                if remaining_path.as_os_str().is_empty() {
                    current_tree.add_entry(TreeEntry::Blob(component_str, hash));
                } else {
                    groups
                        .entry(component_str)
                        .or_default()
                        .push((remaining_path, hash));
                }
            }
        }

        for (folder_name, sub_entries) in groups {
            let subtree_entry = Self::build_recursive_tree(sub_entries, folder_name);
            current_tree.add_entry(subtree_entry);
        }

        current_tree.save();
        current_tree.into_entry()
    }
}
