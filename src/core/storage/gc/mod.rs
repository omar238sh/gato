use crate::core::{
    commit::Commit,
    error::{Error, GatoResult},
    storage::local::LocalStorage,
};

pub struct Gc {
    storages: Vec<LocalStorage>,
}

impl Gc {
    pub fn new(storages: Vec<LocalStorage>) -> Self {
        Self { storages }
    }

    fn list_repo_commits(storage: &LocalStorage) -> GatoResult<Vec<Commit>> {
        let branchs = storage.list_branchs().map_err(|_| Error::GcError)?;
        let mut all_commits = Vec::new();
        for branch in branchs {
            let last_commit_hash =
                hex::encode(storage.read_ref_vec(branch).map_err(|_| Error::GcError)?);
            let mut last_commit = Commit::load(last_commit_hash, &storage);
            let mut commits = vec![last_commit.clone()];

            while let Some(older_hash) = last_commit.parent_hash() {
                last_commit = Commit::load(older_hash, &storage);
                commits.push(last_commit.clone());
            }

            all_commits.extend(commits);
        }
        Ok(all_commits)
    }

    fn list_commits_hashs(storage: &LocalStorage) -> GatoResult<Vec<String>> {
        let branchs = storage.list_branchs().map_err(|_| Error::GcError)?;
        let mut all_hashs = Vec::new();
        for branch in branchs {
            let last_commit_hash = hex::encode(storage.read_ref_vec(branch)?);

            let mut hashes = vec![last_commit_hash.clone()];

            let mut last_commit = Commit::load(last_commit_hash, &storage);

            while let Some(older_hash) = last_commit.parent_hash() {
                hashes.push(older_hash.clone());

                last_commit = Commit::load(older_hash, &storage);
            }

            all_hashs.extend(hashes);
        }
        Ok(all_hashs)
    }

    pub fn repo_dependices(storage: &LocalStorage) -> GatoResult<Vec<String>> {
        let mut dependices = Self::list_commits_hashs(storage)?;
        let commits = Self::list_repo_commits(storage)?;
        for commit in commits {
            dependices.append(&mut commit.dependices());
        }

        Ok(dependices)
    }

    pub fn global_dependices(&self) -> GatoResult<Vec<String>> {
        let mut linked_files = Vec::new();
        for storage in &self.storages {
            let dependices = Self::repo_dependices(&storage)?;
            linked_files.extend(dependices);
        }
        Ok(linked_files)
    }
}
