use std::str::FromStr;

use anyhow::Context;
use mantra_schema::{
    FmtHash, Origin, Properties, SCHEMA_VERSION,
    media_type::MediaType,
    mime_guess::Mime,
    product::ProductId,
    report::{
        ResolvedRevision,
        formattable_content::FormattableContent,
        product::ProductReportSchema,
        requirement::RequirementReference,
        review::{
            IgnoredEntries, IgnoredManualRequirementVerification,
            IgnoredTestCaseLineCoverageOverride, IgnoredTestCaseStateOverride,
            IgnoredTestRunLineCoverageOverride, ResolvedOverrideCoveredLineInfo,
            ResolvedOverrideFileCoverage, ResolvedOverrideTestCase, ResolvedOverrideTestCaseState,
            ResolvedOverrideTestRun, ReviewReference, ReviewReportSchema, VerifiedRequirement,
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
            r.utc_date
        from Reviews r
            left join GeneralTexts gt on r.description_hash = gt.hash
            left join GeneralJson og on r.origin_hash = og.hash
        where r.product_id = $1 and r.name = $2
        "#,
        product.id,
        review.name,
    )
    .fetch_one(transaction.as_mut())
    .await?;

    let origin = match metadata.origin {
        Some(o) => Some(serde_json::Value::from_str(&o)?),
        None => None,
    };

    let media_type = review_content_media_type(transaction, &product.id, &review.name).await?;

    let authors: Vec<_> = sqlx::query!(
        "
        select author
        from ReviewAuthors
        where product_id = $1 and review_name = $2
        ",
        product.id,
        review.name,
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|a| a.author)
    .collect();

    let revision_records = sqlx::query!(
        "
        select r.revision, gt.content
        from ReviewRevisions r, GeneralTexts gt
        where r.product_id = $1 and r.review_name = $2
        and r.summary_hash = gt.hash
        ",
        product.id,
        review.name,
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
                and revision = $3
                ",
                product.id,
                review.name,
                rev.revision
            )
            .fetch_all(transaction.as_mut())
            .await?
            .into_iter()
            .map(|a| a.author)
            .collect();

            revisions.push(ResolvedRevision {
                number: rev.revision,
                authors,
                summary: FormattableContent::new(rev.content, media_type.clone()),
            });
        }

        Some(revisions)
    };

    let property_records = sqlx::query!(
        "
        select rp.property_key, v.content
        from ReviewProperties rp, GeneralJson v
        where rp.product_id = $1 and rp.review_name = $2
        and rp.value_hash = v.hash
        ",
        product.id,
        review.name,
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
            exists (
                select req_id
                from OptionalRequirements o
                where o.product_id = $1 and o.req_id = rs.req_id
            ) as "optional!:bool",
            exists (
                select req_id
                from ManualRequirements o
                where o.product_id = $1 and o.req_id = rs.req_id
            ) as "manual!:bool",
            gt.content
        from RequirementVerificationStates rs, ManuallyVerifiedRequirements vr, GeneralTexts gt
        where vr.product_id = $1 and rs.product_id = $1
        and vr.review_name = $2
        and vr.req_id = rs.req_id
        and vr.rational_hash = gt.hash
        "#,
        product.id,
        review.name,
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
            manual: r.manual,
        },
        rational: FormattableContent::new(r.content, media_type.clone()),
    })
    .collect();
    let requirements = if requirements.is_empty() {
        None
    } else {
        Some(requirements)
    };

    let test_run_overrides =
        review_test_run_overrides(transaction, product, review, &media_type).await?;
    let test_run_overrides = if test_run_overrides.is_empty() {
        None
    } else {
        Some(test_run_overrides)
    };

    let schema_origin = review_schema_origin(transaction, &product.id, &review.name).await?;
    let schema_properties =
        review_schema_properties(transaction, &product.id, &review.name).await?;

    let config_origin = review_config_origin(transaction, &product.id, &review.name).await?;
    let config_properties =
        review_config_properties(transaction, &product.id, &review.name).await?;

    let ignored_entries = review_ignored_entries(transaction, review, &media_type).await?;

    Ok(ReviewReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        name: review.name.clone(),
        utc_date: mantra_schema::datetime_from_str(&metadata.utc_date)
            .context("Converting stored review date")?,
        state: review.state,
        authors,
        description: metadata
            .description
            .map(|d| FormattableContent::new(d, media_type)),
        revisions,
        requirements,
        test_run_overrides,
        ignored_entries,
        origin,
        schema_origin,
        config_origin,
        properties,
        schema_properties,
        config_properties,
    })
}

