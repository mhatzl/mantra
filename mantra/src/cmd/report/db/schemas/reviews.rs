use mantra_schema::{
    SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        review::{ReviewReference, ReviewState},
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
        select product_id, review_name, state
        from ReviewStates
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

    let mut valid = Vec::with_capacity(review_records.len());
    let mut obsolete = Vec::new();

    for record in review_records {
        let state = ReviewState::try_from(record.state)?;
        let review_ref = ReviewReference {
            product_id: product.id.clone(),
            name: record.review_name,
            state,
        };

        match state {
            ReviewState::Obsolete => {
                summary.obsolete.cnt += 1;
                obsolete.push(review_ref);
            }
            ReviewState::Valid => {
                summary.valid.cnt += 1;
                valid.push(review_ref);
            }
        }
    }

    summary.update_percentages();

    Ok(ReviewsReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        reviews: ReviewsOverview {
            summary,
            valid,
            obsolete,
        },
    })
}
