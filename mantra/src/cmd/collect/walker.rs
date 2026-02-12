use std::path::PathBuf;

use ignore::{
    WalkBuilder,
    types::{Types, TypesBuilder},
};

pub(super) fn base_mantra_walker(start_path: PathBuf) -> WalkBuilder {
    let mut builder = WalkBuilder::new(start_path);
    builder.add_custom_ignore_filename(".mantraignore");
    builder
}

pub(super) fn base_schema_types() -> Result<Types, anyhow::Error> {
    let mut builder = TypesBuilder::new();
    builder.add("json", "*.json")?;
    builder.add("json5", "*.json5")?;
    builder.add("toml", "*.toml")?;
    Ok(builder.build()?)
}

pub(super) fn content_to_schema<T: serde::de::DeserializeOwned>(
    extension: &str,
    content: &str,
) -> Result<T, anyhow::Error> {
    println!("in content to schema");
    match extension {
        "toml" => Ok(toml::from_str::<T>(content)?),
        // JSON5 is a superset of JSON, so JSON files are also accepted by JSON5
        "json" | "json5" => Ok(json5::from_str::<T>(content)?),
        _ => Ok(json5::from_str::<T>(content).inspect_err(|_| {
            eprintln!(
                "Tried to read content from unsupported extension '{}'",
                extension
            )
        })?),
    }
}
