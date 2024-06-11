use std::path::{Path, PathBuf};

use crate::db::{MantraDb, TraceChanges};

use ignore::{types::TypesBuilder, WalkBuilder};
use mantra_lang_tracing::{AstCollector, PlainCollector, TraceCollector, TraceEntry};
use mantra_schema::traces::TraceSchema;

#[derive(Debug, Clone, clap::Subcommand, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceKind {
    FromSource(SourceConfig),
    FromSchema { filepath: PathBuf },
}

#[derive(Debug, Clone, clap::Args, serde::Serialize, serde::Deserialize)]
pub struct SourceConfig {
    pub root: PathBuf,
    #[arg(long)]
    #[serde(default, alias = "keep-path-absolute")]
    pub keep_path_absolute: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    #[error("Could not access file '{}'.", .0)]
    CouldNotAccessFile(String),
    #[error("{}", .0)]
    DbError(crate::db::DbError),
    #[error("{}", .0)]
    Deserialize(serde_json::Error),
}

pub async fn collect(db: &MantraDb, kind: TraceKind) -> Result<TraceChanges, TraceError> {
    match kind {
        TraceKind::FromSource(source_cfg) => trace_from_source(db, &source_cfg).await,
        TraceKind::FromSchema { filepath } => trace_from_schema_file(db, &filepath).await,
    }
}

pub async fn trace_from_schema_file(
    db: &MantraDb,
    filepath: &Path,
) -> Result<TraceChanges, TraceError> {
    let content = tokio::fs::read_to_string(filepath)
        .await
        .map_err(|_| TraceError::CouldNotAccessFile(filepath.to_string_lossy().to_string()))?;
    let schema = serde_json::from_str::<TraceSchema>(&content).map_err(TraceError::Deserialize)?;

    trace_from_schema(db, &schema).await
}

pub async fn trace_from_schema(
    db: &MantraDb,
    schema: &TraceSchema,
) -> Result<TraceChanges, TraceError> {
    let old_generation = db.max_trace_generation().await;
    let new_generation = old_generation + 1;

    let mut changes = TraceChanges {
        new_generation,
        ..Default::default()
    };

    for file_traces in &schema.traces {
        let mut trace_changes = db
            .add_traces(&file_traces.filepath, &file_traces.traces, new_generation)
            .await
            .map_err(TraceError::DbError)?;

        changes.merge(&mut trace_changes);
    }

    Ok(changes)
}

pub async fn trace_from_source(
    db: &MantraDb,
    cfg: &SourceConfig,
) -> Result<TraceChanges, TraceError> {
    let old_generation = db.max_trace_generation().await;
    let new_generation = old_generation + 1;

    let mut changes = TraceChanges {
        new_generation,
        ..Default::default()
    };

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
                    let filepath = if cfg.keep_path_absolute {
                        mantra_lang_tracing::path::make_relative(dir_entry.path(), &cfg.root)
                            .unwrap_or(dir_entry.into_path())
                    } else {
                        dir_entry.into_path()
                    };

                    let mut trace_changes = db
                        .add_traces(&filepath, &traces, new_generation)
                        .await
                        .map_err(TraceError::DbError)?;

                    changes.merge(&mut trace_changes);
                }
            }
        }

        Ok(changes)
    } else if let Some(traces) = collect_traces(&cfg.root)? {
        let filepath = if cfg.keep_path_absolute {
            mantra_lang_tracing::path::make_relative(&cfg.root, &cfg.root)
                .unwrap_or(cfg.root.to_path_buf())
        } else {
            cfg.root.to_path_buf()
        };

        db.add_traces(&filepath, &traces, new_generation)
            .await
            .map_err(TraceError::DbError)
    } else {
        Ok(changes)
    }
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
            &tree_sitter_rust::language(),
            Box::new(mantra_rust_trace::collect_traces_in_rust),
        ) {
            Some(mut collector) => {
                return Ok(collector.collect(&()));
            }
            None => {
                log::warn!(
                    "Failed parsing Rust code. File content taken as plain text: {}",
                    filepath.display()
                );
            }
        }
    }

    let mut collector = PlainCollector::new(&content);
    Ok(collector.collect(&()))
}
