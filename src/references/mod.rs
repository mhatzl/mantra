use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use regex::Regex;
use walkdir::WalkDir;

use crate::{req::ReqId, wiki::Wiki};

pub mod changes;

/// Map to store the current reference counter for direct references to requirements.
/// This counter may be used to update/validate the existing reference counts inside the wiki.
///
/// [req:ref_req]
#[derive(Debug)]
pub struct ReferencesMap {
    /// HashMap to store the current reference counter.
    ///
    /// **Note:** Atomic to be updated concurrently.
    map: HashMap<ReqId, AtomicUsize>,
}

impl TryFrom<(&Wiki, &PathBuf)> for ReferencesMap {
    type Error = ReferencesMapError;

    /// Creates a [`ReferencesMap`] for the given wiki, using references from the given project folder.
    fn try_from(value: (&Wiki, &PathBuf)) -> Result<Self, Self::Error> {
        let wiki = value.0;
        let project_folder = value.1;

        if !project_folder.exists() {
            return logid::err!(ReferencesMapError::CouldNotFindProjectFolder(
                project_folder.clone(),
            ));
        }

        let ref_map = ReferencesMap::with(&mut wiki.requirements());

        if project_folder.is_dir() {
            let mut walk = WalkDir::new(project_folder)
                .into_iter()
                // TODO: add filter option using ignore files
                .filter_entry(|entry| {
                    entry.file_name().to_string_lossy() != "target"
                        && entry.file_name().to_string_lossy() != ".git"
                        && entry.file_name().to_string_lossy() != "Cargo.lock"
                        && entry.file_name().to_string_lossy() != ".vscode"
                });
            while let Some(Ok(dir_entry)) = walk.next() {
                if dir_entry.file_type().is_file() {
                    let content = std::fs::read_to_string(dir_entry.path()).map_err(|_| {
                        logid::pipe!(ReferencesMapError::CouldNotAccessFile(
                            dir_entry.path().to_path_buf()
                        ))
                    })?;

                    ref_map.trace(dir_entry.path(), &content)?;
                }
            }
        } else {
            let content = std::fs::read_to_string(project_folder).map_err(|_| {
                logid::pipe!(ReferencesMapError::CouldNotAccessFile(
                    project_folder.clone()
                ))
            })?;

            ref_map.trace(project_folder, &content)?;
        }

        Ok(ref_map)
    }
}

/// Holds the regex matcher for requirement references.
static REFERENCES_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

impl ReferencesMap {
    fn with<'a>(requirements: &'a mut (impl Iterator<Item = &'a ReqId> + Clone)) -> Self {
        let capacity = requirements.clone().count();
        let mut map = HashMap::with_capacity(capacity);
        for req in requirements {
            map.insert(req.clone(), AtomicUsize::new(0));
        }

        ReferencesMap { map }
    }

    /// Goes through the given content and increases the reference counter for referenced requirements.
    ///
    /// **Note:** Not required for `self` to be mutable, because counts are stored as [`AtomicUsize`].
    fn trace(&self, filepath: &Path, content: &str) -> Result<usize, ReferencesMapError> {
        let references_regex = REFERENCES_MATCHER.get_or_init(|| {
            // [mantra:ignore_next]
            Regex::new(r"\[req:(?<req_id>[^\]\s]+)\]")
                .expect("Regex to match requirement references could **not** be created.")
        });

        let lines = content.lines();
        let mut added_refs = 0;
        let mut ignore_match = false;

        for (line_nr, line) in lines.enumerate() {
            if line.contains("[mantra:ignore_next]") {
                ignore_match = true;
            }
            for captures in references_regex.captures_iter(line) {
                if ignore_match {
                    ignore_match = false;
                    continue;
                }

                let req_id = captures
                    .name("req_id")
                    .expect("`req_id` capture group was not in reference match.")
                    .as_str()
                    .to_string();

                match self.map.get(&req_id) {
                    Some(cnt) => {
                        // Only increment counter, so `Relaxed` is ok
                        // Overflow is also highly unlikely (Who has 4Mrd. requirements?)
                        cnt.fetch_add(1, Ordering::Relaxed);
                        added_refs += 1;
                    }
                    None => {
                        return logid::err!(ReferencesMapError::ReqNotInWiki {
                            req_id: req_id.clone(),
                            filepath: filepath.to_path_buf(),
                            line_nr,
                        })
                    }
                }
            }
        }

        Ok(added_refs)
    }
}

/// Enum representing possible errors that may occur, when using functions for [`ReferencesMap`].
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum ReferencesMapError {
    #[error("Could not access file '{}' in the project folder.", .0.to_string_lossy())]
    CouldNotAccessFile(PathBuf),

    #[error("Could not find project folder '{}'.", .0.to_string_lossy())]
    CouldNotFindProjectFolder(PathBuf),

    // Note: +1 for line number, because internally, lines start at index 0.
    #[error("Requirement ID '{}' referenced in file '{}' at line '{}' not found in the wiki.", .req_id, .filepath.to_string_lossy(), .line_nr + 1)]
    ReqNotInWiki {
        req_id: String,
        filepath: PathBuf,
        line_nr: usize,
    },
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::wiki::Wiki;

    use super::ReferencesMap;

    fn setup_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# req_id: Some Title

**References:**

- in branch main: 2
        "#;

        Wiki::try_from((PathBuf::from(filename), content)).unwrap()
    }

    #[test]
    fn single_reference_one_line() {
        let wiki = setup_wiki();
        let filename = "test_file";
        // Note: ID must be identical to the one in `setup_wiki()`.
        let content = "[req:req_id]";

        let ref_map = ReferencesMap::with(&mut wiki.requirements());
        let added_refs = ref_map
            .trace(&PathBuf::from(filename), content)
            .expect("Failed to create references map.");

        assert_eq!(added_refs, 1, "Counter for added references is wrong.");
        assert!(
            ref_map.map.contains_key("req_id"),
            "ID `req_id` not added to the references map."
        )
    }

    #[test]
    fn two_references_two_lines() {
        let wiki = setup_wiki();
        let filename = "test_file";
        // Note: ID must be identical to the one in `setup_wiki()`.
        let content = "[req:req_id]\n[req:req_id]";

        let ref_map = ReferencesMap::with(&mut wiki.requirements());
        let added_refs = ref_map
            .trace(&PathBuf::from(filename), content)
            .expect("Failed to create references map.");

        assert_eq!(added_refs, 2, "Counter for added references is wrong.");
        assert!(
            ref_map.map.contains_key("req_id"),
            "ID `req_id` not added to the references map."
        )
    }

    #[test]
    fn two_references_separated_by_content() {
        let wiki = setup_wiki();
        let filename = "test_file";
        // Note: ID must be identical to the one in `setup_wiki()`.
        let content = "// [req:req_id]\n\n// In addition to [req:req_id].";

        let ref_map = ReferencesMap::with(&mut wiki.requirements());
        let added_refs = ref_map
            .trace(&PathBuf::from(filename), content)
            .expect("Failed to create references map.");

        assert_eq!(added_refs, 2, "Counter for added references is wrong.");
        assert!(
            ref_map.map.contains_key("req_id"),
            "ID `req_id` not added to the references map."
        )
    }
}
