use std::path::PathBuf;

use anyhow::bail;
use glob::Pattern;
use ignore::{
    WalkBuilder,
    types::{Types, TypesBuilder},
};
use mantra_schema::path::RelativePathBuf;

use crate::cmd::collect::collector::CollectableFile;

pub(super) fn base_mantra_walker(
    start_path: PathBuf,
    glob_pattern: Option<Pattern>,
) -> WalkBuilder {
    let mut builder = WalkBuilder::new(start_path);
    builder.add_custom_ignore_filename(".mantraignore");

    if let Some(pattern) = glob_pattern {
        builder.filter_entry(move |entry| {
            entry.path().is_dir()
                || match RelativePathBuf::from_path(entry.path()) {
                    Ok(rel_path) => pattern.matches(rel_path.as_str()),
                    Err(_) => false,
                }
        });
    }

    builder
}

pub(super) fn base_schema_types() -> Result<Types, anyhow::Error> {
    let mut builder = TypesBuilder::new();
    builder.add("json", "*.json")?;
    builder.select("json");
    builder.add("json5", "*.json5")?;
    builder.select("json5");
    builder.add("toml", "*.toml")?;
    builder.select("toml");
    Ok(builder.build()?)
}

pub(super) fn content_to_schema<T: serde::de::DeserializeOwned>(
    file: &CollectableFile,
) -> Result<T, anyhow::Error> {
    match file.extension() {
        Some("toml") => Ok(toml::from_str::<T>(file.content)?),
        // JSON5 is a superset of JSON, so JSON files are also accepted by JSON5
        Some("json") | Some("json5") => Ok(json5::from_str::<T>(file.content)?),
        Some(extension) => Ok(json5::from_str::<T>(file.content).inspect_err(|_| {
            eprintln!(
                "Tried to read content from unsupported extension '{}'",
                extension
            )
        })?),
        None => bail!("No extension to determine collector."),
    }
}
