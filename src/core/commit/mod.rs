use std::{
    collections::BTreeMap,
    fmt::Display,
    fs::{self},
    path::{Path, PathBuf},
};

use bincode::{
    Decode, Encode,
    config::{self},
    decode_from_slice, encode_to_vec,
};
use blake3::hash;
use diffy::merge;
use tracing::instrument;

use crate::core::{
    add::{add_file_dry, index::Index},
    commit::{blob::Blob, error::CommitError},
    config::load::load_config,
    error::{Error, GatoResult},
    storage::{StorageEngine, local::LocalStorage},
};
pub mod blob;
pub mod error;

#[derive(Encode, Decode, Debug, Clone)]
pub enum Commit {
    V1 {
        message: String,
        author: String,
        timestamp: u64,
        email: Option<String>,
        tree_hash: Vec<u8>,
        parent_hash: Option<Vec<u8>>,
        dependencies: Vec<String>,
    },
    MergedCommitV1 {
        message: String,
        author: String,
        timestamp: u64,
        email: Option<String>,
        tree_hash: Vec<u8>,
        parent_hash1: Vec<u8>,
        parent_hash2: Vec<u8>,
        dependencies: Vec<String>,
    },
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commit::V1 {
                message,
                author,
                timestamp,
                email,
                tree_hash,
                parent_hash,
                dependencies,
            } => {
                let parent_hash_str = parent_hash
                    .as_ref()
                    .map(|h| hex::encode(h))
                    .unwrap_or_else(|| "None".to_string());

                let deps_str = if dependencies.is_empty() {
                    "None".to_string()
                } else {
                    dependencies.join(", ")
                };

                let email_str = email.as_ref().map(|e| e.as_str()).unwrap_or("None");

                write!(
                    f,
                    "Commit (V1):\nMessage: {}\nAuthor: {}\nEmail: {}\nTimestamp: {}\nTree Hash: {}\nParent Hash: {}\nDependencies: {}\n",
                    message,
                    author,
                    email_str,
                    timestamp,
                    hex::encode(tree_hash),
                    parent_hash_str,
                    deps_str
                )
            }
            Commit::MergedCommitV1 {
                message,
                author,
                timestamp,
                email,
                tree_hash,
                parent_hash1,
                parent_hash2,
                dependencies,
            } => {
                let deps_str = if dependencies.is_empty() {
                    "None".to_string()
                } else {
                    dependencies.join(", ")
                };

                let email_str = email.as_ref().map(|e| e.as_str()).unwrap_or("None");

                write!(
                    f,
                    "Merged Commit (V1):\nMessage: {}\nAuthor: {}\nEmail: {}\nTimestamp: {}\nTree Hash: {}\nParent Hash 1: {}\nParent Hash 2: {}\nDependencies: {}\n",
                    message,
                    author,
                    email_str,
                    timestamp,
                    hex::encode(tree_hash),
                    hex::encode(parent_hash1),
                    hex::encode(parent_hash2),
                    deps_str
                )
            }
        }
    }
}

impl Commit {
    #[instrument]
    pub fn save(&self, storage: &LocalStorage) -> Result<(), CommitError> {
        let data = encode_to_vec(self, config::standard())?;

        let hash = hash(&data);
        let hash_hex = hash.to_hex().to_string();
        let hash_bytes = hash.as_bytes().to_vec();

        storage.put(&hash_hex, data)?;
        storage.write_ref(storage.get_active_branche(), hash_bytes)?;
        Ok(())
    }
    #[instrument]
    // pub fn compute_hash(&self) -> String {
    //     let data = encode_to_vec(self, config::standard()).expect("Encoding failed");
    //     let hash = hash(&data);
    //     hash.to_hex().to_string()
    // }
    #[instrument]
    pub fn parents_hashes(&self, storage: &LocalStorage) -> Vec<String> {
        let mut parents = Vec::new();
        let mut c = self.clone();
        while let Some(hash) = c.parent_hash() {
            c = Self::load(hash.clone(), storage);
            parents.push(hash);
        }
        parents
    }
    #[instrument]
    pub fn base(commit_a: &Self, commit_b: &Self, storage: &LocalStorage) -> Option<Self> {
        let parents = commit_a.parents_hashes(storage);
        let parents_b = commit_b.parents_hashes(storage);

        // println!("{parents:?} \n {parents_b:?}");

        for hash in parents_b {
            if parents.contains(&hash) {
                let commit = Self::load(hash, &storage);
                return Some(commit);
            }
        }

        None
    }
    #[instrument]
    pub fn get_parent_hash(storage: &LocalStorage) -> Option<Vec<u8>> {
        let hash = storage.read_ref_vec(storage.get_active_branche()).ok();
        hash
    }
    #[instrument]
    pub fn new(message: String, storage: &LocalStorage) -> GatoResult<Self> {
        let (tree_hash, dependencies) = Tree::create_from_index(
            Index::load(&storage).expect("Failed to load index"),
            storage,
        );
        let author = load_config(storage.work_dir())?.author;
        let parent_hash = Self::get_parent_hash(&storage);
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let email = load_config(storage.work_dir())?.email;
        Ok(Commit::V1 {
            message,
            author,
            timestamp,
            email,
            tree_hash,
            parent_hash,
            dependencies,
        })
    }

