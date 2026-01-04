use std::{io::Write, path::PathBuf};

use crate::commit::Commit;

pub fn create_file_layout() {
    // Function to create file layout
    let root_dir = ".gato";
    let files_paths = ["index", "config", "HEAD"];
    let folders_paths = ["objects", "refs/heads", "refs/tags"];
    std::fs::create_dir(&root_dir).expect("Failed to create .gato directory");
    for folder in folders_paths.iter() {
        std::fs::create_dir_all(format!("./{}/{}", root_dir, folder))
            .expect("Failed to create folder");
    }
    std::fs::write(PathBuf::from(root_dir).join(files_paths[2]), "master")
        .expect("Failed to create HEAD file");
    for i in 0..2 {
        std::fs::File::create(format!("./{}/{}", root_dir, files_paths[i]))
            .expect("Failed to create file");
    }
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
