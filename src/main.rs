use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use clap::Parser;

use directories::ProjectDirs;
mod add;
use crate::{
    add::{add_all, find_files},
    cli::{Cli, Commands, api::init},
    commit::Commit,
    init::{change_branch, new_branch},
    storage::{StorageEngine, local::LocalStorage},
};
mod cli;
mod commit;
mod config;
mod init;
mod storage;
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init(cli.path),

        Commands::Add { paths } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), &cli.path)
                .expect("run `gato init` first");
            add_paths(paths, storage);
        }

        Commands::Commit { message } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), &cli.path)
                .expect("run `gato init` first");
            let commit = Commit::new(message, &storage);
            if let Err(e) = commit.save(&storage) {
                eprintln!("commit failed: {e}");
            }
        }

        Commands::Checkout { commit_index } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), &cli.path)
                .expect("run `gato init` first");
            if let Some(commit) = Commit::load_by_index(commit_index, &storage) {
                if let Err(e) = commit.write_tree(&cli.path, &storage) {
                    eprintln!("checkout failed: {e}");
                }
            } else {
                eprintln!("unknown commit index {commit_index}");
            }
        }

        Commands::NewBranch { branch_name } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), &cli.path)
                .expect("run `gato init` first");
            new_branch(branch_name, &storage);
        }

        Commands::ChangeBranch { branch_name } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), &cli.path)
                .expect("run `gato init` first");
            change_branch(branch_name, &storage);
        }

        Commands::SoftReset { commit_index } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), &cli.path)
                .expect("run `gato init` first");
            if let Some(hash) = Commit::get_hash_from_index(commit_index, &storage) {
                if let Ok(bytes) = hex::decode(hash) {
                    if let Err(e) = storage.write_ref(storage.get_active_branche(), bytes) {
                        eprintln!("reset failed: {e}");
                    }
                }
            }
        }
    }
}

fn add_paths(paths: Vec<String>, storage: LocalStorage) {
    let mut all_files: Vec<PathBuf> = Vec::new();

    for path in paths {
        let path_obj = Path::new(&path);
        if path_obj.is_dir() {
            let mut files = find_files(path_obj).unwrap();
            all_files.append(&mut files);
        } else {
            all_files.push(path_obj.to_path_buf());
        }
    }
    match add_all(all_files, Arc::new(storage)) {
        Ok(()) => {}
        Err(e) => println!("{}", e),
    }
}