    pub fn new_merged(
        message: String,
        tree_hash: Vec<u8>,
        parent_hash1: Vec<u8>,
        parent_hash2: Vec<u8>,
        dependencies: Vec<String>,
        storage: &LocalStorage,
    ) -> GatoResult<Self> {
        let config = load_config(storage.work_dir())?;
        let author = config.author;
        let email = config.email;

        let timestamp = chrono::Utc::now().timestamp() as u64;

        Ok(Commit::MergedCommitV1 {
            message,
            author,
            timestamp,
            email,
            tree_hash,
            parent_hash1,
            parent_hash2,
            dependencies,
        })
    }
    #[instrument]
    pub fn load(hash: String, storage: &LocalStorage) -> Self {
        let data = storage.get(&hash).expect("cannot read this commit");
        let commit: Commit = bincode::decode_from_slice(&data, config::standard())
            .expect("Decoding failed")
            .0;
        commit
    }
    #[instrument]
    pub fn get_last_commit_hash(storage: &LocalStorage) -> Option<String> {
        let hash_bytes = Self::get_parent_hash(&storage)?;
        let hash_str = hex::encode(hash_bytes);
        Some(hash_str)
    }
    #[instrument]
    pub fn get_hash_from_index(index: usize, storage: &LocalStorage) -> Option<String> {
        let mut current_hash = Self::get_last_commit_hash(&storage)?;
        for _ in 0..index {
            let commit = Commit::load(current_hash, storage);
            match commit {
                Commit::V1 { parent_hash, .. } => match parent_hash {
                    Some(parent_hash) => {
                        current_hash = hex::encode(parent_hash);
                    }
                    None => return None,
                },
                Commit::MergedCommitV1 { parent_hash1, .. } => {
                    current_hash = hex::encode(parent_hash1)
                }
            }
        }
        Some(current_hash)
    }
    #[instrument]
    pub fn parent_hash(&self) -> Option<String> {
        match self {
            Commit::V1 { parent_hash, .. } => match parent_hash {
                Some(parent_hash) => Some(hex::encode(parent_hash)),
                None => return None,
            },
            Commit::MergedCommitV1 { parent_hash1, .. } => Some(hex::encode(parent_hash1)),
        }
    }
    #[instrument]
    pub fn dependices(&self) -> Vec<String> {
        match self {
            Commit::V1 { dependencies, .. } => dependencies.clone(),
            Commit::MergedCommitV1 { dependencies, .. } => dependencies.clone(),
        }
    }
    #[instrument]
    pub fn load_by_index(index: usize, storage: &LocalStorage) -> Option<Self> {
        let hash = Self::get_hash_from_index(index, storage)?;
        let commit = Commit::load(hash, storage);
        Some(commit)
    }
    #[instrument]
    pub fn tree_hash(&self) -> Vec<u8> {
        match self {
            Commit::V1 { tree_hash, .. } => tree_hash.clone(),
            Commit::MergedCommitV1 { tree_hash, .. } => tree_hash.clone(),
        }
    }
    #[instrument]
    pub fn write_tree(&self, out_path: &Path, storage: &LocalStorage) -> GatoResult<()> {
        let tree_hash_hex = hex::encode(&self.tree_hash());
        let tree = Tree::load(tree_hash_hex, storage)?;
        for entry in tree.entries {
            entry.write(out_path, storage)?;
        }
        Ok(())
    }
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
enum TreeEntry {
    Blob(String, Vec<u8>), // hash of the blob
    Tree(String, Vec<u8>), // hash of the tree
}

impl TreeEntry {
    #[instrument]
    fn write(&self, parent_path: &Path, storage: &LocalStorage) -> GatoResult<()> {
        match self {
            TreeEntry::Blob(name, hash) => {
                let hash_hex = hex::encode(hash);
                let path = parent_path.join(name);
                let blob = storage.get(&hash_hex)?;

                let data: Blob = decode_from_slice(&blob, config::standard())?.0;
                data.restore(path, storage)?;
            }
            TreeEntry::Tree(name, items) => {
                let tree_hash_hex = hex::encode(items);
                let tree = Tree::load(tree_hash_hex, storage)?;
                let dir_path = parent_path.join(name);
                fs::create_dir_all(&dir_path)?;
                for entry in tree.entries {
                    entry.write(&dir_path, storage)?;
                }
            }
        }
        Ok(())
    }
    #[instrument]
    pub fn name(&self) -> &String {
        match self {
            TreeEntry::Blob(name, _) => name,
            TreeEntry::Tree(name, _) => name,
        }
    }
    #[instrument]
    pub fn hash(&self) -> Vec<u8> {
        match self {
            TreeEntry::Blob(_, items) => items.clone(),
            TreeEntry::Tree(_, items) => items.clone(),
        }
    }
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct Tree {
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
    pub fn new(name: String) -> Self {
        Tree {
            name: name,
            entries: Vec::new(),
        }
    }
    #[instrument]
    fn add_entry(&mut self, entry: TreeEntry) {
        self.entries.push(entry);
    }
    #[instrument]
    fn into_entry(&self) -> TreeEntry {
        TreeEntry::Tree(self.name.clone(), self.hash())
    }
    #[instrument]
    fn get_entry(&self, name: &String) -> Option<TreeEntry> {
        for a in &self.entries {
            if a.name() == name {
                return Some(a.clone());
            }
        }
        None
    }
    #[instrument]
    fn get_entry_hash(&self, name: &String) -> Option<String> {
        self.get_entry(name).map(|a| hex::encode(a.hash()))
    }
    #[instrument]
    // encode Object to bincode bytes
    fn tree_bytes(&self) -> Vec<u8> {
        let tree_data = encode_to_vec(self, config::standard()).expect("Encoding failed");
        tree_data
    }
    #[instrument]
    // hash the tree object
    fn hash_str(&self) -> String {
        let hash = hash(&self.tree_bytes());
        let hash_hex = hash.to_hex().to_string();
        hash_hex
    }
    #[instrument]
    pub fn hash(&self) -> Vec<u8> {
        let hash = hash(&self.tree_bytes());
        // let hash_hex = hash;
        hash.as_bytes().to_vec()
    }
    #[instrument]
    // save the tree object to .gato/objects/<first 2 chars>/<rest chars>
    fn save(&self, storage: &LocalStorage) -> String {
        let tree_hash = self.hash_str();
        let tree_data = self.tree_bytes();
        match storage.put(&tree_hash, tree_data) {
            Ok(_) => {}
            Err(e) => {
                println!("{e}")
            }
        };
        tree_hash
    }
    #[instrument]
    // load tree object from .gato/objects/<first 2 chars>/<rest chars>
    pub fn load(hash: String, storage: &LocalStorage) -> GatoResult<Self> {
        let data = storage.get(&hash)?;
        let tree: Tree = bincode::decode_from_slice(&data, config::standard())?.0;
        Ok(tree)
    }
    // return hash of the root tree created from index
    #[instrument]
    pub fn create_from_index(index: Index, storage: &LocalStorage) -> (Vec<u8>, Vec<String>) {
        let mut file_dependencies = index.dependencies;
        let entries: Vec<(PathBuf, Vec<u8>)> = index
            .entries
            .into_iter()
            .map(|(path, entry)| (path, entry.hash))
            .collect();

        let root_tree_entry = Self::build_recursive_tree(
            entries,
            "root".to_string(),
            &mut file_dependencies,
            storage,
        );

        match root_tree_entry {
            TreeEntry::Tree(_, hash) => (hash, file_dependencies),
            _ => panic!("Root is not a tree!"),
        }
    }

    // recursively build tree from entries
    #[instrument]
    fn build_recursive_tree(
        entries: Vec<(PathBuf, Vec<u8>)>,
        name: String,
        dependencies: &mut Vec<String>,
        storage: &LocalStorage,
    ) -> TreeEntry {
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
            let subtree_entry =
                Self::build_recursive_tree(sub_entries, folder_name, dependencies, storage);
            current_tree.add_entry(subtree_entry);
        }

        let tree_hash = current_tree.save(storage);

        dependencies.push(tree_hash);

        current_tree.into_entry()
    }
    #[instrument]
    pub fn merge(
        base: Tree,
        current: Tree,
        target: Tree,
        deps: &mut Vec<String>,
        storage: &LocalStorage,
    ) -> GatoResult<Tree> {
        // println!("{base:?}");

        // println!("{current:?}");

        // println!("{target:?}");

        let mut result_tree = Tree::new(current.name.clone());
        let mut all_filenames = std::collections::HashSet::new();
        for e in &current.entries {
            all_filenames.insert(e.name());
        }
        for e in &target.entries {
            all_filenames.insert(e.name());
        }
        for e in &base.entries {
            all_filenames.insert(e.name());
        }
        for name in all_filenames {
            let b = base.get_entry_hash(&name);
            let c = current.get_entry_hash(&name);
            let t = target.get_entry_hash(&name);

            if c == t {
                if let Some(entry) = current.get_entry(&name) {
                    deps.push(hex::encode(entry.hash()));
                    result_tree.add_entry(entry.clone());
                }
            } else if c == b {
                if let Some(entry) = target.get_entry(&name) {
                    deps.push(hex::encode(entry.hash()));
                    result_tree.add_entry(entry.clone());
                }
            } else if t == b {
                if let Some(entry) = current.get_entry(&name) {
                    deps.push(hex::encode(entry.hash()));
                    result_tree.add_entry(entry.clone());
                }
            } else {
                match (current.get_entry(&name), target.get_entry(&name)) {
                    (Some(TreeEntry::Blob(_, hash1)), Some(TreeEntry::Blob(_, hash2))) => {
                        if let (Ok(current_file), Ok(target_file)) = (
                            storage.get_as_string(&hex::encode(hash1)),
                            storage.get_as_string(&hex::encode(hash2)),
                        ) {
                            let base_content = if let Some(base_hash) = b {
                                storage.get_as_string(&base_hash).unwrap_or(String::new())
                            } else {
                                String::new()
                            };

                            let merged = merge(&base_content, &current_file, &target_file);

                            match merged {
                                Ok(v) => {
                                    let hash = add_file_dry(v.as_bytes(), &storage)?;
                                    let entry = TreeEntry::Blob(name.clone(), hash);
                                    deps.push(hex::encode(entry.hash()));
                                    result_tree.add_entry(entry);
                                }
                                Err(conflict_content) => {
                                    println!("⚠️  CONFLICT detected in file: {}", name);
                                    let hash = add_file_dry(conflict_content.as_bytes(), &storage)?;
                                    let entry = TreeEntry::Blob(name.clone(), hash);
                                    deps.push(hex::encode(entry.hash()));
                                    result_tree.add_entry(entry);
                                }
                            }
                        } else {
                            return Err(Error::MergeConflict(format!(
                                "Binary file conflict: {}",
                                name
                            )));
                        }
                    }
                    (Some(TreeEntry::Tree(_, hash1)), Some(TreeEntry::Tree(_, hash2))) => {
                        let current_tree = Tree::load(hex::encode(hash1), &storage)?;
                        let target_tree = Tree::load(hex::encode(hash2), &storage)?;
                        let base_tree =
                            if let Some(TreeEntry::Tree(_, hash_base)) = base.get_entry(&name) {
                                Tree::load(hex::encode(hash_base), storage)?
                            } else {
                                Tree::new(name.clone())
                            };
                        let merged_subtree =
                            Self::merge(base_tree, current_tree, target_tree, deps, storage)?;
                        deps.push(hex::encode(merged_subtree.into_entry().hash()));
                        result_tree.add_entry(merged_subtree.into_entry());
                    }
                    _ => {
                        return Err(Error::MergeConflict(format!(
                            "{} renamed to file or directory",
                            name
                        )));
                    }
                }
            }
        }
        result_tree.save(&storage);
        Ok(result_tree)
    }
}
