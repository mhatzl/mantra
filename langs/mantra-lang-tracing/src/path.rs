use std::path::{Path, PathBuf};

use path_slash::PathBufExt;

pub fn make_relative(filepath: &Path, root: &Path) -> Option<PathBuf> {
    if root == filepath {
        match filepath.file_name() {
            Some(filename) => {
                return Some(PathBuf::from(filename));
            }
            None => {
                return None;
            }
        }
    }

    match filepath.strip_prefix(root) {
        Ok(relative_path) => Some(relative_path.to_path_buf()),
        Err(_) => None,
    }
}

/// Custom PathBuf to only get forward slashes when displaying the path.
pub struct SlashPathBuf(PathBuf);

impl std::fmt::Display for SlashPathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(target_os = "windows")]
        let slash_path = self.0.to_slash_lossy();
        #[cfg(not(target_os = "windows"))]
        let slash_path = self.0.to_string_lossy();

        write!(f, "{slash_path}")
    }
}

impl std::str::FromStr for SlashPathBuf {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(PathBuf::from(s)))
    }
}

impl From<PathBuf> for SlashPathBuf {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl From<SlashPathBuf> for PathBuf {
    fn from(value: SlashPathBuf) -> Self {
        value.0
    }
}

impl From<&Path> for SlashPathBuf {
    fn from(value: &Path) -> Self {
        Self(value.to_path_buf())
    }
}

impl std::ops::Deref for SlashPathBuf {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SlashPathBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_relative_filepath() {
        let root = PathBuf::from("src/");
        let filepath = PathBuf::from("src/cmd/mod.rs");

        let relative_path = make_relative(&filepath, &root).unwrap();

        assert_eq!(
            relative_path,
            PathBuf::from("cmd/mod.rs"),
            "Relative filepath not extracted correctly."
        )
    }

    #[test]
    fn filepath_is_root() {
        let root = PathBuf::from("src/main.rs");
        let filepath = PathBuf::from("src/main.rs");

        let relative_path = make_relative(&filepath, &root).unwrap();

        assert_eq!(
            relative_path,
            PathBuf::from("main.rs"),
            "Filename not used for root file."
        )
    }
}
