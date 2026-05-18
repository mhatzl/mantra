use std::str::FromStr;

use mantra_schema::{
    FmtHash, Revision, SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        requirement::RequirementReference,
        review::{
            IgnoredEntries, IgnoredRequirement, IgnoredTestCaseLineCoverageOverride,
            IgnoredTestCaseStateOverride, IgnoredTestRunLineCoverageOverride,
            ResolvedOverrideCoveredLineInfo, ResolvedOverrideFileCoverage,
            ResolvedOverrideTestCase, ResolvedOverrideTestCaseState, ResolvedOverrideTestRun,
            ReviewReference, ReviewReportSchema, VerifiedRequirement,
        },
        test_case::TestCaseReference,
        test_run::TestRunReference,
        tests::TestState,
    },
    test_runs::TestCaseState,
};

use crate::{cmd::collect::reviews::db::DbIgnoredEntry, db::MantraTransaction};

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
        r#"
        select
            vr.req_id,
            rs.state,
            case
                when exists (
                    select id
                    from OptionalRequirements o
                    where o.product_id = $1
                    and o.id = vr.req_id
                ) then true
                else false
            end as "optional!:bool",
            gt.content
        from
            ManuallyVerifiedRequirements vr left join GeneralTexts gt on vr.comment_hash = gt.hash,
            RequirementVerificationStates rs
        where vr.product_id = $1 and rs.product_id = $1
        and vr.review_name = $2
        and vr.review_date = $3
        and vr.req_id = rs.id
        "#,
        product.id,
        review.name,
        review.utc_date
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| VerifiedRequirement {
        req: RequirementReference {
            product_id: product.id.clone(),
            id: r
                .req_id
                .try_into()
                .expect("Valid requirement ID in database"),
            state: r
                .state
                .try_into()
                .expect("Valid requirement state in database"),
            optional: r.optional,
        },
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

    let ignored_entries = review_ignored_entries(transaction, review).await?;

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
        ignored_entries,
    })
}

async fn review_test_run_overrides<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    review: &ReviewReference,
) -> Result<Vec<ResolvedOverrideTestRun>, anyhow::Error> {
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
            let old_state: TestCaseState = sqlx::query!(
                "
                select state
                from TestCases
                where product_id = $1
                and test_run_name = $2
                and test_run_date = $3
                and name = $4
                ",
                product.id,
                test_run.test_run_name,
                test_run.test_run_date,
                test_case.test_case_name
            )
            .fetch_one(transaction.as_mut())
            .await?
            .state
            .try_into()?;

            let resolved_state: TestState = sqlx::query!(
                "
                select state
                from ResolvedTestCaseStates
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
            .fetch_one(transaction.as_mut())
            .await?
            .state
            .try_into()?;

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
                select distinct cov_filepath
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
                    r#"
                    select
                        co.cov_line,
                        co.hits as new_hits,
                        tc.hits as old_hits,
                        gt.content
                    from
                        TestCaseLineCoverageOverrides co,
                        TestCaseLineCoverage tc,
                        GeneralTexts gt
                    where co.product_id = $1 and tc.product_id = $1
                    and co.test_run_name = $2 and tc.test_run_name = $2
                    and co.test_run_date = $3 and tc.test_run_date = $3
                    and co.test_case_name = $4 and tc.test_case_name = $4
                    and co.cov_filepath = $5 and tc.cov_filepath = $5
                    and co.cov_line = tc.cov_line
                    and co.comment_hash = gt.hash
                    "#,
                    product.id,
                    test_run.test_run_name,
                    test_run.test_run_date,
                    test_case.test_case_name,
                    filepath.cov_filepath
                )
                .fetch_all(transaction.as_mut())
                .await?;

                coverage.push(ResolvedOverrideFileCoverage {
                    filepath: filepath.cov_filepath.into(),
                    lines: line_overrides
                        .into_iter()
                        .map(|l| ResolvedOverrideCoveredLineInfo {
                            nr: l.cov_line,
                            new_hits: l.new_hits,
                            old_hits: l.old_hits,
                            comment: l.content,
                        })
                        .collect(),
                })
            }

            test_cases.push(ResolvedOverrideTestCase {
                test_case: TestCaseReference {
                    product_id: product.id.clone(),
                    test_run_name: test_run.test_run_name.clone(),
                    test_run_date: mantra_schema::test_runs::test_date_from_str(
                        &test_run.test_run_date,
                    )?,
                    test_case_name: test_case.test_case_name,
                    state: resolved_state,
                },
                state: overriden_state.map(|s| ResolvedOverrideTestCaseState {
                    new: s.state.try_into().expect("Valid state in database"),
                    old: old_state,
                    comment: s.content,
                }),
                coverage: if coverage.is_empty() {
                    None
                } else {
                    Some(coverage)
                },
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
            select distinct cov_filepath
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
                r#"
                select
                    co.cov_line,
                    co.hits as new_hits,
                    tr.hits as old_hits,
                    gt.content
                from
                    TestRunLineCoverageOverrides co,
                    TestRunLineCoverage tr,
                    GeneralTexts gt
                where co.product_id = $1 and tr.product_id = $1
                and co.test_run_name = $2 and tr.test_run_name = $2
                and co.test_run_date = $3 and tr.test_run_date = $3
                and co.cov_filepath = $4 and tr.cov_filepath = $4
                and co.cov_line = tr.cov_line
                and co.comment_hash = gt.hash
                "#,
                product.id,
                test_run.test_run_name,
                test_run.test_run_date,
                filepath.cov_filepath
            )
            .fetch_all(transaction.as_mut())
            .await?;

            coverage.push(ResolvedOverrideFileCoverage {
                filepath: filepath.cov_filepath.into(),
                lines: line_overrides
                    .into_iter()
                    .map(|l| ResolvedOverrideCoveredLineInfo {
                        nr: l.cov_line,
                        new_hits: l.new_hits,
                        old_hits: l.old_hits,
                        comment: l.content,
                    })
                    .collect(),
            })
        }

        let test_run_state: TestState = sqlx::query!(
            r#"
            select state as "state!:i64"
            from TestRunStates
            where product_id = $1
            and test_run_name = $2
            and test_run_date = $3
            "#,
            product.id,
            test_run.test_run_name,
            test_run.test_run_date
        )
        .fetch_one(transaction.as_mut())
        .await?
        .state
        .try_into()?;

        test_run_overrides.push(ResolvedOverrideTestRun {
            test_run: TestRunReference {
                product_id: product.id.clone(),
                name: test_run.test_run_name,
                utc_date: mantra_schema::test_runs::test_date_from_str(&test_run.test_run_date)
                    .expect("Valid test date in database"),
                state: test_run_state,
            },
            test_cases: if test_cases.is_empty() {
                None
            } else {
                Some(test_cases)
            },
            coverage: if coverage.is_empty() {
                None
            } else {
                Some(coverage)
            },
        })
    }

    Ok(test_run_overrides)
}

