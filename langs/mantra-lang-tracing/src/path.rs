use std::path::{Path, PathBuf};

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_relative_filepath() {
        let root = PathBuf::from("src/");
        let filepath = PathBuf::from("src/cmd/mod.rs");

        let relative_path = make_relative(&root, &filepath).unwrap();

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

        let relative_path = make_relative(&root, &filepath).unwrap();

        assert_eq!(
            relative_path,
            PathBuf::from("main.rs"),
            "Filename not used for root file."
        )
    }
}
