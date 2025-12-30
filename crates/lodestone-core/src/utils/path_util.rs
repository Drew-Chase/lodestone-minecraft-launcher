use regex::Regex;
use std::path::PathBuf;
use std::sync::LazyLock;

static INVALID_FILENAME_CHAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[*:"'\\/?<>|]"#).unwrap());

pub trait PathUtil {
    fn clean(&mut self) -> &Self;
    fn unique(&mut self) -> &Self;
}

impl PathUtil for PathBuf {
    fn clean(&mut self) -> &Self {
        if let Some(parent) = self.parent()
            && let Some(filename) = self.file_name()
        {
            let filename = filename.to_string_lossy();
            let filename = filename.as_ref();
            if INVALID_FILENAME_CHAR_RE.is_match(filename) {
                let clean_name = INVALID_FILENAME_CHAR_RE.replace_all(filename, "");
                let clean_name = clean_name.trim();
                *self = parent.join(clean_name);
            }
        }
        self
    }

    fn unique(&mut self) -> &Self {
        if let Some(filename) = self.file_name() {
            let filename = filename.to_string_lossy().into_owned();
            let mut index = 0;
            while self.exists() {
                index += 1;
                *self = self.with_file_name(format!("{} ({})", &filename, index));
            }
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
        assert_eq!(path.clean(), &std::path::PathBuf::from("/some/file"));

        // Invalid chars should be stripped (avoiding \ since it's a path separator on Windows)
        let mut path = std::path::PathBuf::from("/some/Super Duper <?>\"");
        assert_eq!(path.clean(), &std::path::PathBuf::from("/some/Super Duper"));

        // Test other invalid chars
        let mut path = std::path::PathBuf::from("/some/file*name|here");
        assert_eq!(path.clean(), &std::path::PathBuf::from("/some/filenamehere"));
    }

    #[test]
    fn unique_path_name() {
        let mut path = std::path::PathBuf::from("./some/file");
        assert_eq!(path.unique(), &std::path::PathBuf::from("./some/file"));

        let mut path = std::path::PathBuf::from("./Cargo.toml");
        assert_eq!(path.unique(), &std::path::PathBuf::from("./Cargo.toml (1)"));
    }

}
