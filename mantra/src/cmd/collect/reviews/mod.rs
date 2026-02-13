use ignore::types::TypesBuilder;
use mantra_schema::reviews::ReviewSchema;

use crate::cmd::collect::{
    cfg::{CollectReviewsConfig, ReviewSourceVariant},
    collector::SingleFileCollectable,
    walker,
};

pub mod db;

impl<'db> SingleFileCollectable<'db, ReviewSchema> for CollectReviewsConfig {
    fn path(&self) -> &mantra_schema::path::RelativePath {
        &self.path
    }

    fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    fn custom_ignore_filename(&self) -> &'static str {
        ".mantraignore-reviews"
    }

    fn modify_walker(&self, builder: &mut ignore::WalkBuilder) -> Result<(), anyhow::Error> {
        match self.source {
            ReviewSourceVariant::Markup => {
                builder.types(
                    TypesBuilder::new()
                        .add_defaults()
                        .select("markdown")
                        .build()?,
                );
            }
            ReviewSourceVariant::Schema => {
                builder.types(walker::base_schema_types()?);
            }
        }

        Ok(())
    }

    fn collect_fn(
        &self,
    ) -> Result<fn(&str, &str) -> Result<ReviewSchema, anyhow::Error>, anyhow::Error> {
        match self.source {
            ReviewSourceVariant::Markup => Ok(|extension: &str, content: &str| todo!()),
            ReviewSourceVariant::Schema => Ok(walker::content_to_schema::<ReviewSchema>),
        }
    }

    async fn update_db(
        collection: &mut super::Collection<'db>,
        schema: ReviewSchema,
    ) -> Result<(), anyhow::Error> {
        collection.update_per_review_schema(schema).await
    }
}
