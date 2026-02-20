use mantra_schema::{annotations::AnnotationSchema, path::RelativePath};

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
            AnnotationSourceVariant::Content => Ok(|file: &CollectableFile| todo!()),
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
