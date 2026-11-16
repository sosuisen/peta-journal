use std::fs;

use client_cloud_sync::content_hash;
use tempfile::TempDir;

use crate::fake::{hex, sha256};

// 4MBを超えるファイルのハッシュは、4MBごとのハッシュをつないだもののハッシュである。
#[test]
fn the_content_hash_of_a_large_file_is_made_of_the_hashes_of_its_blocks() {
    let dir = TempDir::new().unwrap();
    let block = 4 * 1024 * 1024;
    let content: Vec<u8> = (0..block + 3).map(|at| (at % 251) as u8).collect();
    let file = dir.path().join("large");
    fs::write(&file, &content).unwrap();
    let both = [sha256(&content[..block]), sha256(&content[block..])].concat();
    assert_eq!(content_hash(&file).unwrap(), hex(&sha256(&both)));
}

// 4MBちょうどのファイルのハッシュは、1つのブロックのハッシュの、そのまたハッシュである。
#[test]
fn the_content_hash_of_a_file_of_one_whole_block_is_made_of_that_block_only() {
    let dir = TempDir::new().unwrap();
    let content = vec![7u8; 4 * 1024 * 1024];
    let file = dir.path().join("block");
    fs::write(&file, &content).unwrap();
    assert_eq!(
        content_hash(&file).unwrap(),
        hex(&sha256(&sha256(&content)))
    );
}

// 4MBまでのファイルのハッシュは、中身のハッシュの、そのまたハッシュである。
#[test]
fn the_content_hash_of_a_small_file_is_the_hash_of_the_hash_of_its_content() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("small");
    fs::write(&file, "- a").unwrap();
    assert_eq!(content_hash(&file).unwrap(), hex(&sha256(&sha256(b"- a"))));
}

// 空のファイルのハッシュは、空の中身のハッシュである。
#[test]
fn the_content_hash_of_an_empty_file_is_the_hash_of_nothing() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("empty");
    fs::write(&file, "").unwrap();
    assert_eq!(content_hash(&file).unwrap(), hex(&sha256(&[])));
}

// 読めないファイルには、ハッシュがない。
#[test]
fn a_file_that_cannot_be_read_has_no_content_hash() {
    let dir = TempDir::new().unwrap();
    assert!(content_hash(&dir.path().join("not there")).is_err());
}