pub(super) async fn review_content_media_type<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    review_name: &str,
) -> Result<Option<MediaType>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select media_type
        from SchemaContentMediaTypes mt, Reviews r
        where r.product_id = $1 and r.name = $2
        and r.schema_hash = mt.schema_hash
        order by mt.entry_number
        ",
        product_id,
        review_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if records.is_empty() {
        return Ok(None);
    }

    let mut mimes = Vec::with_capacity(records.len());

    for record in records {
        let mime = Mime::from_str(&record.media_type)?;
        mimes.push(mime);
    }

    Ok(Some(MediaType::new(mimes)))
}

async fn review_test_run_overrides<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    review: &ReviewReference,
    media_type: &Option<MediaType>,
) -> Result<Vec<ResolvedOverrideTestRun>, anyhow::Error> {
    let overriden_test_runs = sqlx::query!(
        "
        select test_run_name
        from TestCaseOverrides
        where product_id = $1
        and review_name = $2

        union

        select test_run_name
        from TestRunLineCoverageOverrides
        where product_id = $1
        and review_name = $2

        union

        select test_run_name
        from TestCaseLineCoverageOverrides
        where product_id = $1
        and review_name = $2
        ",
        product.id,
        review.name,
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

            union

            select test_case_name
            from TestCaseLineCoverageOverrides
            where product_id = $1
            and test_run_name = $2
            ",
            product.id,
            test_run.test_run_name,
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
                and name = $3
                ",
                product.id,
                test_run.test_run_name,
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
                and test_case_name = $3
                ",
                product.id,
                test_run.test_run_name,
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
                and test_case_name = $3
                and rational_hash = hash
                ",
                product.id,
                test_run.test_run_name,
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
                and test_case_name = $3
                ",
                product.id,
                test_run.test_run_name,
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
                        co.covered,
                        tc.hits as original_hits,
                        gt.content
                    from
                        TestCaseLineCoverageOverrides co,
                        TestCaseLineCoverage tc,
                        GeneralTexts gt
                    where co.product_id = $1 and tc.product_id = $1
                    and co.test_run_name = $2 and tc.test_run_name = $2
                    and co.test_case_name = $3 and tc.test_case_name = $3
                    and co.cov_filepath = $4 and tc.cov_filepath = $4
                    and co.cov_line = tc.cov_line
                    and co.rational_hash = gt.hash
                    "#,
                    product.id,
                    test_run.test_run_name,
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
                            covered: l.covered,
                            original_hits: l.original_hits,
                            rational: FormattableContent::new(l.content, media_type.clone()),
                        })
                        .collect(),
                })
            }

            test_cases.push(ResolvedOverrideTestCase {
                test_case: TestCaseReference {
                    product_id: product.id.clone(),
                    test_run_name: test_run.test_run_name.clone(),
                    test_case_name: test_case.test_case_name,
                    state: resolved_state,
                },
                state: overriden_state.map(|s| ResolvedOverrideTestCaseState {
                    new: s.state.try_into().expect("Valid state in database"),
                    old: old_state,
                    rational: FormattableContent::new(s.content, media_type.clone()),
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
                "#,
                product.id,
                test_run.test_run_name,
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
            ",
            product.id,
            test_run.test_run_name,
        )
        .fetch_all(transaction.as_mut())
        .await?;

        for filepath in overriden_files {
            let line_overrides = sqlx::query!(
                r#"
                select
                    co.cov_line,
                    co.covered,
                    tr.hits as original_hits,
                    gt.content
                from
                    TestRunLineCoverageOverrides co,
                    TestRunLineCoverage tr,
                    GeneralTexts gt
                where co.product_id = $1 and tr.product_id = $1
                and co.test_run_name = $2 and tr.test_run_name = $2
                and co.cov_filepath = $3 and tr.cov_filepath = $3
                and co.cov_line = tr.cov_line
                and co.rational_hash = gt.hash
                "#,
                product.id,
                test_run.test_run_name,
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
                        covered: l.covered,
                        original_hits: l.original_hits,
                        rational: FormattableContent::new(l.content, media_type.clone()),
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
            "#,
            product.id,
            test_run.test_run_name,
        )
        .fetch_one(transaction.as_mut())
        .await?
        .state
        .try_into()?;

        test_run_overrides.push(ResolvedOverrideTestRun {
            test_run: TestRunReference {
                product_id: product.id.clone(),
                name: test_run.test_run_name,
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

async fn review_schema_origin<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    review_name: &str,
) -> Result<Option<Origin>, anyhow::Error> {
    let record = sqlx::query!(
        "
        select gj.content
        from Schemas s, Reviews r, GeneralJson gj
        where r.product_id = $1 and r.name = $2
        and r.schema_hash = s.content_hash
        and gj.hash = s.origin_hash
        ",
        product_id,
        review_name
    )
    .fetch_optional(transaction.as_mut())
    .await?;

    match record {
        Some(r) => Ok(Some(serde_json::Value::from_str(&r.content)?)),
        None => Ok(None),
    }
}

async fn review_schema_properties<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    review_name: &str,
) -> Result<Option<Properties>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select sp.property_key, gj.content
        from SchemaReviewProperties sp, Reviews r, GeneralJson gj
        where r.product_id = $1 and r.name = $2
        and r.schema_hash = sp.schema_hash
        and gj.hash = sp.value_hash
        ",
        product_id,
        review_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if records.is_empty() {
        return Ok(None);
    }

    let mut props = Properties::with_capacity(records.len());

    for record in records {
        props.insert(
            record.property_key,
            serde_json::Value::from_str(&record.content)?,
        );
    }

    Ok(Some(props))
}

async fn review_config_origin<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    review_name: &str,
) -> Result<Option<Origin>, anyhow::Error> {
    let record = sqlx::query!(
        "
        select gj.content
        from Reviews r, ReviewCollectConfigs rc, ConfigCollectedSchemas cs, GeneralJson gj
        where r.product_id = $1 and r.name = $2
        and r.schema_hash = cs.schema_hash
        and cs.cfg_entry = rc.cfg_entry
        and gj.hash = rc.origin_hash
        ",
        product_id,
        review_name
    )
    .fetch_optional(transaction.as_mut())
    .await?;

    match record {
        Some(r) => Ok(Some(serde_json::Value::from_str(&r.content)?)),
        None => Ok(None),
    }
}

async fn review_config_properties<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
    review_name: &str,
) -> Result<Option<Properties>, anyhow::Error> {
    let records = sqlx::query!(
        "
        select cp.property_key, gj.content
        from Reviews r, ConfigCollectedSchemas cs, ConfigReviewProperties cp, GeneralJson gj
        where r.product_id = $1 and r.name = $2
        and r.schema_hash = cs.schema_hash
        and cs.cfg_entry = cp.cfg_entry
        and gj.hash = cp.value_hash
        ",
        product_id,
        review_name
    )
    .fetch_all(transaction.as_mut())
    .await?;

    if records.is_empty() {
        return Ok(None);
    }

    let mut props = Properties::with_capacity(records.len());

    for record in records {
        props.insert(
            record.property_key,
            serde_json::Value::from_str(&record.content)?,
        );
    }

    Ok(Some(props))
}

async fn review_ignored_entries<'db>(
    transaction: &mut MantraTransaction<'db>,
    review: &ReviewReference,
    media_type: &Option<MediaType>,
) -> Result<Option<IgnoredEntries>, anyhow::Error> {
    let db_entries = sqlx::query!(
        "
        select gj.content
        from IgnoredReviewEntries ie, GeneralJson gj
        where ie.product_id = $1
        and ie.review_name = $2
        and ie.entry_hash = gj.hash
        ",
        review.product_id,
        review.name,
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

        async fn get_rational<'d>(
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
            DbIgnoredEntry::Requirement { id, rational_hash } => {
                requirements.push(IgnoredManualRequirementVerification {
                    id,
                    rational: get_rational(transaction, rational_hash).await?,
                })
            }
            DbIgnoredEntry::TestCaseStateOverride {
                test_run_name,
                test_case_name,
                state,
                rational_hash,
            } => test_case_state_overrides.push(IgnoredTestCaseStateOverride {
                test_run_name,
                test_case_name,
                state,
                rational: FormattableContent::new(
                    get_rational(transaction, rational_hash).await?,
                    media_type.clone(),
                ),
            }),
            DbIgnoredEntry::TestCaseLineCoverageOverride {
                test_run_name,
                test_case_name,
                cov_filepath,
                cov_line,
                covered,
                rational_hash,
            } => test_case_line_coverage_overrides.push(IgnoredTestCaseLineCoverageOverride {
                test_run_name,
                test_case_name,
                cov_filepath,
                cov_line,
                covered,
                rational: FormattableContent::new(
                    get_rational(transaction, rational_hash).await?,
                    media_type.clone(),
                ),
            }),
            DbIgnoredEntry::TestRunLineCoverageOverride {
                test_run_name,
                cov_filepath,
                cov_line,
                covered,
                rational_hash,
            } => test_run_line_coverage_overrides.push(IgnoredTestRunLineCoverageOverride {
                test_run_name,
                cov_filepath,
                cov_line,
                covered,
                rational: FormattableContent::new(
                    get_rational(transaction, rational_hash).await?,
                    media_type.clone(),
                ),
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
