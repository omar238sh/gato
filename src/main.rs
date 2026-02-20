use std::{path::PathBuf, sync::OnceLock};

use clap::Parser;
use colored::Colorize;
use directories::ProjectDirs;
mod core;
use crate::core::{
    cli::{
        cli::{Cli, Commands},
        init,
    },
    commit::Commit,
    error::GatoResult,
    storage::{StorageEngine, gc::Gc, local::LocalStorage},
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

fn run() -> GatoResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init(cli.path),
        Commands::Add { paths } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.add_paths(paths)?;
        }
        Commands::Commit { message } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.commit(message)?
        }
        Commands::Checkout { commit_index } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.check_out(commit_index)?
        }
        Commands::NewBranch { branch_name } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.new_branch(branch_name)?;
        }
        Commands::ChangeBranch { branch_name } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.change_branch(branch_name)?;
        }
        Commands::SoftReset { commit_index } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.soft_reset(commit_index);
        }
        Commands::Gc => {
            let storage = LocalStorage::tmp(get_store_path().clone());
            storage.gc()?;
        }
        Commands::ListRepos => {
            let storage = LocalStorage::tmp(get_store_path().clone());
            let repos_path = storage.list_repos()?;
            for repo in repos_path {
                println!("{:?}", repo);
            }
        }
        Commands::DeleteRepo => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.delete_repo()?;
            println!("you may need to run `gato gc`.");
        }
        Commands::DeleteBranch { name } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.delete_branch(name)?;
            println!("you may need to run `gato gc`.");
        }
        Commands::Status => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.status()?;
        }
        Commands::Merge {
            target_branch,
            message,
        } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            storage.merge(target_branch, message)?;
        }
        Commands::VerifyCommit { commit_hash } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            let commit = Commit::load(commit_hash, &storage);
            let result = commit.verify_commit(&storage)?;
            if result {
                println!("{}", "the integrity of the commit is OK!".green());
            } else {
                println!("{}", "some files deleted".red());
            }
        }
        Commands::ListCommits => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            let commits = Gc::list_repo_commits(&storage)?;
            for commit in commits {
                println!(
                    "message : {} \nhash : {}\n\n",
                    commit.message().green(),
                    commit.hash()?.bright_yellow()
                );
            }
        }
        Commands::Mount { mount_point } => {
            let storage = LocalStorage::load_from(get_store_path().clone(), cli.path.clone())?;
            let root_tree = storage.get_last_tree()?;
            let fs = core::vfs::GatoFS::new(root_tree, storage);
            fuser::mount2(fs, mount_point, &[])?;
        }
    };
    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    match run() {
        Ok(_) => {}
        Err(e) => println!("{}: {}", "Error".red().bold(), e),
    }
}
