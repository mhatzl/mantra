use mantra_schema::{
    SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        review::ReviewReference,
        reviews::{ReviewsOverview, ReviewsReportSchema, ReviewsSummary},
    },
};

use crate::db::MantraTransaction;

pub async fn generate_reviews_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<ReviewsReportSchema, anyhow::Error> {
    let review_records = sqlx::query!(
        "
        select product_id, name, utc_date
        from Reviews
        where product_id = $1
        ",
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut summary = ReviewsSummary {
        total: review_records.len() as i64,
        ..Default::default()
    };
    summary.obsolete.cnt = 0;
    summary.valid.cnt = summary.total;

    let mut reviews = Vec::with_capacity(review_records.len());

    for record in review_records {
        reviews.push(ReviewReference {
            product_id: product.id.clone(),
            name: record.name,
            utc_date: mantra_schema::reviews::date_from_str(&record.utc_date)?,
            state: mantra_schema::report::review::ReviewState::Valid, // TODO: set correct state
        });
    }

    summary.update_percentages();

    Ok(ReviewsReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        reviews: ReviewsOverview {
            summary,
            valid: reviews,
            obsolete: Vec::new(), // TODO: split depending on state
        },
    })
}
