/// Integration tests for the `gato` core module.
///
/// Each test exercises public APIs through real filesystem interactions,
/// covering the edge-cases called out in the problem statement.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::TempDir;

use gato::core::add::{
    add_all, add_file, compute_hash, compress, decompress, find_files, is_ignored, smart_read,
    FileContent,
};
use gato::core::add::index::Index;
use gato::core::commit::Commit;
use gato::core::storage::StorageEngine;
use gato::core::storage::local::LocalStorage;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Write a minimal `gato.toml` with the given `id` into `dir`.
fn write_config(dir: &Path, id: &str) {
    let cfg = format!(
        "title = \"test\"\nid = \"{id}\"\nauthor = \"Tester\"\ndescription = \"test repo\"\nignore = [\"target\"]\n\n[compression]\nlevel = 1\n",
        id = id
    );
    fs::write(dir.join("gato.toml"), cfg).unwrap();
}

/// A test environment: two temp directories and a ready-to-use [`LocalStorage`].
/// Dropping this struct removes both temp directories.
struct Env {
    _store: TempDir,
    _work: TempDir,
    storage: LocalStorage,
}

fn env() -> Env {
    let store   = TempDir::new().unwrap();
    let work    = TempDir::new().unwrap();
    let repo_id = "aaaabbbb-cccc-dddd-eeee-ffffffffffff";
    write_config(work.path(), repo_id);
    let storage = LocalStorage::new(
        store.path().to_path_buf(),
        repo_id.to_string(),
        work.path().to_path_buf(),
    );
    storage.setup().unwrap();
    Env { _store: store, _work: work, storage }
}

// ── compute_hash ─────────────────────────────────────────────────────────────

#[test]
fn hash_of_empty_slice_is_32_bytes() {
    let h = compute_hash(&[]);
    assert_eq!(h.len(), 32, "blake3 hash must always be 32 bytes");
}

#[test]
fn hash_is_deterministic() {
    let data = b"hello gato";
    assert_eq!(compute_hash(data), compute_hash(data));
}

#[test]
fn different_data_produces_different_hash() {
    let h1 = compute_hash(b"abc");
    let h2 = compute_hash(b"xyz");
    assert_ne!(h1, h2);
}

// ── compress / decompress ────────────────────────────────────────────────────

#[test]
fn compress_decompress_roundtrip_small() {
    let e = env();
    let original = b"hello world - small payload".to_vec();
    let compressed   = compress(&original, e.storage.work_dir()).unwrap();
    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, original);
}

#[test]
fn compress_decompress_roundtrip_empty() {
    let e = env();
    let original: Vec<u8> = vec![];
    let compressed   = compress(&original, e.storage.work_dir()).unwrap();
    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, original);
}

#[test]
fn compress_decompress_roundtrip_large() {
    let e = env();
    // 64 KiB of pseudo-random-ish bytes
    let original: Vec<u8> = (0u32..65_536)
        .map(|i| (i.wrapping_mul(1_664_525).wrapping_add(1_013_904_223) >> 16) as u8)
        .collect();
    let compressed   = compress(&original, e.storage.work_dir()).unwrap();
    let decompressed = decompress(&compressed).unwrap();
    assert_eq!(decompressed, original);
}

// ── smart_read ───────────────────────────────────────────────────────────────

#[test]
fn smart_read_small_file_returns_loaded_variant() {
    let dir  = TempDir::new().unwrap();
    let path = dir.path().join("small.txt");
    fs::write(&path, b"tiny").unwrap();
    let fc = smart_read(&path).unwrap();
    assert!(
        matches!(fc, FileContent::Loaded(_)),
        "files ≤ 16 KiB must use the Loaded variant"
    );
}

