use std::{
    hash,
    sync::{Arc, Mutex, RwLock},
    time::SystemTime,
};

use fuser::{FileAttr, FileType};

use crate::core::{
    commit::{Tree, TreeEntry, blob::Blob},
    storage::{self, local::LocalStorage},
    vfs::error::{VFSError, VFSResult},
};

#[derive(Clone)]
pub struct TreeNode {
    pub entry: TreeEntry,
    pub inode: u64,
    pub parent: u64,
}

impl TreeNode {
    pub fn new(inode: u64, parent: u64, entry: TreeEntry) -> Self {
        Self {
            entry,
            inode,
            parent,
        }
    }

    pub fn is_file(&self) -> bool {
        matches!(self.entry, TreeEntry::Blob(_, _))
    }

    /// return old entry
    pub fn replace_entry(&mut self, entry: TreeEntry) -> TreeEntry {
        let old_entry = self.entry.clone();
        self.entry = entry;
        old_entry
    }

    pub fn update(
        &mut self,
        nodes: &mut TreeNodes,
        new_entry: TreeEntry,
        storage: &LocalStorage,
    ) -> VFSResult<()> {
        let parent_arc = nodes.get_node(self.parent)?;

        let mut parent = parent_arc.write().map_err(|_| VFSError::LockPoisoned)?;

        match &mut self.entry {
            TreeEntry::Blob(_, _) => {
                self.replace_entry(new_entry.clone());
                nodes.replace_node(self.clone())?;
                parent.update(nodes, new_entry, storage)?;
            }
            TreeEntry::Tree(name, items) => {
                let mut tree: Tree = Tree::load(hex::encode(items), storage)
                    .map_err(|_| VFSError::TreeNotFound(name.clone()))?;
                // replace here mean it's replace the hash of the same tree name in the tree
                tree.replace(&new_entry);
                // this mean i will save the tree to the store
                tree.save(&storage);
                self.replace_entry(tree.into_entry());
                nodes.replace_node(self.clone())?;
                if self.inode != self.parent {
                    parent.update(nodes, tree.into_entry(), storage)?;
                }
            }
        }
        Ok(())
    }

    fn get_size(&self, storage: &LocalStorage) -> u64 {
        match &self.entry {
            TreeEntry::Blob(_, hash) => {
                let hash = hex::encode(hash);
                if let Ok(data) = Blob::new(hash, storage) {
                    if let Ok(file) = data.restore_data() {
                        return file.len() as u64;
                    }
                }
                50
            }
            TreeEntry::Tree(_, _) => 4096,
        }
    }

    pub fn make_attr(&self, storage: &LocalStorage) -> FileAttr {
        let now = SystemTime::now();

        let kind = match self.entry {
            TreeEntry::Blob(_, _) => FileType::RegularFile,
            TreeEntry::Tree(_, _) => FileType::Directory,
        };

        FileAttr {
            ino: self.inode,
            size: self.get_size(storage),
            blocks: 1,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: kind,
            perm: if kind == FileType::Directory {
                0o755
            } else {
                0o644
            },
            nlink: if kind == FileType::Directory { 2 } else { 1 },
            uid: 501,
            gid: 20,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}

pub struct TreeNodes {
    data: RwLock<Vec<Arc<RwLock<TreeNode>>>>,
}

impl TreeNodes {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(Vec::new()),
        }
    }
    /// this method return error when a thread panic while use the TreeNodes
    pub fn add_entry(&self, entry: TreeNode) -> VFSResult<()> {
        let mut write = self.data.write().map_err(|_| VFSError::LockPoisoned)?;
        write.push(Arc::new(RwLock::new(entry)));
        Ok(())
    }

    pub fn find_with_name(&self, parent: u64, name: &String) -> VFSResult<Arc<RwLock<TreeNode>>> {
        let read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;

        for i in read.iter() {
            let node_read = i.read().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.entry.name() == name && node_read.parent == parent {
                return Ok(i.clone());
            }
        }

        Err(VFSError::NodeNotLoaded)
    }

    pub fn get_file_attr_with_name(
        &self,
        parent: u64,
        name: &String,
        storage: &LocalStorage,
    ) -> VFSResult<FileAttr> {
        let read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;

        for i in read.iter() {
            let node_read = i.read().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.entry.name() == name && node_read.parent == parent {
                return Ok(node_read.make_attr(storage));
            }
        }

        Err(VFSError::NodeNotLoaded)
    }

    pub fn get_node(&self, inode: u64) -> VFSResult<Arc<RwLock<TreeNode>>> {
        let nodes_read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;
        for e in nodes_read.iter() {
            let node_read = e.read().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.inode == inode {
                return Ok(e.clone());
            }
        }
        Err(VFSError::NodeNotLoaded)
    }

    pub fn get_node_attr(&self, inode: u64, storage: &LocalStorage) -> VFSResult<FileAttr> {
        let nodes_read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;
        for e in nodes_read.iter() {
            let node_read = e.read().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.inode == inode {
                return Ok(node_read.make_attr(storage));
            }
        }
        Err(VFSError::NodeNotLoaded)
    }

    pub fn replace_node(&mut self, new_node: TreeNode) -> VFSResult<()> {
        let mut nodes_read = self.data.write().map_err(|_| VFSError::LockPoisoned)?;
        for e in nodes_read.iter_mut() {
            let e2 = e.clone();
            let node_read = e2.read().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.inode == new_node.inode {
                *e = Arc::new(RwLock::new(new_node));
                return Ok(());
            }
        }
        Err(VFSError::NodeNotLoaded)
    }

    pub fn add_entries(&self, new_nodes: impl Iterator<Item = TreeNode>) -> VFSResult<()> {
        let mut write = self.data.write().map_err(|_| VFSError::LockPoisoned)?;
        new_nodes.for_each(|a| write.push(Arc::new(RwLock::new(a))));
        Ok(())
    }

    pub fn get_by_parent(&self, parent: u64) -> VFSResult<Vec<Arc<RwLock<TreeNode>>>> {
        let read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;
        let mut result = Vec::new();
        for i in read.iter() {
            let node_read = i.read().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.parent == parent {
                result.push(i.clone());
            }
        }
        Ok(result)
    }

    pub fn get_by_parent_fuser(&self, parent: u64) -> VFSResult<Vec<(u64, FileType, String)>> {
        let read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;
        let mut result = Vec::new();
        for i in read.iter() {
            let node_read = i.read().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.parent == parent {
                let a = if node_read.is_file() {
                    FileType::RegularFile
                } else {
                    FileType::Directory
                };
                result.push((node_read.inode, a, node_read.entry.name().clone()));
            }
        }
        Ok(result)
    }
}
