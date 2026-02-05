use std::{path::PathBuf, sync::OnceLock};
pub mod cli;
use directories::ProjectDirs;

use crate::core::{
    config::load::load_config,
    init::{create_file_layout, lib::new_id},
    storage::local::LocalStorage,
};

static GLOBAL_STORE_PATH: OnceLock<PathBuf> = OnceLock::new();
pub fn get_store_path() -> &'static PathBuf {
    GLOBAL_STORE_PATH.get_or_init(|| {
        if let Some(proj_dirs) = ProjectDirs::from("com", "gatocloud", "gato") {
            proj_dirs.data_local_dir().to_path_buf()
        } else {
            PathBuf::from(".gato")
        }
    })
}

pub fn init(path: PathBuf) {
    let id = load_config(&path);
    match id {
        Ok(_) => {
            println!("Repo already initialized");
        }
        Err(_) => {
            let storage = LocalStorage::new(get_store_path().clone(), new_id(), path);
            match create_file_layout(&storage) {
                Ok(()) => println!("initialized successfuly"),
                Err(_) => {}
            }
        }
    }
}
