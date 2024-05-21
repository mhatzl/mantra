use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Could not execute Cargo to find the project root directory.")]
    ExecutingCargo(std::io::Error),
    #[error("Cargo could not locate the project root directory.")]
    LocatingWorkspaceRoot,
    #[error("Path to the project root directory is not valid UTF8.")]
    InvalidPath,
    #[error("Could not get the current directory.")]
    CurrentDir,
}

/// Returns the path to the workspace directory of a Cargo workspace.
/// Reason for this workaround: https://github.com/rust-lang/cargo/issues/3946
pub fn get_cargo_root() -> Result<PathBuf, PathError> {
    let locate_project_output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--quiet")
        .arg("--message-format=plain")
        .current_dir(
            std::env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .or(std::env::current_dir())
                .map_err(|_| PathError::CurrentDir)?,
        )
        .output()
        .map_err(PathError::ExecutingCargo)?;

    if locate_project_output.status.success() {
        let workspace_root = PathBuf::from(
            String::from_utf8(locate_project_output.stdout).map_err(|_| PathError::InvalidPath)?,
        );
        Ok(workspace_root
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default())
    } else {
        Err(PathError::LocatingWorkspaceRoot)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::get_cargo_root;

    #[test]
    fn workspace_root_of_mantra() {
        let workspace_root = get_cargo_root().unwrap().canonicalize().unwrap();
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let expected_root = PathBuf::from(manifest_dir).canonicalize().unwrap();
        let expected_root = expected_root.parent().unwrap();

        assert_eq!(
            workspace_root, expected_root,
            "Returned workspace root is wrong."
        );
    }
}
