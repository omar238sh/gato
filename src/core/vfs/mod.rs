use std::sync::{Arc, RwLock, atomic::AtomicU64};
mod error;
mod models;
use fuser::{FileType, Filesystem};

use crate::core::{
    commit::{Tree, blob::Blob},
    storage::local::LocalStorage,
    vfs::{
        error::{VFSError, VFSResult},
        models::{TreeNode, TreeNodes},
    },
};

pub struct GatoFS {
    root_tree: Arc<RwLock<Tree>>,
    inodes: TreeNodes,
    next: AtomicU64,
    loaded: Vec<u64>,
    storage: LocalStorage,
}

impl GatoFS {
    pub fn new(root_tree: Tree, storage: LocalStorage) -> Self {
        let mut root_entry = root_tree.into_entry();
        root_entry.change_name(".".to_string());
        let root_node = TreeNode::new(1, 1, root_entry);
        let inodes = TreeNodes::new();
        inodes.add_entry(root_node).unwrap();
        Self {
            root_tree: Arc::new(RwLock::new(root_tree)),
            inodes: inodes,
            next: AtomicU64::new(2),
            storage: storage,
            loaded: Vec::new(),
        }
    }

    pub fn next_inode(&self) -> u64 {
        self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn load(&mut self, inode: u64) -> VFSResult<()> {
        if self.loaded.contains(&inode) {
            return Ok(());
        }
        let node = self.inodes.get_node(inode)?;
        let node = node.read().map_err(|_| VFSError::LockPoisoned)?;
        let tree = node.entry.clone();
        let tree = Tree::load(hex::encode(tree.hash()), &self.storage)
            .map_err(|_| VFSError::TreeNotFound(tree.name().clone()))?;
        let nodes = tree.entries.into_iter().map(|a| {
            let node = TreeNode::new(self.next_inode(), inode, a);
            node
        });
        self.inodes.add_entries(nodes)?;
        self.loaded.push(inode);
        Ok(())
    }

    fn do_read(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
    ) -> VFSResult<Vec<u8>> {
        let node = self.inodes.get_node(ino)?;
        let read = node.read().map_err(|_| VFSError::LockPoisoned)?;
        if !read.is_file() {
            return Err(VFSError::NotAFile);
        }
        let hash = hex::encode(read.entry.hash());
        let blob =
            Blob::new(hash, &self.storage).map_err(|e| VFSError::GatoError(e.to_string()))?;
        let data = blob
            .restore_data()
            .map_err(|e| VFSError::GatoError(e.to_string()))?;
        let len = data.len();
        let start = std::cmp::min(offset as usize, len);
        let end = std::cmp::min(start + size as usize, len);

        return Ok(data[start..end].to_vec());
    }
}

impl Filesystem for GatoFS {
    fn destroy(&mut self) {}
    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        match self.inodes.get_file_attr_with_name(
            parent,
            &name.to_os_string().into_string().unwrap_or(String::new()),
            &self.storage,
        ) {
            Ok(v) => {
                reply.entry(&std::time::Duration::from_secs(1), &v, 0);
            }
            Err(_) => {
                reply.error(libc::ENOENT);
            }
        }
    }

    fn getattr(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: Option<u64>,
        reply: fuser::ReplyAttr,
    ) {
        match self.inodes.get_node_attr(ino, &self.storage) {
            Ok(attr) => {
                reply.attr(&std::time::Duration::from_secs(1), &attr);
            }
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        let mut parent_ino: u64 = 1;
        if let Ok(()) = self.load(ino) {}
        if let Ok(node) = self.inodes.get_node(ino) {
            if let Ok(n) = node.read() {
                parent_ino = n.parent;
            }
        }
        let mut entries = Vec::new();
        if ino != 1 {
            entries.push((ino, FileType::Directory, ".".to_string()))
        };
        entries.push((parent_ino, FileType::Directory, "..".to_string()));
        if let Ok(v) = self.inodes.get_by_parent_fuser(ino) {
            entries.extend(v);
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            let (entry_ino, entry_type, name) = entry;

            if reply.add(entry_ino, (i + 1) as i64, entry_type, name) {
                break;
            }
        }

        reply.ok();
    }

    fn open(&mut self, _req: &fuser::Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        match self.inodes.get_node(ino) {
            Ok(node) => {
                let node = node.read().map_err(|_| VFSError::LockPoisoned).unwrap();
                if node.is_file() {
                    reply.opened(0, 0);
                } else {
                    reply.error(libc::EISDIR);
                }
            }
            Err(_) => reply.error(libc::ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        match self.do_read(_req, ino, fh, offset, size, flags, lock_owner) {
            Ok(v) => {
                reply.data(v.as_slice());
            }
            Err(_) => {
                reply.error(libc::EIO);
            }
        }
    }
}
