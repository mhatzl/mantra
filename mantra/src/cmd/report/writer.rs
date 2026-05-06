use std::collections::HashSet;

use mantra_schema::report::nav::ReportNavigationSchema;

use crate::cmd::report::{
    cfg::ReportFormat,
    templates::{MantraTemplates, TemplateName},
};

pub struct ReportWriter<'templates> {
    nav: ReportNavigationSchema,
    formats: HashSet<ReportFormat>,
    templates: MantraTemplates<'templates>,
}

impl<'templates> ReportWriter<'templates> {
    pub fn new(
        nav: ReportNavigationSchema,
        formats: HashSet<ReportFormat>,
        templates: MantraTemplates<'templates>,
    ) -> Self {
        Self {
            nav,
            formats,
            templates,
        }
    }

    pub async fn write_file<T: serde::Serialize>(
        &self,
        filepath: &std::path::Path,
        schema: T,
        template_name: TemplateName,
    ) -> Result<(), anyhow::Error> {
        let context = serde_json::json!({
            "nav": self.nav,
            "schema": schema
        });

        for format in &self.formats {
            let content = if format == &ReportFormat::Json {
                json5::to_string(&schema)?
            } else {
                self.templates.render(&template_name, &format, &context)?
            };
            let mut path = filepath.to_path_buf();
            path.set_extension(format.as_extension());

            tokio::fs::write(path, content).await?;
        }

        Ok(())
    }
}