async fn review_ignored_entries<'db>(
    transaction: &mut MantraTransaction<'db>,
    review: &ReviewReference,
) -> Result<Option<IgnoredEntries>, anyhow::Error> {
    let db_entries = sqlx::query!(
        "
        select gj.content
        from IgnoredReviewEntries ie, GeneralJson gj
        where ie.product_id = $1
        and ie.review_name = $2
        and ie.review_date = $3
        and ie.entry_hash = gj.hash
        ",
        review.product_id,
        review.name,
        review.utc_date
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if db_entries.is_empty() {
        return Ok(None);
    }

    let mut requirements = Vec::new();
    let mut test_case_state_overrides = Vec::new();
    let mut test_case_line_coverage_overrides = Vec::new();
    let mut test_run_line_coverage_overrides = Vec::new();

    for db_entry in db_entries {
        let entry: DbIgnoredEntry = serde_json::from_str(&db_entry.content)?;

        async fn get_comment<'d>(
            transaction: &mut MantraTransaction<'d>,
            comment_hash: FmtHash,
        ) -> Result<String, anyhow::Error> {
            let comment = sqlx::query!(
                "
                select content
                from GeneralTexts
                where hash = $1
                ",
                comment_hash
            )
            .fetch_one(transaction.as_mut())
            .await?;

            Ok(comment.content)
        }

        match entry {
            DbIgnoredEntry::Requirement { id, comment_hash } => {
                requirements.push(IgnoredRequirement {
                    id,
                    comment: get_comment(transaction, comment_hash).await?,
                })
            }
            DbIgnoredEntry::TestCaseStateOverride {
                test_run_name,
                test_run_date: test_run_utc_date,
                test_case_name,
                state,
                comment_hash,
            } => test_case_state_overrides.push(IgnoredTestCaseStateOverride {
                test_run_name,
                test_run_date: test_run_utc_date,
                test_case_name,
                state,
                comment: get_comment(transaction, comment_hash).await?,
            }),
            DbIgnoredEntry::TestCaseLineCoverageOverride {
                test_run_name,
                test_run_date: test_run_utc_date,
                test_case_name,
                cov_filepath,
                cov_line,
                hits,
                comment_hash,
            } => test_case_line_coverage_overrides.push(IgnoredTestCaseLineCoverageOverride {
                test_run_name,
                test_run_date: test_run_utc_date,
                test_case_name,
                cov_filepath,
                cov_line,
                hits,
                comment: get_comment(transaction, comment_hash).await?,
            }),
            DbIgnoredEntry::TestRunLineCoverageOverride {
                test_run_name,
                test_run_date: test_run_utc_date,
                cov_filepath,
                cov_line,
                hits,
                comment_hash,
            } => test_run_line_coverage_overrides.push(IgnoredTestRunLineCoverageOverride {
                test_run_name,
                test_run_date: test_run_utc_date,
                cov_filepath,
                cov_line,
                hits,
                comment: get_comment(transaction, comment_hash).await?,
            }),
        };
    }

    Ok(Some(IgnoredEntries {
        total: (requirements.len()
            + test_case_state_overrides.len()
            + test_case_line_coverage_overrides.len()
            + test_run_line_coverage_overrides.len()) as i64,
        requirements: if requirements.is_empty() {
            None
        } else {
            Some(requirements)
        },
        test_case_state_overrides: if test_case_state_overrides.is_empty() {
            None
        } else {
            Some(test_case_state_overrides)
        },
        test_case_line_coverage_overrides: if test_case_line_coverage_overrides.is_empty() {
            None
        } else {
            Some(test_case_line_coverage_overrides)
        },
        test_run_line_coverage_overrides: if test_run_line_coverage_overrides.is_empty() {
            None
        } else {
            Some(test_run_line_coverage_overrides)
        },
    }))
}
