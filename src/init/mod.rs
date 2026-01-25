use std::{fs, path::PathBuf};

use crate::{
    init::lib::new_id,
    storage::{StorageEngine, StorageError, local::LocalStorage},
};
mod error;
pub mod lib;
pub fn create_file_layout(global_path: PathBuf, repo_path: PathBuf) -> Result<(), StorageError> {
    let id = new_id();
    let pointer = repo_path.join(".gato");
    let config = include_str!("gato.toml");
    let config_path = repo_path.join("gato.toml");
    fs::write(config_path, config)?;
    fs::write(pointer, &id)?;
    let storage = LocalStorage::new(global_path, id);
    storage.setup()?;
    Ok(())
}

pub fn new_branch(branch_name: String, storage: &impl StorageEngine) {
    match storage.new_branch(branch_name) {
        Err(e) => println!("{e}"),
        _ => {}
    }
}

pub fn change_branch(branch_name: String, storage: &impl StorageEngine) {
    match storage.change_branch(branch_name) {
        Err(e) => println!("{e}"),
        _ => {}
    }
}
