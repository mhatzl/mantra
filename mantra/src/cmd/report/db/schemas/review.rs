use std::str::FromStr;

use mantra_schema::{
    Revision, SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        review::{ReviewReference, ReviewReportSchema, VerifiedRequirement},
    },
    reviews::{
        OverrideCoveredLineInfo, OverrideFileCoverage, OverrideTestCase, OverrideTestCaseState,
        OverrideTestRun,
    },
};

use crate::db::MantraTransaction;

pub async fn generate_review_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    review: &ReviewReference,
) -> Result<ReviewReportSchema, anyhow::Error> {
    let metadata = sqlx::query!(
        r#"
        select
            gt.content as "description?",
            og.content as "origin?",
            bo.content as "base_origin?"
        from Reviews r
            left join GeneralTexts gt on r.description_hash = gt.hash
            left join GeneralJson og on r.origin_hash = og.hash
            left join GeneralJson bo on r.base_origin_hash = bo.hash
        where r.product_id = $1 and r.name = $2 and r.utc_date = $3
        "#,
        product.id,
        review.name,
        review.utc_date
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let origin = match metadata.origin {
        Some(o) => Some(serde_json::Value::from_str(&o)?),
        None => None,
    };
    let base_origin = match metadata.base_origin {
        Some(o) => Some(serde_json::Value::from_str(&o)?),
        None => None,
    };

    let authors: Vec<_> = sqlx::query!(
        "
        select author
        from ReviewAuthors
        where product_id = $1 and review_name = $2
        and review_date = $3
        ",
        product.id,
        review.name,
        review.utc_date
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|a| a.author)
    .collect();

    let revision_records = sqlx::query!(
        "
        select revision, comment
        from ReviewRevisions
        where product_id = $1 and review_name = $2
        and review_date = $3
        ",
        product.id,
        review.name,
        review.utc_date
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let revisions = if revision_records.is_empty() {
        None
    } else {
        let mut revisions = Vec::with_capacity(revision_records.len());

        for rev in revision_records {
            let authors = sqlx::query!(
                "
                select author
                from ReviewRevisionAuthors
                where product_id = $1 and review_name = $2
                and review_date = $3 and revision = $4
                ",
                product.id,
                review.name,
                review.utc_date,
                rev.revision
            )
            .fetch_all(transaction.as_mut())
            .await?
            .into_iter()
            .map(|a| a.author)
            .collect();

            revisions.push(Revision {
                nr: rev.revision,
                authors,
                comment: rev.comment,
            });
        }

        Some(revisions)
    };

    let property_records = sqlx::query!(
        "
        select rp.property_key, v.content
        from ReviewProperties rp left join GeneralJson v on rp.value_hash = v.hash
        where rp.product_id = $1 and rp.review_name = $2
        and rp.review_date = $3
        ",
        product.id,
        review.name,
        review.utc_date
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let properties = if property_records.is_empty() {
        None
    } else {
        let mut properties = serde_json::value::Map::with_capacity(property_records.len());

        for prop in property_records {
            properties.insert(
                prop.property_key,
                serde_json::Value::from_str(&prop.content)?,
            );
        }

        Some(properties)
    };

    let requirements: Vec<VerifiedRequirement> = sqlx::query!(
        "
        select vr.req_id, gt.content
        from ManuallyVerifiedRequirements vr left join GeneralTexts gt on vr.comment_hash = gt.hash
        where vr.product_id = $1
        and vr.review_name = $2
        and vr.review_date = $3",
        product.id,
        review.name,
        review.utc_date
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| VerifiedRequirement {
        id: r.req_id,
        comment: r.content,
    })
    .collect();
    let requirements = if requirements.is_empty() {
        None
    } else {
        Some(requirements)
    };

    let test_run_overrides = review_test_run_overrides(transaction, product, review).await?;
    let test_run_overrides = if test_run_overrides.is_empty() {
        None
    } else {
        Some(test_run_overrides)
    };

    Ok(ReviewReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        name: review.name.clone(),
        utc_date: review.utc_date,
        state: review.state,
        authors,
        description: metadata.description,
        origin,
        base_origin,
        properties,
        revisions,
        requirements,
        test_run_overrides,
    })
}

async fn review_test_run_overrides<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    review: &ReviewReference,
) -> Result<Vec<OverrideTestRun>, anyhow::Error> {
    let overriden_test_runs = sqlx::query!(
        "
        select test_run_name, test_run_date
        from TestCaseOverrides
        where product_id = $1
        and review_name = $2
        and review_date = $3

        union

        select test_run_name, test_run_date
        from TestRunLineCoverageOverrides
        where product_id = $1
        and review_name = $2
        and review_date = $3

        union

        select test_run_name, test_run_date
        from TestCaseLineCoverageOverrides
        where product_id = $1
        and review_name = $2
        and review_date = $3
        ",
        product.id,
        review.name,
        review.utc_date
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut test_run_overrides = Vec::with_capacity(overriden_test_runs.len());

    for test_run in overriden_test_runs {
        let overriden_test_cases = sqlx::query!(
            "
            select test_case_name
            from TestCaseOverrides
            where product_id = $1
            and test_run_name = $2
            and test_run_date = $3

            union

            select test_case_name
            from TestCaseLineCoverageOverrides
            where product_id = $1
            and test_run_name = $2
            and test_run_date = $3
            ",
            product.id,
            test_run.test_run_name,
            test_run.test_run_date
        )
        .fetch_all(transaction.as_mut())
        .await?;

        let mut test_cases = Vec::with_capacity(overriden_test_cases.len());

        for test_case in overriden_test_cases {
            let overriden_state = sqlx::query!(
                "
                select state, content
                from TestCaseOverrides, GeneralTexts
                where product_id = $1
                and test_run_name = $2
                and test_run_date = $3
                and test_case_name = $4
                and comment_hash = hash
                ",
                product.id,
                test_run.test_run_name,
                test_run.test_run_date,
                test_case.test_case_name
            )
            .fetch_optional(transaction.as_mut())
            .await?;

            let overriden_files = sqlx::query!(
                "
                select cov_filepath
                from TestCaseLineCoverageOverrides
                where product_id = $1
                and test_run_name = $2
                and test_run_date = $3
                and test_case_name = $4
                ",
                product.id,
                test_run.test_run_name,
                test_run.test_run_date,
                test_case.test_case_name
            )
            .fetch_all(transaction.as_mut())
            .await?;

            let mut coverage = Vec::with_capacity(overriden_files.len());

            for filepath in overriden_files {
                let line_overrides = sqlx::query!(
                    "
                    select cov_line, hits, content
                    from TestCaseLineCoverageOverrides, GeneralTexts
                    where product_id = $1
                    and test_run_name = $2
                    and test_run_date = $3
                    and test_case_name = $4
                    and cov_filepath = $5
                    and comment_hash = hash
                    ",
                    product.id,
                    test_run.test_run_name,
                    test_run.test_run_date,
                    test_case.test_case_name,
                    filepath.cov_filepath
                )
                .fetch_all(transaction.as_mut())
                .await?;

                coverage.push(OverrideFileCoverage {
                    filepath: filepath.cov_filepath.into(),
                    lines: line_overrides
                        .into_iter()
                        .map(|l| OverrideCoveredLineInfo {
                            nrs: vec![l.cov_line],
                            hits: l.hits,
                            comment: l.content,
                        })
                        .collect(),
                })
            }

            test_cases.push(OverrideTestCase {
                name: test_case.test_case_name,
                state: overriden_state.map(|s| OverrideTestCaseState {
                    new: s.state.try_into().expect("Valid state in database"),
                    comment: s.content,
                }),
                coverage,
            });
        }

        let mut coverage = Vec::with_capacity(
            sqlx::query!(
                r#"
                select count(*) as "len" from TestRunLineCoverageOverrides
                where product_id = $1
                and test_run_name = $2
                and test_run_date = $3
                "#,
                product.id,
                test_run.test_run_name,
                test_run.test_run_date,
            )
            .fetch_one(transaction.as_mut())
            .await?
            .len
            .try_into()?,
        );

        let overriden_files = sqlx::query!(
            "
            select cov_filepath
            from TestRunLineCoverageOverrides
            where product_id = $1
            and test_run_name = $2
            and test_run_date = $3
            ",
            product.id,
            test_run.test_run_name,
            test_run.test_run_date
        )
        .fetch_all(transaction.as_mut())
        .await?;

        for filepath in overriden_files {
            let line_overrides = sqlx::query!(
                "
                select cov_line, hits, content
                from TestRunLineCoverageOverrides, GeneralTexts
                where product_id = $1
                and test_run_name = $2
                and test_run_date = $3
                and cov_filepath = $4
                and comment_hash = hash
                ",
                product.id,
                test_run.test_run_name,
                test_run.test_run_date,
                filepath.cov_filepath
            )
            .fetch_all(transaction.as_mut())
            .await?;

            coverage.push(OverrideFileCoverage {
                filepath: filepath.cov_filepath.into(),
                lines: line_overrides
                    .into_iter()
                    .map(|l| OverrideCoveredLineInfo {
                        nrs: vec![l.cov_line],
                        hits: l.hits,
                        comment: l.content,
                    })
                    .collect(),
            })
        }

        test_run_overrides.push(OverrideTestRun {
            name: test_run.test_run_name,
            utc_date: mantra_schema::test_runs::test_date_from_str(&test_run.test_run_date)
                .expect("Valid test date in database"),
            test_cases,
            coverage,
        })
    }

    Ok(test_run_overrides)
}
