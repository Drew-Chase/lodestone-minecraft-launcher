//! Reads the `SHA256SUMS.txt` that each release publishes, in `sha256sum` format: one `<hex>  <file name>` per line.

/// The SHA-256 of `file_name` as lowercase hex, when `sums` lists it.
pub fn checksum_for(sums: &str, file_name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        let name = name.trim().trim_start_matches('*');
        let is_sha256 = hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit());
        (name == file_name && is_sha256).then(|| hash.to_ascii_lowercase())
    })
}

#[cfg(test)]
#[path = "../tests/checksum.rs"]
mod tests;