#[test]
fn smart_read_large_file_returns_mmapped_variant() {
    let dir  = TempDir::new().unwrap();
    let path = dir.path().join("big.bin");
    // 17 KiB – just above the 16 KiB threshold
    fs::write(&path, vec![0u8; 17 * 1024]).unwrap();
    let fc = smart_read(&path).unwrap();
    assert!(
        matches!(fc, FileContent::Mmapped(_)),
        "files > 16 KiB must use the Mmapped variant"
    );
}

#[test]
fn smart_read_boundary_exactly_16kib_uses_loaded() {
    let dir  = TempDir::new().unwrap();
    let path = dir.path().join("boundary.bin");
    // exactly 16 KiB → ≤ threshold → Loaded
    fs::write(&path, vec![0u8; 16 * 1024]).unwrap();
    let fc = smart_read(&path).unwrap();
    assert!(matches!(fc, FileContent::Loaded(_)));
}

#[test]
fn smart_read_nonexistent_file_returns_error() {
    let result = smart_read(Path::new("/tmp/gato_no_such_file_xyz_______abc.bin"));
    assert!(result.is_err(), "reading a non-existent file must fail");
}

// ── is_ignored ───────────────────────────────────────────────────────────────

#[test]
fn is_ignored_matches_direct_component() {
    let patterns = vec!["target".to_string()];
    assert!(is_ignored(Path::new("target"), &patterns));
}

#[test]
fn is_ignored_matches_nested_component() {
    let patterns = vec!["target".to_string()];
    assert!(is_ignored(Path::new("src/target/foo.rs"), &patterns));
}

#[test]
fn is_ignored_returns_false_when_no_component_matches() {
    let patterns = vec!["target".to_string(), ".git".to_string()];
    assert!(!is_ignored(Path::new("src/main.rs"), &patterns));
}

#[test]
fn is_ignored_empty_patterns_never_ignores() {
    assert!(!is_ignored(Path::new("anything/here"), &[]));
}

#[test]
fn is_ignored_partial_name_does_not_match() {
    // "target2" must NOT be ignored by a "target" pattern (exact match only).
    let patterns = vec!["target".to_string()];
    assert!(!is_ignored(Path::new("target2/foo.rs"), &patterns));
}

// ── find_files ───────────────────────────────────────────────────────────────

#[test]
fn find_files_in_empty_directory_returns_empty_vec() {
    let e   = env();
    let sub = e.storage.work_dir().join("empty_sub");
    fs::create_dir_all(&sub).unwrap();
    let found = find_files(&sub, &e.storage).unwrap();
    assert!(found.is_empty());
}

#[test]
fn find_files_discovers_files_in_flat_directory() {
    let e   = env();
    let sub = e.storage.work_dir().join("flat");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("a.txt"), b"aaa").unwrap();
    fs::write(sub.join("b.txt"), b"bbb").unwrap();

    let found = find_files(&sub, &e.storage).unwrap();
    assert_eq!(found.len(), 2);
}

#[test]
fn find_files_recurses_into_nested_directories() {
    let e   = env();
    let sub = e.storage.work_dir().join("nested");
    fs::create_dir_all(sub.join("deep")).unwrap();
    fs::write(sub.join("top.txt"),          b"top").unwrap();
    fs::write(sub.join("deep/bottom.txt"),  b"bottom").unwrap();

    let found = find_files(&sub, &e.storage).unwrap();
    assert_eq!(found.len(), 2);
}

#[test]
fn find_files_respects_ignore_patterns_from_config() {
    let e   = env();
    let sub = e.storage.work_dir().join("proj");
    // "target" is in the ignore list written by `env()`
    fs::create_dir_all(sub.join("target")).unwrap();
    fs::write(sub.join("target/ignored.rs"), b"x").unwrap();
    fs::write(sub.join("main.rs"),           b"fn main() {}").unwrap();

    let found = find_files(&sub, &e.storage).unwrap();
    assert_eq!(found.len(), 1, "only main.rs should be found; target/ must be skipped");
    assert!(found[0].ends_with("main.rs"));
}

// ── Index ────────────────────────────────────────────────────────────────────

