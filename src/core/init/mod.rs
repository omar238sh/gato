use std::fs::{self};

use crate::core::{
    error::GatoResult,
    storage::{StorageEngine, local::LocalStorage},
};

pub mod lib;
pub fn create_file_layout(storage: &LocalStorage) -> GatoResult<()> {
    let id = storage.repo_id();
    let work_dir = storage.work_dir();
    let config = include_str!("config.toml").replace("<repo_id>", &id);
    let config_path = work_dir.join("gato.toml");
    fs::write(config_path, config)?;
    storage.push_to_repos()?;
    storage.setup()?;
    Ok(())
}
