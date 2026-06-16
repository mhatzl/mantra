use std::path::Path;

use mantra_schema::{
    path::{PathExt, RelativePathBuf},
    report::nav::ReportNavigationSchema,
};

use crate::cmd::report::{
    cfg::{ReportConfig, ReportFormat},
    templates::{MantraTemplates, TemplateName},
};

pub struct ReportWriter<'templates> {
    base_path: std::path::PathBuf,
    nav: ReportNavigationSchema,
    cfg: ReportConfig,
    templates: MantraTemplates<'templates>,
}

impl<'templates> ReportWriter<'templates> {
    pub fn new(
        base_path: std::path::PathBuf,
        nav: ReportNavigationSchema,
        cfg: ReportConfig,
        templates: MantraTemplates<'templates>,
    ) -> Self {
        Self {
            base_path,
            nav,
            cfg,
            templates,
        }
    }

    pub async fn write_file<T: serde::Serialize>(
        &self,
        filepath: &std::path::Path,
        schema: T,
        template_name: TemplateName,
    ) -> Result<(), anyhow::Error> {
        let mut rel_path = self.base_path.relative_to(filepath)?;
        if let Some(parent) = rel_path.parent() {
            rel_path = parent.to_relative_path_buf();
        }
        if rel_path.as_str() == "" {
            rel_path = RelativePathBuf::from(".");
        }

        let context = serde_json::json!({
            "nav": self.nav,
            "path_to_root": rel_path,
            "schema": schema
        });

        for format in self.cfg.formats() {
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

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}