#[test]
fn index_new_has_no_entries() {
    let idx = Index::new();
    assert!(idx.entries.is_empty());
    assert!(idx.dependencies.is_empty());
}

#[test]
fn index_save_and_load_roundtrip_empty() {
    let e   = env();
    let idx = Index::new();
    idx.save(&e.storage).unwrap();
    let loaded = Index::load(&e.storage).unwrap();
    assert!(loaded.entries.is_empty());
}

#[test]
fn index_save_and_load_roundtrip_with_entries() {
    let e   = env();
    let mut idx = Index::new();
    idx.add_entry(
        PathBuf::from("src/main.rs"),
        gato::core::add::index::IndexEntry {
            hash:  vec![1, 2, 3, 4],
            size:  42,
            mtime: 12_345,
            mode:  0o644,
        },
    );
    idx.dependencies.push("dep-hash-abc".to_string());
    idx.save(&e.storage).unwrap();

    let loaded = Index::load(&e.storage).unwrap();
    assert_eq!(loaded.entries.len(), 1);
    assert!(loaded.entries.contains_key(&PathBuf::from("src/main.rs")));
    assert_eq!(loaded.dependencies, vec!["dep-hash-abc".to_string()]);
}

#[test]
fn index_load_from_missing_file_returns_error() {
    let e = env();
    // No index has been saved; loading must fail.
    assert!(Index::load(&e.storage).is_err());
}

// ── LocalStorage / StorageEngine ─────────────────────────────────────────────

#[test]
fn storage_exist_is_false_before_any_put() {
    let e = env();
    assert!(!e.storage.exist(&"deadbeef00112233".repeat(4)));
}

#[test]
fn storage_put_get_exist_roundtrip() {
    let e    = env();
    let data = b"blob content".to_vec();
    let hash = hex::encode(compute_hash(&data));

    assert!(!e.storage.exist(&hash));
    e.storage.put(&hash, data.clone()).unwrap();
    assert!(e.storage.exist(&hash));
    assert_eq!(e.storage.get(&hash).unwrap(), data);
}

#[test]
fn storage_put_same_hash_twice_is_idempotent() {
    let e    = env();
    let data = b"idempotent blob".to_vec();
    let hash = hex::encode(compute_hash(&data));

    e.storage.put(&hash, data.clone()).unwrap();
    // Second put must not error even though the object already exists.
    e.storage.put(&hash, data.clone()).unwrap();
    assert_eq!(e.storage.get(&hash).unwrap(), data);
}

#[test]
fn storage_get_nonexistent_hash_returns_error() {
    let e = env();
    assert!(e.storage.get(&"0".repeat(64)).is_err());
}

#[test]
fn storage_write_ref_and_read_ref_vec_roundtrip() {
    let e         = env();
    let fake_hash = vec![0xde, 0xad, 0xbe, 0xef];
    e.storage.write_ref("master".to_string(), fake_hash.clone()).unwrap();
    let retrieved = e.storage.read_ref_vec("master".to_string()).unwrap();
    assert_eq!(retrieved, fake_hash);
}

#[test]
fn storage_read_ref_of_missing_branch_returns_error() {
    let e = env();
    assert!(e.storage.read_ref_vec("no-such-branch".to_string()).is_err());
}

#[test]
fn storage_get_active_branch_defaults_to_master_when_head_missing() {
    let e = env();
    // HEAD file is not written by setup(), so the fallback must be "master".
    assert_eq!(e.storage.get_active_branche(), "master");
}

#[test]
fn storage_change_branch_updates_active_branch() {
    let e = env();
    e.storage.change_branch("feature".to_string()).unwrap();
    assert_eq!(e.storage.get_active_branche(), "feature");
}

#[test]
fn storage_setup_creates_refs_heads_directory() {
    // `env()` already calls setup(); verify the resulting directory layout.
    let e = env();
    let heads_dir = e.storage.repo_path().join("refs").join("heads");
    assert!(heads_dir.is_dir(), "refs/heads must exist after setup()");
}

