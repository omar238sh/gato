use std::path::PathBuf;

use colored::Colorize;

use crate::core::{add::get_dry_hash, error::GatoResult, storage::local::LocalStorage};

pub enum FileStatus {
    ToBeCommited { path: PathBuf },
    NotStagedForCommit { path: PathBuf },
    UntrackedFiles { path: PathBuf },
    Unmodified,
}

impl FileStatus {
    pub fn from(
        path: PathBuf,
        deps: &Vec<String>,
        index_hash: Option<String>,
        storage: &LocalStorage,
    ) -> GatoResult<Self> {
        let hash_now = get_dry_hash(&path, storage)?;
        match index_hash {
            Some(v) => {
                if deps.contains(&hash_now) {
                    return Ok(Self::Unmodified);
                } else if v == hash_now {
                    return Ok(Self::ToBeCommited { path });
                } else {
                    return Ok(Self::NotStagedForCommit { path });
                }
            }
            None => {
                return Ok(Self::UntrackedFiles { path });
            }
        }
    }
}

impl std::fmt::Display for FileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileStatus::ToBeCommited { path } => {
                write!(
                    f,
                    "\t{}:   {}",
                    "to be commited".green(),
                    path.display().to_string().green()
                )
            }
            FileStatus::NotStagedForCommit { path } => {
                write!(
                    f,
                    "\t{}:   {}",
                    "modified".yellow(),
                    path.display().to_string().yellow()
                )
            }
            FileStatus::UntrackedFiles { path } => {
                write!(f, "\t{}", path.display().to_string().red())
            }
            FileStatus::Unmodified => {
                write!(f, "")
            }
        }
    }
}
