use anyhow::{Result, anyhow};
use regex::Regex;
use std::path::PathBuf;
use std::sync::LazyLock;

static INVALID_FILENAME_CHAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[*:"'\\/?<>|]"#).unwrap());

pub trait PathUtil {
    /// Cleans or resets the internal state of the object and returns a reference to itself.
    ///
    /// # Returns
    ///
    /// * `&Self` - A mutable reference to the instance after performing the cleaning operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut obj = MyStruct::new();
    /// obj.clean();
    /// // The internal state of `obj` is now reset or cleaned.
    /// ```
    fn clean(&mut self) -> Result<&Self>;
    /// Ensures that all elements in the collection are unique by removing any duplicate entries.
    ///
    /// This method modifies the collection in-place, retaining only the first occurrence of
    /// each element, and removes subsequent duplicates. The ordering of elements in the collection
    /// is preserved.
    ///
    /// # Returns
    /// A mutable reference to the collection (`&Self`) after duplicates have been removed.
    ///
    /// # Example
    /// ```rust
    /// let mut collection = vec![1, 2, 2, 3, 4, 4, 5];
    /// collection.unique();
    /// assert_eq!(collection, vec![1, 2, 3, 4, 5]);
    /// ```
    fn unique(&mut self) -> &Self;
}

impl PathUtil for PathBuf {
    fn clean(&mut self) -> Result<&Self> {
        if let Some(filename) = self.file_name() {
            let filename = filename.to_string_lossy();
            if INVALID_FILENAME_CHAR_RE.is_match(filename.as_ref()) {
                let clean_name = INVALID_FILENAME_CHAR_RE.replace_all(filename.as_ref(), "");
                let clean_name = clean_name.trim();
                if clean_name.is_empty() {
                    return Err(anyhow!("Path did not contain any valid filename characters"));
                }
                *self = self.with_file_name(clean_name);
            }
        }
        Ok(self)
    }

    fn unique(&mut self) -> &Self {
        if !self.exists() {
            return self;
        }

        let stem = self.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let extension = self.extension().map(|e| e.to_string_lossy().into_owned());

        let mut index = 1;
        while self.exists() {
            let new_name = match &extension {
                Some(ext) => format!("{} ({}).{}", stem, index, ext),
                None => format!("{} ({})", stem, index),
            };
            *self = self.with_file_name(new_name);
            index += 1;
        }
        self
    }
}

#[cfg(test)]
mod test {
    use crate::utils::path_util::PathUtil;

    #[test]
    fn clean_path_name() {
        // Clean path should remain unchanged
        let mut path = std::path::PathBuf::from("/some/file");
        assert_eq!(path.clean().unwrap(), &std::path::PathBuf::from("/some/file"));

        // Invalid chars should be stripped (avoiding \ since it's a path separator on Windows)
        let mut path = std::path::PathBuf::from("/some/Super Duper <?>\"");
        assert_eq!(path.clean().unwrap(), &std::path::PathBuf::from("/some/Super Duper"));

        // Test other invalid chars
        let mut path = std::path::PathBuf::from("/some/file*name|here");
        assert_eq!(path.clean().unwrap(), &std::path::PathBuf::from("/some/filenamehere"));
    }

    #[test]
    fn unique_path_name() {
        // Non-existent file stays unchanged
        let mut path = std::path::PathBuf::from("./some/nonexistent/file");
        assert_eq!(path.unique(), &std::path::PathBuf::from("./some/nonexistent/file"));

        // Existing file gets (1) before extension
        let mut path = std::path::PathBuf::from("./Cargo.toml");
        assert_eq!(path.unique(), &std::path::PathBuf::from("./Cargo (1).toml"));

        // Directory without extension (src/ exists in this crate)
        let mut path = std::path::PathBuf::from("./src");
        assert_eq!(path.unique(), &std::path::PathBuf::from("./src (1)"));
    }
}
