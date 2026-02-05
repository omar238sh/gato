
# 🐾 Gato
### **A High-Performance, Parallelized Version Control System**

**Gato** is a modern, blazing-fast version control system built with **Rust**. It is engineered for developers who deal with massive datasets and demand a VCS that leverages multi-core architectures to provide near-instantaneous performance.

---

## 🚀 Key Features

* **⚡ Parallel Processing:** Built on Rust's memory safety and the `Rayon` library, Gato parallelizes file hashing, compression, and indexing tasks to maximize CPU utilization.
* **🧩 Content-Defined Chunking (CDC):** Utilizes the `FastCDC` algorithm to split large files into variable-sized chunks. This ensures that minor changes in a file don't result in full re-uploads, significantly reducing storage overhead.
* **📦 Advanced Compression:** Supports industry-standard algorithms like `Zstd` and `Zlib`, allowing you to balance between extreme speed and maximum storage savings.
* **🛡️ Integrity & Security:** Uses the `Blake3` cryptographic hash function—one of the fastest in the world—to ensure data integrity without sacrificing performance.
* **🧹 Garbage Collection:** Built-in `gc` command to automatically prune unreferenced objects and keep your repository lean.

---

## 🛠 Installation

Ensure you have the [Rust toolchain](https://rustup.rs/) installed, then clone the repository and build:

```bash
git clone [https://github.com/omar238sh/gato.git](https://github.com/omar238sh/gato.git)
cd gato
cargo build --release

```

The binary will be available at `./target/release/gato`.

---

## 💻 Quick Start Guide

### 1. Initialize a Repository

Start tracking your project by creating a new Gato instance:

```bash
gato init

```

*This creates a `gato.toml` configuration file and repo in `.local/share/gato` .*

### 2. Add and Commit

Stage your files and record the current state of your project:

```bash
gato add .
gato commit "Initial commit"

```

### 3. Branching Workflow

Create isolated environments for new features:

```bash
gato new-branch feature-parallel-hashing
gato change-branch feature-parallel-hashing

```

### 4. Restore and Reset

Roll back to previous states effortlessly:

```bash
gato checkout 0    # Returns to the last commit
gato soft-reset 5  # Resets the index to a specific commit index

```

---

## 📜 Commands Reference

| Command | Description |
| --- | --- |
| `init` | Initialize a new Gato repository in the current directory. |
| `add` | Add file contents to the staging index. |
| `commit` | Record staged changes to the repository. |
| `checkout` | Checkout a specific commit (use `0` for the latest). |
| `new-branch` | Create a new branch. |
| `change-branch` | Switch to an existing branch. |
| `soft-reset` | Reset the repository head to a specific commit index. |
| `gc` | Perform garbage collection to remove unreferenced objects. |
| `list-repos` | List all repositories linked to the system. |
| `delete-repo` | Completely remove a repository. |

---

## ⚙️ Configuration (`gato.toml`)

Customize Gato's behavior to fit your project's needs:

```toml
title = "My Project"
author = "Developer Name"
description = "Gato Repo description"
ignore = ["target"]
[compression]
method = "Zstd" # Options: Zstd, Zlib
level = 3       # Compression level (1-22 for Zstd)
```

---

## 🏗 Built With

* **Rust (2024 Edition)**
* **Blake3:** For ultra-fast hashing.
* **Rayon:** For data parallelism.
* **FastCDC:** For intelligent file chunking.
* **Bincode:** For high-efficiency binary serialization.

---

**Gato** — *The speed of Rust, the power of parallelism.* 🐾
