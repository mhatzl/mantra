use ignore::types::TypesBuilder;
use mantra_schema::{path::RelativePath, requirements::RequirementSchema};

use crate::cmd::collect::{
    cfg::{CollectRequirementsConfig, RequirementSourceVariant},
    collector::{CollectableFile, SingleFileCollectable},
    walker,
};

pub mod db;
pub mod markup;

#[cfg(test)]
mod tests;

impl<'db> SingleFileCollectable<'db, RequirementSchema> for CollectRequirementsConfig {
    fn path(&self) -> &mantra_schema::path::RelativePath {
        &self.path
    }

    fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    fn custom_ignore_filename(&self) -> &'static str {
        ".mantraignore-requirements"
    }

    fn modify_walker(&self, builder: &mut ignore::WalkBuilder) -> Result<(), anyhow::Error> {
        match self.source {
            RequirementSourceVariant::Markup => {
                builder.types(
                    TypesBuilder::new()
                        .add_defaults()
                        .select("markdown")
                        .build()?,
                );
            }
            RequirementSourceVariant::Schema => {
                builder.types(walker::base_schema_types()?);
            }
        }

        Ok(())
    }

    fn collect_fn(
        &self,
    ) -> Result<fn(&CollectableFile) -> Result<RequirementSchema, anyhow::Error>, anyhow::Error>
    {
        match self.source {
            RequirementSourceVariant::Markup => Ok(|file: &CollectableFile| todo!()),
            RequirementSourceVariant::Schema => Ok(walker::content_to_schema::<RequirementSchema>),
        }
    }

    async fn update_db(
        collection: &mut super::Collection<'db>,
        filepath: &RelativePath,
        schema: RequirementSchema,
    ) -> Result<(), anyhow::Error> {
        collection.update_per_req_schema(filepath, schema).await
    }
}
