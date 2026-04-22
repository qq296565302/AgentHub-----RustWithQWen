use sha2::{Sha256, Digest};

pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn truncate_hash(hash: &str, length: usize) -> String {
    hash[..length.min(hash.len())].to_string()
}
