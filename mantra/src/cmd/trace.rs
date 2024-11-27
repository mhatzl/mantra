use std::{
    io::Read,
    path::{Path, PathBuf},
};

use crate::db::{MantraDb, TraceChanges};

use ignore::{types::TypesBuilder, WalkBuilder};
use mantra_lang_tracing::{
    collect::{AstCollector, PlainCollector, TraceCollector},
    lsif_graph::LsifGraph,
    path::SlashPathBuf,
};
use mantra_schema::traces::{TraceEntry, TraceSchema};

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
    #[arg(long)]
    #[serde(default, alias = "lsif-data")]
    pub lsif_data: Option<Vec<PathBuf>>,
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

    let mut lsif_graphs = Vec::new();

    if let Some(lsif_files) = &cfg.lsif_data {
        for lsif_data in lsif_files {
            let raw_content = tokio::fs::read(lsif_data).await.map_err(|err| {
                log::error!("{err}");
                TraceError::CouldNotAccessFile(lsif_data.to_string_lossy().to_string())
            })?;
            // decoding is needed, because LSIF-JSON may be encoded in UTF-8 or UTF-16
            let mut decoder = encoding_rs_io::DecodeReaderBytes::new(raw_content.as_slice());
            let mut content = String::with_capacity(raw_content.len());
            let _ = decoder.read_to_string(&mut content);

            let graph = mantra_lang_tracing::lsif_graph::LsifGraph::create(&content)
                .map_err(TraceError::Deserialize)?;
            lsif_graphs.push(graph);
        }
    }

    let lsif_graphs = if lsif_graphs.is_empty() {
        None
    } else {
        Some(lsif_graphs)
    };

    if cfg.root.is_dir() || cfg.root == PathBuf::from("") || cfg.root == PathBuf::from("./") {
        let root = if cfg.root == PathBuf::from("") || cfg.root == PathBuf::from("./") {
            std::env::current_dir().expect("Current directory must be valid.")
        } else {
            cfg.root.clone()
        };

        let walk = WalkBuilder::new(&root)
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
                let filepath = if cfg.keep_path_absolute {
                    dir_entry.clone().into_path()
                } else {
                    mantra_lang_tracing::path::make_relative(dir_entry.path(), &root)
                        .unwrap_or(dir_entry.clone().into_path())
                };

                if let Some(traces) =
                    collect_traces(dir_entry.path(), filepath.clone().into(), &lsif_graphs)?
                {
                    let mut trace_changes = db
                        .add_traces(&filepath, &traces, new_generation)
                        .await
                        .map_err(TraceError::DbError)?;

                    changes.merge(&mut trace_changes);
                }
            }
        }

        Ok(changes)
    } else {
        let filepath = if cfg.keep_path_absolute {
            cfg.root.to_path_buf()
        } else {
            mantra_lang_tracing::path::make_relative(&cfg.root, &cfg.root)
                .unwrap_or(cfg.root.to_path_buf())
        };

        if let Some(traces) = collect_traces(&cfg.root, filepath.clone().into(), &lsif_graphs)? {
            db.add_traces(&filepath, &traces, new_generation)
                .await
                .map_err(TraceError::DbError)
        } else {
            Ok(changes)
        }
    }
}

fn collect_traces(
    abs_filepath: &Path,
    rel_filepath: SlashPathBuf,
    lsif_graphs: &Option<Vec<LsifGraph>>,
) -> Result<Option<Vec<TraceEntry>>, TraceError> {
    let is_textfile = mime_guess::from_path(abs_filepath)
        .first()
        .map(|mime| mime.type_() == "text")
        .unwrap_or(false);

    if !is_textfile {
        // Traces are only collected from text files
        return Ok(None);
    }

    let content = std::fs::read_to_string(abs_filepath)
        .map_err(|_| TraceError::CouldNotAccessFile(abs_filepath.to_string_lossy().to_string()))?;

    let extension_str = abs_filepath
        .extension()
        .map(|osstr| osstr.to_str().unwrap_or_default());

    if extension_str == Some("rs") {
        match AstCollector::new(
            content.as_bytes(),
            &tree_sitter_rust::language(),
            rel_filepath.to_string(),
            Box::new(mantra_rust_trace::collect_traces_in_rust),
        ) {
            Some(mut collector) => {
                return Ok(collector.collect(lsif_graphs));
            }
            None => {
                log::warn!(
                    "Failed parsing Rust code. File content taken as plain text: {}",
                    abs_filepath.display()
                );
            }
        }
    }

    let mut collector = PlainCollector::new(&content);
    Ok(collector.collect(&()))
}
