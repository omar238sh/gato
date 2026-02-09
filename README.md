#  Gato

### **A High-Performance, Parallelized Version Control System**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)

**Gato** is a modern, blazing-fast version control system built from scratch in **Rust**. It is engineered for developers who deal with massive datasets and demand a VCS that leverages multi-core architectures to provide near-instantaneous performance. Gato uses content-defined chunking, parallelized hashing and compression, and a content-addressable object store — all concepts inspired by Git but reimagined with performance-first design.

---

## Table of Contents

- [Key Features](#-key-features)
- [Architecture Overview](#-architecture-overview)
- [Project Structure](#-project-structure)
- [Dependencies (Cargo.toml)](#-dependencies-cargotoml)
- [How It Works — Module by Module](#-how-it-works--module-by-module)
  - [Entry Point (`src/main.rs`)](#1-entry-point-srcmainrs)
  - [CLI (`src/core/cli/`)](#2-cli-srccorecli)
  - [Configuration (`src/core/config/`)](#3-configuration-srccoreconfig)
  - [Initialization (`src/core/init/`)](#4-initialization-srccoreinit)
  - [Storage Engine (`src/core/storage/`)](#5-storage-engine-srccorestorage)
  - [Add & Staging (`src/core/add/`)](#6-add--staging-srccoreadd)
  - [Chunker (`src/core/add/chunker/`)](#7-chunker-srccoreaddchunker)
  - [Index (`src/core/add/index.rs`)](#8-index-srccoreaddindexrs)
  - [Commit & Tree (`src/core/commit/`)](#9-commit--tree-srccorecommit)
  - [Blob (`src/core/commit/blob.rs`)](#10-blob-srccorecommitblobrs)
  - [Garbage Collection (`src/core/storage/gc/`)](#11-garbage-collection-srccorestoragegc)
  - [Status (`src/core/storage/status.rs`)](#12-status-srccorestatusstatusrs)
  - [Error Handling (`src/core/error.rs`)](#13-error-handling-srccoreerrrors)
- [Quick Start Guide](#-quick-start-guide)
- [Commands Reference](#-commands-reference)
- [Configuration (`gato.toml`)](#%EF%B8%8F-configuration-gatotoml)
- [Object Storage Layout](#-object-storage-layout)
- [Built With](#-built-with)
- [License](#-license)

---

## 🚀 Key Features

| Feature | Description |
| --- | --- |
| **⚡ Parallel Processing** | Built on Rust's memory safety and the `Rayon` library. File hashing, compression, and indexing are all parallelized to maximize CPU utilization. |
| **🧩 Content-Defined Chunking (CDC)** | Uses the `FastCDC` algorithm to split large files (≥ 8 MB) into variable-sized chunks (1–8 MB). Minor changes in a file do not result in full re-uploads. |
| **📦 Zstd Compression** | Compresses all stored blobs with `Zstd` at a configurable compression level (1–22). |
| **🛡️ Blake3 Hashing** | Uses the `Blake3` cryptographic hash — one of the fastest in the world — for content-addressable storage and integrity verification. |
| **🧹 Garbage Collection** | Built-in `gc` command to prune unreferenced objects across all linked repositories. |
| **🔀 Three-Way Merge** | Full three-way merge with automatic conflict resolution for text files (using `diffy`) and conflict markers for binary files. |
| **📊 Status Tracking** | Colorized `status` command showing staged, modified, and untracked files. |
| **🗂️ Memory-Mapped I/O** | Large files (> 16 KB) are memory-mapped via `memmap2` for zero-copy reads. |
| **🆔 UUID v7 Repo IDs** | Each repository gets a unique, time-sortable UUID v7 identifier. |

---

## 🏗 Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                           CLI (clap)                                │
│   init │ add │ commit │ checkout │ new-branch │ change-branch │ ... │
└────────┬─────┬────────┬──────────┬────────────┬───────────────┬─────┘
         │     │        │          │            │               │
         ▼     ▼        ▼          ▼            ▼               ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      LocalStorage (StorageEngine)                   │
│  ┌──────────┐  ┌─────────┐  ┌──────────┐  ┌───────┐  ┌──────────┐ │
│  │ Objects  │  │  Refs   │  │  Index   │  │  GC   │  │  Status  │ │
│  │ (blobs,  │  │ (branch │  │ (staging │  │       │  │          │ │
│  │  trees,  │  │  heads, │  │  area)   │  │       │  │          │ │
│  │ commits) │  │  HEAD)  │  │          │  │       │  │          │ │
│  └──────────┘  └─────────┘  └──────────┘  └───────┘  └──────────┘ │
└──────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│         Content-Addressable Store        │
│    ~/.local/share/gato/objects/XX/YY..   │
│         (Zstd compressed blobs)          │
└──────────────────────────────────────────┘
```

---

## 📁 Project Structure

```
gato/
├── Cargo.toml                         # Project manifest & dependencies
├── LICENSE                            # AGPL-3.0 License
├── README.md                          # This file
└── src/
    ├── main.rs                        # Entry point, CLI dispatch, global store path
    └── core/
        ├── mod.rs                     # Module declarations
        ├── error.rs                   # Unified error types (GatoResult)
        ├── cli/
        │   ├── mod.rs                 # CLI init logic, global store path
        │   └── cli.rs                 # Clap CLI definition (Commands enum)
        ├── config/
        │   ├── mod.rs                 # Config & CompressionConfig structs
        │   └── load.rs               # TOML config loader
        ├── init/
        │   ├── mod.rs                 # Repository initialization (file layout)
        │   ├── lib.rs                 # UUID v7 ID generator
        │   └── config.toml            # Template config embedded at compile time
        ├── storage/
        │   ├── mod.rs                 # StorageEngine trait & StorageError
        │   ├── local.rs               # LocalStorage implementation (all operations)
        │   ├── gc/
        │   │   └── mod.rs             # Garbage collector
        │   └── status.rs             # File status (staged/modified/untracked)
        ├── add/
        │   ├── mod.rs                 # File reading, hashing, compression, staging
        │   ├── index.rs               # Index & IndexEntry (staging area)
        │   └── chunker/
        │       └── mod.rs             # FastCDC chunking for large files
        └── commit/
            ├── mod.rs                 # Commit, Tree, TreeEntry, merge logic
            ├── blob.rs                # Blob enum (Normal | ChunksMap)
            └── error.rs              # CommitError type
```

---

## 📦 Dependencies (`Cargo.toml`)

```toml
[package]
name = "gato"
version = "0.6.0"
edition = "2024"
license = "AGPL-3.0-only"
```

| Crate | Version | Purpose |
| --- | --- | --- |
| `blake3` | 1.8.2 | Ultra-fast cryptographic hashing (content addressing) |
| `chrono` | 0.4.42 | Timestamp generation for commits |
| `fastcdc` | 3.2.1 | Content-defined chunking (CDC) for large file deduplication |
| `hex` | 0.4.3 | Hex encoding/decoding of hashes |
| `memmap2` | 0.9.9 | Memory-mapped file I/O for zero-copy reads |
| `rayon` | 1.11.0 | Data parallelism (parallel iterators for hashing/compression) |
| `toml` | 0.9.10 | TOML config file parsing |
| `uuid` | 1.19.0 | UUID v7 generation for repository IDs |
| `zstd` | 0.13.3 | Zstandard compression/decompression |
| `bincode` | 2.0.1 | High-performance binary serialization (index, trees, commits) |
| `directories` | 6.0.0 | Platform-specific data directory resolution |
| `thiserror` | 2.0.17 | Ergonomic error type derivation |
| `serde` | 1.0.228 | Serialization/deserialization framework |
| `clap` | 4.5.51 | Command-line argument parsing with derive macros |
| `colored` | 3.1.1 | Colorized terminal output |
| `diffy` | 0.4.2 | Three-way merge for text files |
| `tracing` | 0.1.44 | Structured logging and instrumentation |
| `tracing-subscriber` | 0.3.22 | Tracing output formatting with env-filter |

---

## 🔍 How It Works — Module by Module

### 1. Entry Point (`src/main.rs`)

The application entry point performs three tasks:

1. **Initializes tracing** via `tracing_subscriber` with an environment filter (`RUST_LOG`).
2. **Resolves the global store path** using `OnceLock` — a lazily-initialized, thread-safe static. The path is resolved via the `directories` crate (`ProjectDirs::from("com", "gatocloud", "gato")`) which maps to platform-specific locations:
   - **Linux:** `~/.local/share/gato/`
   - **macOS:** `~/Library/Application Support/com.gatocloud.gato/`
   - **Fallback:** `.gato/` in the current directory
3. **Dispatches CLI commands** by parsing with `clap` and matching on the `Commands` enum. Each command loads a `LocalStorage` instance (either via `load_from` for repo-specific commands, or `tmp` for global commands like `gc` and `list-repos`) and calls the appropriate method.

**Error handling:** The `run()` function returns a `GatoResult<()>`. If any error propagates up, `main()` prints it in red using the `colored` crate.

---

### 2. CLI (`src/core/cli/`)

#### `cli.rs` — Command Definitions

Uses `clap`'s derive API to define the full CLI interface:

```rust
pub struct Cli {
    pub path: PathBuf,     // --path (-p), defaults to "."
    pub command: Commands,  // Subcommand
}
```

**All commands with their aliases:**

| Command | Alias | Arguments |
| --- | --- | --- |
| `init` | `i` | *(none)* |
| `add` | `a` | `paths: Vec<String>` — files/directories to stage |
| `commit` | `c` | `message: String` — commit message |
| `checkout` | `co` | `commit_index: usize` — 0 = latest commit |
| `new-branch` | `nb` | `branch_name: String` |
| `change-branch` | `cb` | `branch_name: String` |
| `soft-reset` | `ci` | `commit_index: usize` |
| `gc` | `gc` | *(none)* |
| `list-repos` | `lr` | *(none)* |
| `delete-repo` | `dr` | *(none)* |
| `delete-branch` | `db` | `name: String` |
| `status` | `st` | *(none)* |
| `merge` | `m` | `target_branch: String`, `message: String` |

#### `mod.rs` — Initialization Wrapper

Contains the `init()` function which:
1. Attempts to load an existing `gato.toml` from the given path.
2. If found → prints "Repo already initialized".
3. If not found → creates a new `LocalStorage` with a fresh UUID v7 ID, then calls `create_file_layout()` to scaffold the repository.

---

### 3. Configuration (`src/core/config/`)

#### `mod.rs` — Config Structs

```rust
pub struct Config {
    pub title: String,
    pub id: String,                              // UUID v7 repo identifier
    pub author: String,
    pub email: Option<String>,
    pub description: String,
    pub compression: Option<CompressionConfig>,
    ignore: Vec<String>,                         // Patterns to ignore
}

pub struct CompressionConfig {
    pub level: Option<i32>,   // Zstd level (1-22), defaults to 1
}
```

The `ignored()` method returns the user's ignore list **plus** two hardcoded entries: `.gato` and `gato.toml`.

#### `load.rs` — Config Loader

Reads `gato.toml` from the working directory using `smart_read()` (memory-mapped for large files), converts to UTF-8, and deserializes with `toml::from_str()`.

---

### 4. Initialization (`src/core/init/`)

#### `lib.rs` — ID Generator

```rust
pub(crate) fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
```

Generates a **UUID v7** (time-sortable, monotonic) identifier for each new repository.

#### `mod.rs` — File Layout Creation

The `create_file_layout()` function:
1. Reads the embedded `config.toml` template (compiled into the binary via `include_str!`).
2. Replaces the `<repo_id>` placeholder with the actual UUID.
3. Writes `gato.toml` to the working directory.
4. Registers the repo in the global `repos` file via `push_to_repos()`.
5. Creates the directory structure: `<store_path>/<repo_id>/refs/heads/`.

#### `config.toml` — Template

```toml
title = "My App Config"
id = "<repo_id>"
author = "Gato"
description = "Gato Repo description"
ignore = ["target"]
[compression]
level = 1
```

---

### 5. Storage Engine (`src/core/storage/`)

#### `mod.rs` — `StorageEngine` Trait

Defines the abstract interface that any storage backend must implement:

```rust
pub trait StorageEngine: Send + Sync {
    fn get(&self, hash: &String) -> Result<Vec<u8>, StorageError>;
    fn put(&self, hash: &String, data: Vec<u8>) -> Result<(), StorageError>;
    fn exist(&self, hash: &String) -> bool;
    fn write_ref(&self, ref_name: String, hash: Vec<u8>) -> Result<(), StorageError>;
    fn setup(&self) -> Result<(), StorageError>;
    fn new_branch(&self, name: String) -> Result<(), StorageError>;
    fn change_branch(&self, name: String) -> Result<(), StorageError>;
}
```

The trait is `Send + Sync` to allow safe sharing across parallel iterators.

#### `local.rs` — `LocalStorage` Implementation

The primary storage backend. Key fields:

```rust
pub struct LocalStorage {
    pub root_path: PathBuf,   // Global store (e.g., ~/.local/share/gato/)
    repo_id: String,          // UUID v7 of this repository
    work_dir: PathBuf,        // Working directory (where gato.toml lives)
}
```

**Object addressing:** Objects are stored using a **fan-out directory structure** (like Git):
```
<root_path>/objects/<first 2 hex chars>/<remaining hex chars>
```

**Key methods:**

| Method | What It Does |
| --- | --- |
| `load_from(store_path, work_dir)` | Loads config from `gato.toml`, extracts `id`, creates `LocalStorage` |
| `tmp(store_path)` | Creates a temporary storage (no repo ID) for global commands |
| `add_paths(paths)` | Resolves all files (recursively), then calls `add_all()` in parallel |
| `commit(message)` | Creates a `Commit` from the current `Index`, saves it, deletes the index file |
| `check_out(commit_index)` | Loads a commit by index (0 = latest), writes its tree to the working directory |
| `soft_reset(commit_index)` | Moves the current branch ref to point at a different commit |
| `gc()` | Runs garbage collection across all linked repositories |
| `delete_repo()` | Removes `gato.toml` and the entire `<repo_id>/` directory |
| `delete_branch(name)` | Deletes a branch ref (prevents deleting the active branch) |
| `status()` | Shows staged/modified/untracked files with color coding |
| `merge(target_branch, message)` | Three-way merge of current branch with target branch |
| `list_repos()` | Reads the global `repos` binary file to list all registered repos |
| `list_branchs()` | Lists all branch names from `refs/heads/` |
| `list_files()` | Enumerates all object hashes in the store (for GC) |

**`StorageEngine` implementation:**
- `get()` — reads the file at `objects/XX/YYY...`
- `put()` — writes data to `objects/XX/YYY...`, creates parent dirs, **skips if object already exists** (content-addressable deduplication)
- `exist()` — checks if the object path exists on disk
- `write_ref()` — writes raw hash bytes to `<repo_id>/refs/heads/<branch_name>`
- `setup()` — creates `<repo_id>/refs/heads/` directory structure
- `new_branch()` — copies the current branch's HEAD hash to a new branch file
- `change_branch()` — writes the branch name to `<repo_id>/HEAD`

---

### 6. Add & Staging (`src/core/add/`)

#### `mod.rs` — Core Add Logic

**`FileContent` enum:**
```rust
pub enum FileContent {
    Mmapped(Mmap),      // Memory-mapped (files > 16 KB)
    Loaded(Vec<u8>),    // Heap-loaded (files ≤ 16 KB)
}
```
Implements `Deref<Target = [u8]>` so both variants can be used transparently as byte slices.

**`smart_read(path)`** — Adaptive file reading:
- Files **> 16 KB** → memory-mapped with `memmap2` (zero-copy, OS-managed paging)
- Files **≤ 16 KB** → read into a `Vec<u8>` via `BufReader`

**`compute_hash(data)`** — Computes a **Blake3** 32-byte hash of the input data.

**`compress(data, work_dir)`** — Reads the compression config from `gato.toml` and compresses with Zstd at the configured level (defaults to 1).

**`decompress(data)`** — Decompresses Zstd-encoded data.

**`add_file(file_path, storage)`:**
1. Reads the file with `smart_read()`
2. Computes Blake3 hash
3. If the hash doesn't already exist in storage → compresses and stores as `Blob::Normal`
4. Returns an `IndexEntry` with hash, file size, mtime, and Unix permissions mode

**`add_all(paths, storage)`** — The parallelized staging pipeline:
1. Loads or creates a new `Index`
2. Uses `rayon`'s `par_iter()` to process all files in parallel:
   - Files **< 8 MB** → processed as a single blob via `add_file()`
   - Files **≥ 8 MB** → processed via `add_as_chunk()` (chunked storage)
3. Collects results, adds entries and dependencies to the index
4. Saves the index

**`find_files(dir_path, storage)`** — Recursively walks a directory, skipping ignored paths.

**`is_ignored(path, ignored_patterns)`** — Checks each path component against the ignore list (exact match, not glob).

**`get_dry_hash(file_path, storage)`** — Computes the hash of a file **without** storing it (used by `status`).

---

### 7. Chunker (`src/core/add/chunker/`)

Handles large files (≥ 8 MB) by splitting them into content-defined chunks.

**`cut(data)`** — Uses **FastCDC v2020** to split data into variable-sized chunks:
- Min chunk size: **1 MB**
- Average chunk size: **4 MB**
- Max chunk size: **8 MB**

Content-defined chunking means chunk boundaries are determined by the **content** of the file (using a rolling hash), not by fixed offsets. This ensures that inserting or deleting bytes in the middle of a file only affects the chunks around the edit, not the entire file.

**`process_chunk(chunks, storage)`:**
1. Uses `rayon` to hash and compress all chunks **in parallel**
2. For each chunk: computes Blake3 hash, checks if it already exists in storage, compresses if new
3. Returns a `ChunkerResult` containing:
   - `chunks: BTreeMap<Vec<u8>, Vec<u8>>` — hash → compressed data (only new chunks)
   - `ordered_hashes: Vec<Vec<u8>>` — ordered list of chunk hashes

**`ChunkerResult`:**
- `save_chunks()` — saves all new chunks to storage in parallel via `par_iter()`
- `index_data()` — creates a `Blob::ChunksMap(IndexData)` containing the ordered list of chunk hashes, serialized with bincode

**`add_as_chunk(path, storage)`:**
1. Reads the file → cuts into chunks → processes in parallel
2. Saves all chunks to storage
3. Creates an `IndexData` blob (the "manifest" that maps ordered chunk hashes)
4. Hashes and saves the index data blob
5. Returns the path, IndexEntry, and list of all dependency hashes

**`IndexData`:**
```rust
pub struct IndexData {
    pub path: Vec<Vec<u8>>,  // Ordered list of chunk hashes
}
```
- `restore_file()` — reconstructs the original file by reading each chunk from storage, decompressing, and writing sequentially

---

### 8. Index (`src/core/add/index.rs`)

The **staging area** — a binary file mapping file paths to their metadata.

```rust
pub struct IndexEntry {
    pub hash: Vec<u8>,   // Blake3 hash of the file content
    pub size: u64,       // File size in bytes
    pub mtime: u32,      // Last modification time (seconds since epoch)
    pub mode: u32,       // Unix file permissions
}

pub struct Index {
    pub entries: BTreeMap<PathBuf, IndexEntry>,  // Path → metadata (sorted)
    pub dependencies: Vec<String>,               // All object hashes this index depends on
}
```

- `BTreeMap` ensures entries are sorted by path (deterministic ordering)
- Serialized with `bincode` to `<repo_id>/index`
- The index file is **deleted** after each successful commit

---

### 9. Commit & Tree (`src/core/commit/`)

#### `mod.rs` — Commit and Tree Objects

**`Commit` enum** (versioned for forward compatibility):

```rust
pub enum Commit {
    V1 {
        message: String,
        author: String,
        timestamp: u64,             // Unix timestamp
        email: Option<String>,
        tree_hash: Vec<u8>,         // Hash of the root Tree object
        parent_hash: Option<Vec<u8>>,  // Previous commit (None for initial)
        dependencies: Vec<String>,  // All object hashes this commit needs
    },
    MergedCommitV1 {
        message: String,
        author: String,
        timestamp: u64,
        email: Option<String>,
        tree_hash: Vec<u8>,
        parent_hash1: Vec<u8>,      // Current branch's commit
        parent_hash2: Vec<u8>,      // Target branch's commit
        dependencies: Vec<String>,
    },
}
```

**Key commit methods:**
- `new()` — builds a tree from the current index, reads author/email from config, gets parent hash from the active branch ref
- `save()` — serializes with bincode, hashes with Blake3, stores the commit object, updates the branch ref
- `load(hash)` / `load_by_index(index)` — deserializes a commit from storage
- `get_hash_from_index(index)` — walks the parent chain `index` steps back from HEAD
- `parents_hashes()` — traverses the entire parent chain, returning all ancestor hashes
- `base(commit_a, commit_b)` — finds the **common ancestor** (merge base) by comparing parent chains
- `write_tree()` — reconstructs the full file tree from a commit

**`TreeEntry` enum:**
```rust
enum TreeEntry {
    Blob(String, Vec<u8>),  // (filename, content_hash)
    Tree(String, Vec<u8>),  // (dirname, tree_hash)
}
```

**`Tree` struct:**
```rust
pub struct Tree {
    name: String,
    entries: Vec<TreeEntry>,
}
```

**Tree construction** (`create_from_index`):
1. Takes the index entries (path → hash) and builds a hierarchical tree
2. `build_recursive_tree()` recursively:
   - Separates files (single-component paths → `TreeEntry::Blob`) from directories (multi-component paths → recurse)
   - Groups files by their first path component (directory name)
   - Creates sub-trees for each directory
   - Saves each tree object to storage
   - Returns the root `TreeEntry::Tree`
3. Each tree is serialized with bincode, hashed with Blake3, and stored in the object store

**Three-way merge** (`Tree::merge`):

The merge algorithm compares three trees: **base** (common ancestor), **current** (active branch), and **target** (branch being merged):

1. Collects all unique filenames across all three trees
2. For each file:
   - **`current == target`** → no conflict, use current
   - **`current == base`** → only target changed, use target
   - **`target == base`** → only current changed, use current
   - **Both changed (blobs)** → attempts text merge via `diffy::merge(base, current, target)`:
     - Success → saves the merged content
     - Conflict → saves with conflict markers and prints a warning
   - **Both changed (subtrees)** → recursively merges the subtrees
   - **Type mismatch (blob ↔ tree)** → returns `MergeConflict` error

---

### 10. Blob (`src/core/commit/blob.rs`)

```rust
pub enum Blob {
    Normal(Vec<u8>),           // Compressed file content (single blob)
    ChunksMap(IndexData),      // Ordered list of chunk hashes (for large files)
}
```

**Methods:**
- `restore(path, storage)` — writes the original file to disk:
  - `Normal` → decompress and write
  - `ChunksMap` → calls `IndexData::restore_file()` which reads, decompresses, and concatenates each chunk
- `restore_data()` — returns the decompressed bytes (only for `Normal` blobs)
- `encode()` — serializes with bincode

---

### 11. Garbage Collection (`src/core/storage/gc/`)

The `Gc` struct orchestrates garbage collection across **all linked repositories** sharing the same object store.

**Algorithm:**
1. Load all registered repositories from the global `repos` file
2. For each repository, across all branches:
   - Walk the entire commit history
   - Collect all commit hashes and their dependency lists (tree hashes, blob hashes, chunk hashes)
3. Combine all dependencies into a global set of "referenced" objects
4. List every object in `objects/`
5. Delete any object **not** in the referenced set

This ensures objects shared between repositories are never prematurely deleted.

**Safety:** The `gc` command prevents running if there are uncommitted staged files (`GcError`).

---

### 12. Status (`src/core/storage/status.rs`)

The `FileStatus` enum represents the state of each file:

```rust
pub enum FileStatus {
    ToBeCommited { path: PathBuf },       // Staged and different from last commit
    NotStagedForCommit { path: PathBuf }, // Staged but modified since staging
    UntrackedFiles { path: PathBuf },     // Not in the index at all
    Unmodified,                           // Same as last commit
}
```

**How status is determined:**
1. Compute the current hash of the file on disk (`get_dry_hash`)
2. Look up the file in the index (staging area)
3. Compare:
   - **Hash in last commit's dependencies** → `Unmodified`
   - **Hash matches index entry** → `ToBeCommited` (staged, ready to commit)
   - **Hash differs from index entry** → `NotStagedForCommit` (modified after staging)
   - **Not in index** → `UntrackedFiles`

Output uses `colored` for visual clarity:
- 🟢 **Green** — to be committed
- 🟡 **Yellow** — modified (not staged)
- 🔴 **Red** — untracked

---

### 13. Error Handling (`src/core/error.rs`)

Uses `thiserror` for ergonomic error derivation:

```rust
pub enum Error {
    StorageError(StorageError),
    CommitError(CommitError),
    IoError(std::io::Error),
    DeserialzeError(toml::de::Error),
    DecodeError(bincode::error::DecodeError),
    EncodeError(bincode::error::EncodeError),
    GcError,                        // "There are staged files that have not been committed yet"
    ActiveBranchDeletionError,      // "Cannot delete the active branch"
    NoFilesAddedError,              // "you don't add any file after last commit"
    MergeConflict(String),          // "Merge conflict detected in file: {}"
    RestoreDataError,               // "cannot restore data from blob"
    FromUTF8Error(FromUtf8Error),
}

pub type GatoResult<T> = std::result::Result<T, Error>;
```

All errors are automatically convertible via `#[from]`, enabling clean `?` propagation throughout the codebase.

---

## 💻 Quick Start Guide

### 1. Build from Source

```bash
git clone https://github.com/omar238sh/gato.git
cd gato
cargo build --release
```

The binary will be at `target/release/gato`.

### 2. Initialize a Repository

```bash
gato init
```

This creates a `gato.toml` configuration file in the current directory and sets up the internal storage at `~/.local/share/gato/`.

### 3. Add and Commit

```bash
gato add .
gato commit "Initial commit"
```

### 4. Branching Workflow

```bash
gato new-branch feature-parallel-hashing
gato change-branch feature-parallel-hashing
# ... make changes ...
gato add .
gato commit "Add parallel hashing"
gato change-branch master
gato merge feature-parallel-hashing "Merge parallel hashing feature"
```

### 5. Restore and Reset

```bash
gato checkout 0    # Restores the working directory to the latest commit
gato checkout 3    # Restores to 3 commits back
gato soft-reset 5  # Moves HEAD back 5 commits (doesn't touch files)
```

### 6. Check Status

```bash
gato status
```

### 7. Maintenance

```bash
gato gc            # Remove unreferenced objects
gato list-repos    # Show all linked repositories
gato delete-branch old-feature   # Delete a branch
gato delete-repo   # Completely remove the repository
```

---

## 📜 Commands Reference

| Command | Alias | Description |
| --- | --- | --- |
| `gato init` | `i` | Initialize a new Gato repository in the current directory |
| `gato add <paths...>` | `a` | Add file contents to the staging index |
| `gato commit <message>` | `c` | Record staged changes to the repository |
| `gato checkout <index>` | `co` | Checkout a specific commit (`0` for the latest) |
| `gato new-branch <name>` | `nb` | Create a new branch from the current HEAD |
| `gato change-branch <name>` | `cb` | Switch to an existing branch |
| `gato soft-reset <index>` | `ci` | Reset the branch HEAD to a specific commit index |
| `gato gc` | — | Garbage collect unreferenced objects across all repos |
| `gato list-repos` | `lr` | List all repositories linked to this Gato instance |
| `gato delete-repo` | `dr` | Completely remove the current repository |
| `gato delete-branch <name>` | `db` | Delete a branch (cannot delete the active branch) |
| `gato status` | `st` | Show staged, modified, and untracked files |
| `gato merge <branch> <msg>` | `m` | Three-way merge a branch into the current branch |

**Global option:** `--path (-p)` — specify the working directory (defaults to `.`).

---

## ⚙️ Configuration (`gato.toml`)

Created automatically by `gato init`. Customize it to fit your project:

```toml
title = "My Project"
id = "019476e2-..."           # Auto-generated UUID v7 (do not modify)
author = "Your Name"
email = "you@example.com"     # Optional
description = "Project description"
ignore = ["target", "node_modules", ".git"]

[compression]
level = 3                     # Zstd compression level (1-22)
                              # 1 = fastest, 22 = smallest
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `title` | String | ✅ | Project title |
| `id` | String | ✅ | UUID v7 repository identifier (auto-generated) |
| `author` | String | ✅ | Author name (recorded in commits) |
| `email` | String | ❌ | Author email (recorded in commits) |
| `description` | String | ✅ | Project description |
| `ignore` | Array | ✅ | List of directory/file names to exclude |
| `compression.level` | Integer | ❌ | Zstd compression level (default: 1) |

---

## 🗄 Object Storage Layout

```
~/.local/share/gato/
├── objects/                          # Content-addressable object store
│   ├── a3/                           # First 2 hex chars of hash
│   │   ├── 9f8e2c...                 # Remaining chars (blob/tree/commit)
│   │   └── ...
│   └── ...
├── repos                             # Binary file listing all registered repo paths
└── <uuid-v7>/                        # Per-repository metadata
    ├── HEAD                          # Current branch name (plain text)
    ├── index                         # Staging area (bincode-serialized Index)
    └── refs/
        └── heads/
            ├── master                # Branch ref (raw hash bytes)
            └── feature-branch        # Branch ref
```

**Key design decisions:**
- **Shared object store**: All repositories on the same machine share `objects/`. Identical content is stored only once.
- **Fan-out directories**: The first 2 hex characters of the hash form subdirectories, preventing any single directory from having too many entries.
- **Raw binary refs**: Branch refs store the commit hash as **raw bytes** (not hex-encoded text), for compact storage.
- **Bincode index**: The staging index uses bincode for fast serialization/deserialization (much faster than JSON/TOML for binary data).

---

## 🏗 Built With

| Technology | Role |
| --- | --- |
| **Rust (2024 Edition)** | Systems language with zero-cost abstractions and memory safety |
| **Blake3** | Ultra-fast cryptographic hashing for content addressing |
| **Rayon** | Work-stealing parallelism for hashing and compression |
| **FastCDC** | Content-defined chunking for efficient large file storage |
| **Bincode** | High-performance binary serialization for internal data structures |
| **Zstd** | Industry-standard compression with tunable speed/ratio |
| **memmap2** | Memory-mapped I/O for zero-copy file reads |
| **Clap** | Ergonomic CLI argument parsing with derive macros |
| **Diffy** | Three-way merge algorithm for text file conflict resolution |

---

## 📄 License

This project is licensed under the **GNU Affero General Public License v3.0** — see the [LICENSE](LICENSE) file for details.

---

**Gato** — *The speed of Rust, the power of parallelism.*
