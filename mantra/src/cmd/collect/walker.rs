use std::path::PathBuf;

use anyhow::bail;
use glob::Pattern;
use ignore::{
    WalkBuilder,
    types::{Types, TypesBuilder},
};
use mantra_schema::{
    annotations::AnnotationSchema, product::ProductId, requirements::RequirementSchema,
    reviews::ReviewSchema, test_runs::TestRunSchema,
};

use crate::cmd::collect::collector::CollectableFile;

pub(super) fn base_mantra_walker(
    start_path: PathBuf,
    glob_pattern: Option<Pattern>,
) -> WalkBuilder {
    let mut builder = WalkBuilder::new(&start_path);
    builder.add_custom_ignore_filename(".mantraignore");

    if let Some(pattern) = glob_pattern {
        builder
            .filter_entry(move |entry| entry.path().is_dir() || pattern.matches_path(entry.path()));
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

pub(super) fn content_to_schema<T: MantraSchema + serde::de::DeserializeOwned>(
    product_id: &ProductId,
    file: &CollectableFile,
) -> Result<Option<T>, anyhow::Error> {
    let schema: T = match file.extension() {
        Some("toml") => toml::from_str::<T>(file.content)?,
        // JSON5 is a superset of JSON, so JSON files are also accepted by JSON5
        Some("json") | Some("json5") => json5::from_str::<T>(file.content)?,
        Some(extension) => json5::from_str::<T>(file.content).inspect_err(|_| {
            log::warn!(
                "Tried to read content from unsupported extension '{}'",
                extension
            )
        })?,
        None => bail!("No extension to determine collector."),
    };

    let opt_related_product = schema.related_product();

    if opt_related_product.is_none() || opt_related_product == Some(product_id) {
        Ok(Some(schema))
    } else {
        Ok(None)
    }
}

trait MantraSchema {
    fn related_product(&self) -> Option<&ProductId>;
}

impl MantraSchema for AnnotationSchema {
    fn related_product(&self) -> Option<&ProductId> {
        self.product_id.as_ref()
    }
}

impl MantraSchema for RequirementSchema {
    fn related_product(&self) -> Option<&ProductId> {
        self.product_id.as_ref()
    }
}

impl MantraSchema for ReviewSchema {
    fn related_product(&self) -> Option<&ProductId> {
        self.product_id.as_ref()
    }
}

impl MantraSchema for TestRunSchema {
    fn related_product(&self) -> Option<&ProductId> {
        self.product_id.as_ref()
    }
}
