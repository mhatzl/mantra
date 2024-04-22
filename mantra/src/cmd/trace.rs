use std::path::{Path, PathBuf};

use crate::db::MantraDb;

use ignore::{types::TypesBuilder, WalkBuilder};
use mantra_lang_tracing::{AstCollector, PlainCollector, TraceCollector, TraceEntry};

pub async fn trace(db: &MantraDb, cfg: &Config) -> Result<(), TraceError> {
    if cfg.root.is_dir() {
        let walk = WalkBuilder::new(&cfg.root)
            .types(
                TypesBuilder::new()
                    .add_defaults()
                    .select("all")
                    .build()
                    .expect("Could not create file filter."),
            )
            .build();

        for dir_entry_res in walk {
            let dir_entry = match dir_entry_res {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            if dir_entry
                .file_type()
                .expect("No file type found for given entry. Note: stdin is not supported.")
                .is_file()
            {
                if let Some(traces) = collect_traces(dir_entry.path())? {
                    db.add_traces(&cfg.project_name, &cfg.root, dir_entry.path(), &traces)
                        .await
                        .map_err(TraceError::DbError)?
                }
            }
        }
    } else if let Some(traces) = collect_traces(&cfg.root)? {
        db.add_traces(&cfg.project_name, &cfg.root, &cfg.root, &traces)
            .await
            .map_err(TraceError::DbError)?
    }

    Ok(())
}

fn collect_traces(filepath: &Path) -> Result<Option<Vec<TraceEntry>>, TraceError> {
    let is_textfile = mime_guess::from_path(filepath)
        .first()
        .map(|mime| mime.type_() == "text")
        .unwrap_or(false);

    if !is_textfile {
        // Traces are only collected from text files
        return Ok(None);
    }

    let content = std::fs::read_to_string(filepath)
        .map_err(|_| TraceError::CouldNotAccessFile(filepath.to_string_lossy().to_string()))?;

    let extension_str = filepath
        .extension()
        .map(|osstr| osstr.to_str().unwrap_or_default());

    if extension_str == Some("rs") {
        match AstCollector::new(
            content.as_bytes(),
            tree_sitter_rust::language(),
            Box::new(mantra_rust_trace::collect_traces_in_rust),
        ) {
            Some(mut collector) => {
                return Ok(collector.collect(&()));
            }
            None => {
                // warn: failed parsing Rust code => content will be parsed as plain text
            }
        }
    }

    let mut collector = PlainCollector::new(&content);
    Ok(collector.collect(&()))
}

#[derive(Debug, Clone, clap::Args)]
#[group(id = "trace")]
pub struct Config {
    pub root: PathBuf,
    pub project_name: String,
}

#[derive(Debug)]
pub enum TraceError {
    CouldNotAccessFile(String),
    DbError(crate::db::DbError),
}
