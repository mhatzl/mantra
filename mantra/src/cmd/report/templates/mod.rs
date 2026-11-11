use anyhow::{Context, bail};
use minijinja::Environment;
use tokio_stream::{StreamExt, wrappers::ReadDirStream};

use crate::cmd::report::cfg::ReportFormat;

mod functions;

pub struct MantraTemplates<'templates> {
    environment: minijinja::Environment<'templates>,
}

impl<'templates> MantraTemplates<'templates> {
    pub fn new() -> Result<Self, anyhow::Error> {
        let mut environment = Environment::new();

        environment.add_template(
            TemplateName::BaseLayout.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/base-layout.html"),
        )?;
        environment.add_template(
            TemplateName::BaseStyle.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/base-style.html"),
        )?;
        environment.add_template(
            TemplateName::EvidenceMatrix.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/evidence-matrix.html"),
        )?;
        environment.add_template(
            TemplateName::Nav.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/nav.html"),
        )?;
        environment.add_template(
            TemplateName::Product.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/product.html"),
        )?;
        environment.add_template(
            TemplateName::Products.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/products.html"),
        )?;
        environment.add_template(
            TemplateName::Requirement.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/requirement.html"),
        )?;
        environment.add_template(
            TemplateName::RequirementOrigin.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/requirement-origin.html"),
        )?;
        environment.add_template(
            TemplateName::Requirements.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/requirements.html"),
        )?;
        environment.add_template(
            TemplateName::Review.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/review.html"),
        )?;
        environment.add_template(
            TemplateName::Reviews.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/reviews.html"),
        )?;
        environment.add_template(
            TemplateName::TestCase.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/test-case.html"),
        )?;
        environment.add_template(
            TemplateName::TestRun.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/test-run.html"),
        )?;
        environment.add_template(
            TemplateName::TestRuns.template_name_for_format(&ReportFormat::Html),
            include_str!("defaults/test-runs.html"),
        )?;

        environment.add_function("product_url_path", functions::product_url_path);
        environment.add_function("requirement_url_path", functions::requirement_url_path);
        environment.add_function("review_url_path", functions::review_url_path);
        environment.add_function("test_run_url_path", functions::test_run_url_path);
        environment.add_function("test_case_url_path", functions::test_case_url_path);
        environment.add_function("format_html", functions::format_html);
        environment.add_function("formatted_datetime", functions::formatted_datetime);
        environment.add_function(
            "formatted_datetime_in_system_timezone",
            functions::formatted_datetime_in_system_timezone,
        );

        Ok(Self { environment })
    }

    pub async fn custom_templates(&mut self, dir: &std::path::Path) -> Result<(), anyhow::Error> {
        let read_dir = tokio::fs::read_dir(dir).await?;
        let mut dir_stream = ReadDirStream::new(read_dir);

        while let Some(res) = dir_stream.next().await {
            if let Ok(dir_entry) = res
                && let Some(extension) = dir_entry.path().extension()
                && ReportFormat::try_from(extension).is_ok()
                && let Ok(filename) = dir_entry.file_name().into_string()
                && let Ok(src) = crate::io::async_read_encoding_independent(dir_entry.path()).await
            {
                self.environment
                    .add_template_owned(filename, src)
                    .with_context(|| {
                        format!(
                            "Failed adding custom template '{}'",
                            dir_entry.path().display(),
                        )
                    })?;
            }
        }

        Ok(())
    }

    pub fn render<T: serde::Serialize>(
        &self,
        template_name: &TemplateName,
        format: &ReportFormat,
        context: T,
    ) -> Result<String, anyhow::Error> {
        if format == &ReportFormat::Json {
            bail!("JSON format does not allow templates!");
        }

        let resolved_template_name = template_name.template_name_for_format(format);
        let template = self.environment.get_template(resolved_template_name)?;
        template
            .render(context)
            .with_context(|| format!("Failed rendering template '{}'", resolved_template_name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateName {
    BaseLayout,
    BaseStyle,
    EvidenceMatrix,
    Nav,
    Product,
    Products,
    Requirement,
    RequirementOrigin,
    Requirements,
    Review,
    Reviews,
    SourceFile,
    SourceFolder,
    Sources,
    TestCase,
    TestRun,
    TestRuns,
}

impl TemplateName {
    const fn template_name_for_format(&self, format: &ReportFormat) -> &'static str {
        macro_rules! template_format_concat {
            ($name:literal, $format:ident) => {
                match $format {
                    ReportFormat::Html => concat!($name, ".", "html"),
                    ReportFormat::Markdown => concat!($name, ".", "md"),
                    ReportFormat::Json => concat!($name, ".", "json5"),
                }
            };
        }

        match self {
            TemplateName::BaseLayout => template_format_concat!("base-layout", format),
            TemplateName::BaseStyle => template_format_concat!("base-style", format),
            TemplateName::EvidenceMatrix => template_format_concat!("evidence-matrix", format),
            TemplateName::Nav => template_format_concat!("nav", format),
            TemplateName::Product => template_format_concat!("product", format),
            TemplateName::Products => template_format_concat!("products", format),
            TemplateName::Requirement => template_format_concat!("requirement", format),
            TemplateName::RequirementOrigin => {
                template_format_concat!("requirement-origin", format)
            }
            TemplateName::Requirements => template_format_concat!("requirements", format),
            TemplateName::Review => template_format_concat!("review", format),
            TemplateName::Reviews => template_format_concat!("reviews", format),
            TemplateName::SourceFile => template_format_concat!("source-file", format),
            TemplateName::SourceFolder => template_format_concat!("source-folder", format),
            TemplateName::Sources => template_format_concat!("sources", format),
            TemplateName::TestCase => template_format_concat!("test-case", format),
            TemplateName::TestRun => template_format_concat!("test-run", format),
            TemplateName::TestRuns => template_format_concat!("test-runs", format),
        }
    }
}