#[test]
fn storage_new_branch_creates_branch_pointing_at_same_commit() {
    let e         = env();
    let fake_hash = vec![0xca, 0xfe, 0xba, 0xbe];
    // Write master ref so new_branch() has a source to copy.
    e.storage.write_ref("master".to_string(), fake_hash.clone()).unwrap();

    e.storage.new_branch("feature".to_string()).unwrap();

    let branch_hash = e.storage.read_ref_vec("feature".to_string()).unwrap();
    assert_eq!(branch_hash, fake_hash, "new branch should point at the same commit as master");
}

#[test]
fn storage_list_files_reflects_stored_objects() {
    let e    = env();
    let data = b"some content".to_vec();
    let hash = hex::encode(compute_hash(&data));
    // put creates the objects/ directory as a side-effect
    e.storage.put(&hash, data).unwrap();

    let files = e.storage.list_files().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], hash);
}

#[test]
fn storage_list_branchs_after_write_ref() {
    let e = env();
    assert!(e.storage.list_branchs().unwrap().is_empty());

    e.storage.write_ref("master".to_string(), vec![0u8; 32]).unwrap();
    let branches = e.storage.list_branchs().unwrap();
    assert_eq!(branches, vec!["master".to_string()]);
}

// ── add_file ─────────────────────────────────────────────────────────────────

#[test]
fn add_file_stores_blob_and_returns_correct_hash() {
    let e         = env();
    let file_path = e.storage.work_dir().join("hello.txt");
    fs::write(&file_path, b"hello gato").unwrap();

    let entry         = add_file(&file_path, &e.storage).unwrap();
    let expected_hash = compute_hash(b"hello gato");

    assert_eq!(entry.hash, expected_hash.to_vec());
    assert!(e.storage.exist(&hex::encode(expected_hash)));
}

#[test]
fn add_file_is_idempotent_for_same_content() {
    let e         = env();
    let file_path = e.storage.work_dir().join("idem.txt");
    fs::write(&file_path, b"same content").unwrap();

    let entry1 = add_file(&file_path, &e.storage).unwrap();
    let entry2 = add_file(&file_path, &e.storage).unwrap();
    assert_eq!(entry1.hash, entry2.hash);
}

#[test]
fn add_file_empty_file_succeeds() {
    let e         = env();
    let file_path = e.storage.work_dir().join("empty.txt");
    fs::write(&file_path, b"").unwrap();

    let entry         = add_file(&file_path, &e.storage).unwrap();
    let expected_hash = compute_hash(b"");
    assert_eq!(entry.hash, expected_hash.to_vec());
    assert_eq!(entry.size, 0);
}

// ── add_all ──────────────────────────────────────────────────────────────────

#[test]
fn add_all_empty_paths_leaves_index_empty() {
    let e = env();
    add_all(vec![], Arc::new(e.storage.clone())).unwrap();
    let idx = Index::load(&e.storage).unwrap();
    assert!(idx.entries.is_empty());
}

#[test]
fn add_all_indexes_multiple_files() {
    let e = env();
    fs::write(e.storage.work_dir().join("a.txt"), b"file A").unwrap();
    fs::write(e.storage.work_dir().join("b.txt"), b"file B").unwrap();

    add_all(
        vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
        Arc::new(e.storage.clone()),
    )
    .unwrap();

    let idx = Index::load(&e.storage).unwrap();
    assert_eq!(idx.entries.len(), 2);
    assert!(idx.entries.contains_key(&PathBuf::from("a.txt")));
    assert!(idx.entries.contains_key(&PathBuf::from("b.txt")));
}

