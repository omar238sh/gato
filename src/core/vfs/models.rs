use std::{
    sync::{Arc, RwLock},
    time::SystemTime,
};

use fuser::{FileAttr, FileType};

use crate::core::{
    add::add_file_dry,
    commit::{Tree, TreeEntry, blob::Blob},
    storage::local::LocalStorage,
    vfs::error::{VFSError, VFSResult},
};

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub entry: TreeEntry,
    pub inode: u64,
    pub parent: u64,
    pub data: Arc<RwLock<Option<Vec<u8>>>>,
    pub loaded: bool,
    pub len: u64,
}

impl TreeNode {
    pub fn new(inode: u64, parent: u64, entry: TreeEntry) -> Self {
        Self {
            entry,
            inode,
            parent,
            data: Arc::new(RwLock::new(None)),
            loaded: false,
            len: 0,
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

    pub fn get_parents(&self, inodes: &TreeNodes) -> VFSResult<Vec<Arc<RwLock<Self>>>> {
        let mut parents = Vec::new();
        let mut current_parent = self.parent;
        let mut flag = false;
        loop {
            if current_parent == 1 {
                flag = true;
            }
            if let Ok(node) = inodes.get_node(current_parent) {
                parents.push(node.clone());
                let node_read = node.read().map_err(|_| VFSError::LockPoisoned)?;
                current_parent = node_read.parent;
            } else {
                break;
            }
            if flag {
                break;
            }
        }
        Ok(parents)
    }

    pub fn write(
        &mut self,
        storage: &LocalStorage,
        offset: usize,
        data: &[u8],
        nodes: &mut TreeNodes,
    ) -> VFSResult<()> {
        if !self.loaded {
            self.load(storage);
        }

        let end = offset + data.len();
        if end > self.len as usize {
            let mut write = self.data.write().map_err(|_| VFSError::LockPoisoned)?;
            if let Some(v) = write.as_mut() {
                v.resize(end, 0);
                v[offset..end].copy_from_slice(data);
                let hash = add_file_dry(v.as_slice(), storage)
                    .map_err(|a| VFSError::GatoError(a.to_string()))?;
                self.entry.change_hash(hash);
            }
            drop(write);
            let mut parents = self.get_parents(nodes)?;

            self.update(nodes, self.entry.clone(), storage, &mut parents)?;
            self.len = end as u64;
        }
        Ok(())
    }

    pub fn update(
        &mut self,
        nodes: &mut TreeNodes,
        new_entry: TreeEntry,
        storage: &LocalStorage,
        parents: &mut Vec<Arc<RwLock<Self>>>,
    ) -> VFSResult<()> {
        println!("update run ");
        // let parents = self.get_parents(nodes)?;
        if parents.len() != 0 {
            let parent_arc = parents.remove(0);
            match &mut self.entry {
                TreeEntry::Blob(_, _) => {
                    self.replace_entry(new_entry.clone());

                    let mut parent = parent_arc.write().map_err(|_| VFSError::LockPoisoned)?;
                    parent.update(nodes, new_entry, storage, parents)?;
                }
                TreeEntry::Tree(name, items) => {
                    let mut tree: Tree = Tree::load(hex::encode(items), storage)
                        .map_err(|_| VFSError::TreeNotFound(name.clone()))?;
                    // replace here mean it's replace the hash of the same tree name in the tree
                    tree.replace(&new_entry);
                    // this mean i will save the tree to the store
                    tree.save(&storage);
                    self.replace_entry(tree.into_entry());

                    if self.inode != self.parent {
                        let mut parent = parent_arc.write().map_err(|_| VFSError::LockPoisoned)?;
                        parent.update(nodes, tree.into_entry(), storage, parents)?;
                    }
                }
            }
        } else {
            return Ok(());
        };

        Ok(())
    }

    pub fn load(&mut self, storage: &LocalStorage) {
        if !self.loaded {
            match &self.entry {
                TreeEntry::Blob(_, hash) => {
                    let hash = hex::encode(hash);
                    if let Ok(data) = Blob::new(hash, storage) {
                        if let Ok(file) = data.restore_data(storage) {
                            self.len = file.len() as u64;
                            self.data = Arc::new(RwLock::new(Some(file)));
                            self.loaded = true;
                        }
                    }
                }
                TreeEntry::Tree(_, _) => {}
            }
        }
    }

    fn get_size(&mut self, storage: &LocalStorage) -> u64 {
        self.load(storage);
        match &self.entry {
            TreeEntry::Blob(_, _) => self.len,
            TreeEntry::Tree(_, _) => 4096,
        }
    }

    pub fn make_attr(&mut self, storage: &LocalStorage) -> FileAttr {
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

    // pub fn find_with_name(&self, parent: u64, name: &String) -> VFSResult<Arc<RwLock<TreeNode>>> {
    //     let read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;

    //     for i in read.iter() {
    //         let node_read = i.read().map_err(|_| VFSError::LockPoisoned)?;
    //         if node_read.entry.name() == name && node_read.parent == parent {
    //             return Ok(i.clone());
    //         }
    //     }

    //     Err(VFSError::NodeNotLoaded)
    // }

    pub fn delete(&self, inode: u64) -> VFSResult<()> {
        let mut write = self.data.write().map_err(|_| VFSError::LockPoisoned)?;
        write.retain_mut(|x| {
            let e = x.write().map_err(|_| VFSError::LockPoisoned);
            match e {
                Ok(v) => v.inode != inode || v.inode == 1,
                Err(_) => false,
            }
        });

        Ok(())
    }

    pub fn get_file_attr_with_name(
        &self,
        parent: u64,
        name: &String,
        storage: &LocalStorage,
    ) -> VFSResult<FileAttr> {
        let read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;

        for i in read.iter() {
            let mut node_read = i.write().map_err(|_| VFSError::LockPoisoned)?;
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

    // pub fn get_all_parents(&self, inodes: Vec<u64>) -> VFSResult<Vec<u64>> {
    //     let mut parents = Vec::new();
    //     let nodes_read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;
    //     for e in nodes_read.iter() {
    //         let node_read = e.read().map_err(|_| VFSError::LockPoisoned)?;
    //         if inodes.contains(&node_read.inode) {
    //             parents.push(node_read.parent);
    //         }
    //     }
    //     Ok(parents)
    // }

    pub fn get_node_attr(&self, inode: u64, storage: &LocalStorage) -> VFSResult<FileAttr> {
        let nodes_read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;
        for e in nodes_read.iter() {
            let mut node_read = e.write().map_err(|_| VFSError::LockPoisoned)?;
            if node_read.inode == inode {
                return Ok(node_read.make_attr(storage));
            }
        }
        Err(VFSError::NodeNotLoaded)
    }

    // pub fn replace_node(&mut self, new_node: TreeNode) -> VFSResult<()> {
    //     let mut nodes_write = self.data.write().map_err(|_| VFSError::LockPoisoned)?;
    //     for e in nodes_write.iter_mut() {
    //         let e2 = e.clone();
    //         let node_read = e2.read().map_err(|_| VFSError::LockPoisoned)?;
    //         if node_read.inode == new_node.inode {
    //             *e = Arc::new(RwLock::new(new_node));
    //             drop(node_read);
    //             return Ok(());
    //         }
    //     }
    //     Err(VFSError::NodeNotLoaded)
    // }

    pub fn add_entries(&self, new_nodes: impl Iterator<Item = TreeNode>) -> VFSResult<()> {
        let mut write = self.data.write().map_err(|_| VFSError::LockPoisoned)?;
        new_nodes.for_each(|a| write.push(Arc::new(RwLock::new(a))));
        Ok(())
    }

    // pub fn get_by_parent(&self, parent: u64) -> VFSResult<Vec<Arc<RwLock<TreeNode>>>> {
    //     let read = self.data.read().map_err(|_| VFSError::LockPoisoned)?;
    //     let mut result = Vec::new();
    //     for i in read.iter() {
    //         let node_read = i.read().map_err(|_| VFSError::LockPoisoned)?;
    //         if node_read.parent == parent {
    //             result.push(i.clone());
    //         }
    //     }
    //     Ok(result)
    // }

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
