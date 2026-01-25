use std::{fs, path::PathBuf};

use crate::storage::{StorageEngine, StorageError};

pub struct LocalStorage {
    root_path: PathBuf,
    repo_id: String,
}

impl LocalStorage {
    pub fn new(path: PathBuf, repo_id: String) -> Self {
        Self {
            root_path: path,
            repo_id: repo_id,
        }
    }

    pub fn objects_path(&self, hash: &String) -> PathBuf {
        self.root_path
            .join("objects")
            .join(&hash[..2])
            .join(&hash[2..])
    }

    pub fn get_active_branche(&self) -> String {
        let head_path = self.root_path.join(self.repo_id.to_owned()).join("HEAD");
        let branche = fs::read_to_string(head_path).unwrap_or(String::from("master"));
        branche
    }

    pub fn get_branch_path(&self, ref_name: String) -> PathBuf {
        self.root_path
            .join(&self.repo_id)
            .join("refs")
            .join("heads")
            .join(ref_name)
    }

    // pub fn active_branche_path(&self) -> PathBuf {
    //     self.root_path
    //         .join(&self.repo_id)
    //         .join("refs")
    //         .join("heads")
    //         .join(self.get_active_branche())
    // }

    pub fn load_from(store_path: PathBuf, work_dir: &PathBuf) -> Result<Self, StorageError> {
        let id_file_path = work_dir.join(".gato");
        let repo_id = std::fs::read_to_string(id_file_path)
            .map(|content| content.trim().to_string())
            .map_err(|_| StorageError::UninitializedRepository)?;

        Ok(Self::new(store_path, repo_id))
    }

    pub fn repo_path(&self) -> PathBuf {
        self.root_path.join(&self.repo_id)
    }

    pub fn read_ref_vec(&self, ref_name: String) -> Result<Vec<u8>, StorageError> {
        let ref_path = self.get_branch_path(ref_name);
        fs::read(ref_path).map_err(|_| StorageError::ReadError)
    }
}

impl StorageEngine for LocalStorage {
    fn get(&self, hash: &String) -> Result<Vec<u8>, super::StorageError> {
        let object_path = self.objects_path(hash);
        let data = fs::read(object_path).map_err(|_| StorageError::ReadError);
        data
    }

    fn put(&self, hash: &String, data: Vec<u8>) -> Result<(), super::StorageError> {
        if !self.exist(hash) {
            let object_path = self.objects_path(hash);

            if let Some(parent) = object_path.parent() {
                std::fs::create_dir_all(parent).map_err(|_| StorageError::WriteError)?;
            }

            fs::write(object_path, data).map_err(|_| StorageError::WriteError)?;
        }
        Ok(())
    }

    fn exist(&self, hash: &String) -> bool {
        self.objects_path(hash).exists()
    }

    fn write_ref(&self, ref_name: String, hash: Vec<u8>) -> Result<(), super::StorageError> {
        let ref_path = self.get_branch_path(ref_name);

        if let Some(parent) = ref_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| StorageError::WriteError)?;
        }

        fs::write(ref_path, hash).map_err(|_| StorageError::WriteError)
    }

    // fn read_ref(&self, ref_name: String) -> Result<String, super::StorageError> {
    //     self.read_ref_vec(ref_name).map(|a| hex::encode(a))
    // }

    fn setup(&self) -> Result<(), StorageError> {
        let heads_path = self.repo_path().join("refs").join("heads");
        std::fs::create_dir_all(heads_path).map_err(|_| StorageError::WriteError)?;
        Ok(())
    }

    fn new_branch(&self, name: String) -> Result<(), StorageError> {
        let branch_path = self.get_branch_path(name);
        if let Some(parent) = branch_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| StorageError::WriteError)?;
        }
        fs::write(branch_path, self.read_ref_vec(self.get_active_branche())?)
            .map_err(|_| StorageError::WriteError)
    }

    fn change_branch(&self, name: String) -> Result<(), StorageError> {
        fs::write(self.repo_path().join("HEAD"), name)?;
        Ok(())
    }
}