#[test]
fn add_all_deduplicates_objects_for_identical_content() {
    let e = env();
    fs::write(e.storage.work_dir().join("x.txt"), b"same").unwrap();
    fs::write(e.storage.work_dir().join("y.txt"), b"same").unwrap();

    add_all(
        vec![PathBuf::from("x.txt"), PathBuf::from("y.txt")],
        Arc::new(e.storage.clone()),
    )
    .unwrap();

    // Identical content must produce exactly one stored object.
    let files = e.storage.list_files().unwrap();
    assert_eq!(files.len(), 1, "identical content should produce a single stored object");
}

// ── commit workflow ───────────────────────────────────────────────────────────

#[test]
fn commit_workflow_creates_loadable_commit() {
    let e = env();
    fs::write(e.storage.work_dir().join("readme.txt"), b"Hello").unwrap();
    e.storage.add_paths(vec!["readme.txt".to_string()]).unwrap();
    e.storage.commit("initial commit".to_string()).unwrap();

    let commit = Commit::load_by_index(0, &e.storage).expect("commit must be loadable");
    assert_eq!(commit.message(), "initial commit");
}

#[test]
fn second_commit_has_first_as_parent() {
    let e = env();

    fs::write(e.storage.work_dir().join("f1.txt"), b"first").unwrap();
    e.storage.add_paths(vec!["f1.txt".to_string()]).unwrap();
    e.storage.commit("first commit".to_string()).unwrap();

    fs::write(e.storage.work_dir().join("f2.txt"), b"second").unwrap();
    e.storage.add_paths(vec!["f2.txt".to_string()]).unwrap();
    e.storage.commit("second commit".to_string()).unwrap();

    let latest  = Commit::load_by_index(0, &e.storage).unwrap();
    let earlier = Commit::load_by_index(1, &e.storage).unwrap();

    assert_eq!(latest.message(),  "second commit");
    assert_eq!(earlier.message(), "first commit");
    assert!(latest.parent_hash().is_some(), "second commit must have a parent");
}

#[test]
fn commit_without_staged_files_returns_error() {
    let e      = env();
    let result = e.storage.commit("empty commit".to_string());
    assert!(result.is_err(), "committing with no staged files must fail");
}

// ── branch workflow ───────────────────────────────────────────────────────────

#[test]
fn branch_workflow_create_and_switch() {
    let e = env();

    // Need at least one commit so master has a valid ref.
    fs::write(e.storage.work_dir().join("init.txt"), b"init").unwrap();
    e.storage.add_paths(vec!["init.txt".to_string()]).unwrap();
    e.storage.commit("init".to_string()).unwrap();

    e.storage.new_branch("feature".to_string()).unwrap();
    e.storage.change_branch("feature".to_string()).unwrap();

    assert_eq!(e.storage.get_active_branche(), "feature");

    let branches = e.storage.list_branchs().unwrap();
    assert!(branches.contains(&"master".to_string()));
    assert!(branches.contains(&"feature".to_string()));
}

#[test]
fn delete_branch_removes_non_active_branch() {
    let e = env();

    fs::write(e.storage.work_dir().join("init.txt"), b"init").unwrap();
    e.storage.add_paths(vec!["init.txt".to_string()]).unwrap();
    e.storage.commit("init".to_string()).unwrap();

    e.storage.new_branch("to-delete".to_string()).unwrap();
    assert!(e.storage.list_branchs().unwrap().contains(&"to-delete".to_string()));

    e.storage.delete_branch("to-delete".to_string()).unwrap();
    assert!(!e.storage.list_branchs().unwrap().contains(&"to-delete".to_string()));
}

#[test]
fn deleting_active_branch_returns_error() {
    let e = env();

    fs::write(e.storage.work_dir().join("init.txt"), b"init").unwrap();
    e.storage.add_paths(vec!["init.txt".to_string()]).unwrap();
    e.storage.commit("init".to_string()).unwrap();

    // "master" is currently active; deleting it must fail.
    assert!(
        e.storage.delete_branch("master".to_string()).is_err(),
        "deleting the active branch must return an error"
    );
}
