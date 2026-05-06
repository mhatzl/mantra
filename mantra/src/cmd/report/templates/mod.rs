use anyhow::bail;
use minijinja::Environment;

use crate::cmd::report::cfg::ReportFormat;

pub struct MantraTemplates<'templates> {
    environment: minijinja::Environment<'templates>,
}

impl<'templates> MantraTemplates<'templates> {
    pub fn new() -> Result<Self, anyhow::Error> {
        let mut environment = Environment::new();

        environment.add_template(
            TemplateName::EVIDENCE_MATRIX_HTML,
            include_str!("defaults/evidence_matrix.html"),
        )?;
        environment.add_template(
            TemplateName::PRODUCT_HTML,
            include_str!("defaults/product.html"),
        )?;
        environment.add_template(
            TemplateName::PRODUCTS_HTML,
            include_str!("defaults/products.html"),
        )?;

        Ok(Self { environment })
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

        let template = self
            .environment
            .get_template(&template_name.template_name_for_format(format))?;
        Ok(template.render(context)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateName {
    EvidenceMatrix,
    Nav,
    Product,
    Products,
    Requirement,
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

impl std::fmt::Display for TemplateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateName::EvidenceMatrix => write!(f, "evidence-matrix"),
            TemplateName::Nav => write!(f, "nav"),
            TemplateName::Product => write!(f, "product"),
            TemplateName::Products => write!(f, "products"),
            TemplateName::Requirement => write!(f, "requirement"),
            TemplateName::Requirements => write!(f, "requirements"),
            TemplateName::Review => write!(f, "review"),
            TemplateName::Reviews => write!(f, "reviews"),
            TemplateName::SourceFile => write!(f, "source-file"),
            TemplateName::SourceFolder => write!(f, "source-folder"),
            TemplateName::Sources => write!(f, "sources"),
            TemplateName::TestCase => write!(f, "test-case"),
            TemplateName::TestRun => write!(f, "test-run"),
            TemplateName::TestRuns => write!(f, "test-runs"),
        }
    }
}

impl TemplateName {
    const EVIDENCE_MATRIX_HTML: &str = "evidence-matrix::html";
    const EVIDENCE_MATRIX_MD: &str = "evidence-matrix::md";
    const NAV_HTML: &str = "nav::html";
    const PRODUCT_HTML: &str = "product::html";
    const PRODUCTS_HTML: &str = "product::html";
    const REQUIREMENT_HTML: &str = "requirement::html";

    fn template_name_for_format(&self, format: &ReportFormat) -> String {
        format!("{}::{}", self, format)
    }
}
