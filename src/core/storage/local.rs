use std::{fs, path::PathBuf, sync::Arc};

use bincode::encode_to_vec;

use crate::core::{
    add::{add_all, find_files, index::Index},
    cli::get_store_path,
    commit::Commit,
    config::load::load_config,
    error::{Error, GatoResult},
    storage::{StorageEngine, StorageError, gc::Gc},
};

#[derive(Clone)]
pub struct LocalStorage {
    pub root_path: PathBuf,
    repo_id: String,
    work_dir: PathBuf,
}

impl LocalStorage {
    pub fn new(global_path: PathBuf, repo_id: String, path: PathBuf) -> Self {
        Self {
            root_path: global_path,
            repo_id: repo_id,
            work_dir: path,
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

    pub fn tmp(store_path: PathBuf) -> Self {
        Self::new(store_path, "".to_string(), PathBuf::new())
    }

    pub fn load_from(store_path: PathBuf, work_dir: PathBuf) -> GatoResult<Self> {
        let repo_id = load_config(&work_dir)?.id;
        Ok(Self::new(store_path, repo_id, work_dir))
    }

    pub fn repo_path(&self) -> PathBuf {
        self.root_path.join(&self.repo_id)
    }

    pub fn read_ref_vec(&self, ref_name: String) -> Result<Vec<u8>, StorageError> {
        let ref_path = self.get_branch_path(ref_name);
        fs::read(ref_path).map_err(|_| StorageError::ReadError)
    }

    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }

    pub fn add_paths(&self, paths: Vec<String>) -> GatoResult<()> {
        add_paths(paths, self)?;
        Ok(())
    }

    pub fn commit(&self, message: String) -> GatoResult<()> {
        let commit = Commit::new(message, &self)?;
        commit.save(&self)?;
        fs::remove_file(Index::index_file_path(&self))?;
        Ok(())
    }

    pub fn check_out(&self, commit_index: usize) -> GatoResult<()> {
        if let Some(commit) = Commit::load_by_index(commit_index, &self) {
            commit.write_tree(&self.work_dir(), &self)?;
        } else {
            eprintln!("unknown commit index {commit_index}");
        }
        Ok(())
    }

    pub fn soft_reset(&self, commit_index: usize) {
        if let Some(hash) = Commit::get_hash_from_index(commit_index, &self) {
            if let Ok(bytes) = hex::decode(hash) {
                if let Err(e) = self.write_ref(self.get_active_branche(), bytes) {
                    eprintln!("reset failed: {e}");
                }
            }
        }
    }

    pub fn repo_id(&self) -> &str {
        &self.repo_id
    }

    // pub fn init(path: PathBuf) {
    //     init(path)
    // }

    pub fn list_repos(&self) -> GatoResult<Vec<PathBuf>> {
        let repos_path = self.root_path.join("repos");
        let past_data = fs::read(repos_path);
        let data: Vec<PathBuf> = match past_data {
            Ok(v) => bincode::decode_from_slice(v.as_slice(), bincode::config::standard())?.0,
            Err(_) => Vec::new(),
        };
        Ok(data)
    }

    pub fn push_to_repos(&self) -> GatoResult<()> {
        let mut data = self.list_repos()?;
        data.push(self.work_dir().canonicalize()?.to_owned());
        fs::write(
            self.root_path.join("repos"),
            encode_to_vec(data, bincode::config::standard())?,
        )?;
        Ok(())
    }

    fn remove(&self, hash: &String) -> GatoResult<()> {
        let object_path = self.objects_path(hash);
        fs::remove_file(object_path)?;
        Ok(())
    }

    pub fn list_branchs(&self) -> GatoResult<Vec<String>> {
        let path = self
            .root_path
            .join(&self.repo_id)
            .join("refs")
            .join("heads");
        let mut branchs_names = Vec::new();
        let branchs = fs::read_dir(&path)?;

        for branch in branchs {
            if let Ok(name) = branch?.file_name().into_string() {
                branchs_names.push(name);
            }
        }

        Ok(branchs_names)
    }

    pub fn list_files(&self) -> GatoResult<Vec<String>> {
        let objects_dir = self.root_path.join("objects");
        let mut hashes: Vec<String> = Vec::new();
        let dirs = fs::read_dir(objects_dir)?;

        for dir in dirs {
            let dir = dir?;
            let prefix = dir.file_name();
            for file in fs::read_dir(dir.path())? {
                let file = file?;
                let rest = file.file_name();
                if let Ok(p) = prefix.clone().into_string()
                    && let Ok(r) = rest.into_string()
                {
                    hashes.push(format!("{}{}", p, r));
                }
            }
        }

        Ok(hashes)
    }

    pub fn gc(&self) -> GatoResult<()> {
        let repos: Vec<_> = self
            .list_repos()?
            .iter()
            .map(|repo| Self::load_from(get_store_path().clone(), repo.clone()))
            .map(|res| res.ok())
            .flatten()
            .collect();

        let gc = Gc::new(repos);
        let dependices = gc.global_dependices()?;
        let all_data = self.list_files()?;

        for a in all_data {
            if !dependices.contains(&a) {
                println!("removing file : {}", a);
                self.remove(&a)?;
            }
        }

        Ok(())
    }

    pub fn delete_repo(&self) -> GatoResult<()> {
        fs::remove_file(self.work_dir().join("gato.toml"))?;
        fs::remove_dir(self.repo_path())?;
        Ok(())
    }

    pub fn delete_branch(&self, name: String) -> GatoResult<()> {
        let active_branch = self.get_active_branche();

        if name == active_branch {
            return Err(Error::ActiveBranchDeletionError);
        } else {
            fs::remove_file(self.get_branch_path(name))?;
        }

        Ok(())
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

fn add_paths(paths: Vec<String>, storage: &LocalStorage) -> GatoResult<()> {
    let mut all_files: Vec<PathBuf> = Vec::new();

    for path in paths {
        let path_obj = &storage.work_dir().join(&path);
        if path_obj.is_dir() {
            let mut files = find_files(path_obj, storage)
                .unwrap()
                .iter()
                .map(|a| {
                    a.strip_prefix(storage.work_dir())
                        .unwrap_or(a)
                        .to_path_buf()
                })
                .collect();
            all_files.append(&mut files);
        } else {
            all_files.push(
                path_obj
                    .strip_prefix(storage.work_dir())
                    .unwrap_or(path_obj)
                    .to_path_buf(),
            );
        }
    }
    add_all(all_files, Arc::new(storage.clone()))?;
    Ok(())
}
