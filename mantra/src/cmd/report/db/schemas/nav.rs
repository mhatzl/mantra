use mantra_schema::{
    SCHEMA_VERSION,
    report::{nav::ReportNavigationSchema, product::ProductReportSchema},
};

use crate::db::MantraTransaction;

pub async fn generate_navigation_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    products: &[ProductReportSchema],
) -> Result<ReportNavigationSchema, anyhow::Error> {
    // TODO

    Ok(ReportNavigationSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        products: Vec::new(),
        root_sources: Vec::new(),
    })
}
