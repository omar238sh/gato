use std::path::{Path, PathBuf};

use clap::Parser;
mod add;
use crate::{
    add::{add_all, find_files},
    cli::Cli,
    commit::Commit,
    init::create_file_layout,
};
mod cli;
mod commit;
mod config;
mod init;
fn main() {
    let cli = Cli::parse();

    match cli.command {
        cli::Commands::Init => {
            // println!("Initializing a new Gato repository...");
            create_file_layout();
        }
        cli::Commands::Add { paths } => {
            add_paths(paths);
            // println!("[+] Files added successfully.");
        }
        cli::Commands::Commit { message } => {
            let result = Commit::new(message).save();
            match result {
                Ok(()) => {
                    std::fs::remove_file(PathBuf::from(".gato/index"))
                        .expect("cannot remove index file");
                }
                Err(e) => {
                    println!("{e}")
                }
            }
        }
        cli::Commands::Status => {
            // println!("Displaying status...");
            // Add status display logic here
        }
        cli::Commands::Log => {
            // println!("Displaying commit log...");
            // Add log display logic here
        }
        cli::Commands::Checkout { commit_index } => {
            let c = Commit::load_by_index(commit_index).expect("cannot load this index");
            println!("{}", c);
            let path = PathBuf::from(".");
            match c.write_tree(&path) {
                Ok(()) => {}
                Err(e) => println!("{}", e),
            }
            // Add checkout logic here
        }
        cli::Commands::NewBranch { branch_name } => {
            init::new_branch(&branch_name);
        }
        cli::Commands::ChangeBranch { branch_name } => {
            init::change_branch(&branch_name);
        }
    }
}

fn add_paths(paths: Vec<String>) {
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
    add_all(all_files).unwrap();
}
