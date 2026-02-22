use mantra_lang_tracing::collect::collector::AnnotationCollector;
use mantra_schema::{
    annotations::{AnnotationSchema, Annotations, FileAnnotations},
    path::RelativePath,
};

use crate::cmd::collect::{
    cfg::{AnnotationSourceVariant, CollectAnnotationsConfig},
    collector::{CollectableFile, SingleFileCollectable},
    walker,
};

pub mod db;

impl<'db> SingleFileCollectable<'db, AnnotationSchema> for CollectAnnotationsConfig {
    fn path(&self) -> &mantra_schema::path::RelativePath {
        &self.path
    }

    fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    fn custom_ignore_filename(&self) -> &'static str {
        ".mantraignore-annotations"
    }

    fn modify_walker(&self, builder: &mut ignore::WalkBuilder) -> Result<(), anyhow::Error> {
        match self.source {
            AnnotationSourceVariant::Content => {
                // CI configuration is often in dot-folder which is considered "hidden"
                // We want to detect traces in CI configuration files.
                builder.hidden(false);
            }
            AnnotationSourceVariant::Schema => {
                builder.types(walker::base_schema_types()?);
            }
        }

        Ok(())
    }

    fn collect_fn(
        &self,
    ) -> Result<fn(&CollectableFile) -> Result<AnnotationSchema, anyhow::Error>, anyhow::Error>
    {
        match self.source {
            AnnotationSourceVariant::Content => Ok(collect_from_content),
            AnnotationSourceVariant::Schema => Ok(walker::content_to_schema::<AnnotationSchema>),
        }
    }

    async fn update_db(
        collection: &mut super::Collection<'db>,
        filepath: &RelativePath,
        schema: AnnotationSchema,
    ) -> Result<(), anyhow::Error> {
        collection
            .update_per_annotation_schema(filepath, schema)
            .await
    }
}

fn collect_from_content(file: &CollectableFile) -> Result<AnnotationSchema, anyhow::Error> {
    if file.extension() == Some("rs") {
        let annotations =
            mantra_lang_tracing::collect::rust::RustCodeCollector::collect(file.content)?;
        Ok(AnnotationSchema {
            version: None,
            files: vec![FileAnnotations {
                filepath: file.filepath.clone(),
                file_hash: file.file_hash.clone(),
                annotations,
            }],
            trace_properties: None,
            origin: None,
        })
    } else {
        eprintln!(
            "Got unsupported file type to collect annotations from '{}'. No traces or elements are collected.",
            file.filepath
        );

        Ok(AnnotationSchema {
            version: None,
            files: vec![FileAnnotations {
                filepath: file.filepath.clone(),
                file_hash: file.file_hash.clone(),
                annotations: Annotations {
                    traces: vec![],
                    elements: vec![],
                    coverage_excludes: vec![],
                },
            }],
            trace_properties: None,
            origin: None,
        })
    }
}
