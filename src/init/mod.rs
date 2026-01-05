use std::{fs, io::Write, path::PathBuf};

use crate::commit::Commit;

pub fn create_file_layout() {
    let root_dir = PathBuf::from(".gato");

    fs::create_dir(&root_dir).expect("Failed to create .gato directory");

    let folders_paths = ["objects", "refs/heads", "refs/tags"];
    for folder in folders_paths {
        fs::create_dir_all(root_dir.join(folder)).expect("Failed to create folder");
    }

    fs::write(root_dir.join("HEAD"), "master").expect("Failed to create HEAD file");

    let config_content = r#"title = "My App Config"

[compression]
method = "Zstd"
level = 1"#;

    fs::write(root_dir.join("config.toml"), config_content).expect("Failed to create config.toml");

    fs::File::create(root_dir.join("index")).expect("Failed to create index file");
}

pub fn new_branch(branch_name: &str) {
    let commit_hash = Commit::get_parent_hash().expect("Failed to get current commit hash");
    let branch_path = PathBuf::from(".gato")
        .join("refs")
        .join("heads")
        .join(branch_name);
    let mut file = std::fs::File::create(&branch_path).expect("Failed to create new branch file");
    std::fs::File::write(&mut file, &commit_hash).expect("Failed to create new branch");
    std::fs::write(PathBuf::from(".gato").join("HEAD"), branch_name)
        .expect("Failed to write to HEAD file");
}

pub fn change_branch(branch_name: &str) {
    std::fs::write(PathBuf::from(".gato").join("HEAD"), branch_name)
        .expect("Failed to write to HEAD file");
}
