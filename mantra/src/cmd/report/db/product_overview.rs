use std::{collections::HashMap, path::PathBuf, str::FromStr};

use mantra_schema::{
    FmtHash, Line,
    annotations::TraceKind,
    path::RelativePathBuf,
    product::{Product, ProductId},
    report::{
        Aggregated, RequirementReference, RequirementState, ReviewReference, TestCaseReference,
        TestRunReference, TraceReference,
        overview::{
            AnnotationsOverview, CoverageExcludesOverview, CoveredByTestsOverview,
            CoveredLinesSummary, ElementsOverview, ExclusionAnnotationReference,
            ProductOverviewReport, RequirementOverview, RequirementTracesOverview,
            RequirementsOverview, RequirementsSummary, ResolvedCoveredFile, ResolvedCoveredLine,
            ResolvedCoveredLineState, ResolvedCoveredLines, ReviewOverview, ReviewsOverview,
            ReviewsSummary, TestCaseOverview, TestCasesOverview, TestCasesSummary,
            TestCoverageOverview, TestCoverageSummary, TestRelatedRequirementKind,
            TestRelatedRequirementOverview, TestRunOverview, TestRunsOverview, TraceOverview,
            TracesOverview, TracesPerFile, TracesSummary, VerifiedRequirementOverview, percentage,
        },
    },
    reviews::{
        OverrideCoveredLineInfo, OverrideFileCoverage, OverrideTestCase, OverrideTestCaseState,
        OverrideTestRun,
    },
    test_runs::{TestCaseLocation, TestState},
    time::OffsetDateTime,
};

use crate::db::{MantraConnection, MantraTransaction};

pub(super) struct ProductReporter<'t, 'db> {
    transaction: &'t mut MantraTransaction<'db>,
    cfg_filepath: PathBuf,
    abs_cfg_file_parent_path: PathBuf,
    collect_nr: i64,
    product: Product,
    reported_at_utc: OffsetDateTime,
}

impl<'t, 'db> ProductReporter<'t, 'db> {
    pub(super) async fn new(
        transaction: &'t mut MantraTransaction<'db>,
        cfg_filepath: PathBuf,
        product_id: ProductId,
    ) -> Result<Self, anyhow::Error> {
        let reported_at_utc = OffsetDateTime::now_utc();

        let product_record = sqlx::query!(
            r#"
                select
                    p.last_collect_nr,
                    p.id,
                    p.name,
                    p.base,
                    p.version,
                    p.homepage,
                    p.repository,
                    p.license,
                    gt.content as "description?"
                from Products p left join GeneralTexts gt on p.description_hash = gt.hash
                where id = $1
            "#,
            product_id
        )
        .fetch_optional(transaction.as_mut())
        .await?;

        match product_record {
            Some(entry) => {
                let mut product_properties = serde_json::Map::new();

                for entry in sqlx::query!(
                    r#"
                        select p.property_key, gj.content
                        from ProductProperties p, GeneralJson gj
                        where p.product_id = $1 and gj.hash = p.value_hash
                    "#,
                    product_id
                )
                .fetch_all(transaction.as_mut())
                .await?
                {
                    product_properties.insert(
                        entry.property_key,
                        serde_json::Value::from_str(&entry.content)?,
                    );
                }

                let product_properties = if product_properties.is_empty() {
                    None
                } else {
                    Some(product_properties)
                };

                Ok(Self {
                    transaction,
                    abs_cfg_file_parent_path: crate::io::abs_parent_path(&cfg_filepath)?,
                    cfg_filepath: cfg_filepath,
                    collect_nr: entry.last_collect_nr,
                    product: Product {
                        id: entry.id,
                        base: entry.base,
                        name: entry.name,
                        version: entry.version,
                        homepage: entry.homepage,
                        repository: entry.repository,
                        license: entry.license,
                        description: entry.description,
                        properties: product_properties,
                    },
                    reported_at_utc,
                })
            }
            None => anyhow::bail!("No data collected for product '{}'", product_id),
        }
    }

    fn connection_mut(&mut self) -> &mut MantraConnection {
        self.transaction.as_mut()
    }

    fn collect_nr(&self) -> i64 {
        self.collect_nr
    }

    fn product_id(&self) -> ProductId {
        self.product.id.clone()
    }
}

impl<'t, 'db> ProductReporter<'t, 'db> {
    pub async fn product_overview(mut self) -> Result<ProductOverviewReport, anyhow::Error> {
        let annotations = self.annotations_overview().await?;
        let requirements = self.requirements_overview().await?;
        let test_runs = self.test_runs_overview().await?;
        let reviews = self.reviews_overview().await?;

        Ok(ProductOverviewReport {
            product: self.product,
            annotations,
            requirements,
            test_runs,
            reviews,
        })
    }

    async fn annotations_overview(&mut self) -> Result<AnnotationsOverview, anyhow::Error> {
        let traces = self.traces_overview().await?;
        let elements = self.elements_overview().await?;
        let coverage_excludes = self.coverage_excludes_overview().await?;

        Ok(AnnotationsOverview {
            traces,
            elements,
            coverage_excludes,
        })
    }

    async fn traces_overview(&mut self) -> Result<TracesOverview, anyhow::Error> {
        let product_id = self.product_id();

        let traces = sqlx::query!(
            "
            select pf.filepath, pf.file_hash, t.line, t.kind
            from ProductRelatedFiles pf, Traces t
            where pf.product_id = $1
            and pf.file_hash = t.file_hash
            ",
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        let mut summary = TracesSummary {
            total: traces.len() as i64,
            ..Default::default()
        };

        let mut traces_per_file = HashMap::<String, TracesPerFile>::new();

        for trace in traces {
            let referenced_ids = sqlx::query!(
                r#"
                select
                    dt.req_id, rs.state,
                    case when dt.req_id in (
                        select id
                        from OptionalRequirements
                        where product_id = $1
                    ) then true
                    else false end as "optional!:bool"
                from DirectProductReqTraces dt, RequirementVerificationStates rs
                where dt.product_id = $1 and rs.product_id = $1
                and dt.filepath = $2 and dt.file_hash = $3 and dt.line = $4
                and dt.req_id = rs.id
                "#,
                product_id,
                trace.filepath,
                trace.file_hash,
                trace.line
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|r| RequirementReference {
                id: r.req_id,
                product_id: None, // Note: only requirements in the same product may be traced
                state: RequirementState::try_from(r.state)
                    .expect("Valid requirement state in the database"),
                optional: r.optional,
            })
            .collect();

            let covering_test_runs: Vec<TestRunReference> = sqlx::query!(
                r#"
                select distinct tr.test_run_name, tr.test_run_date, ts.state as "state!:i64"
                from
                    TracesCoveredByTestRuns tr,
                    TestRunStates ts
                where tr.product_id = $1 and ts.product_id = $1
                and tr.filepath = $2 and tr.file_hash = $3
                and tr.traced_line = $4
                and tr.test_run_name = ts.test_run_name
                and tr.test_run_date = ts.test_run_date
                "#,
                product_id,
                trace.filepath,
                trace.file_hash,
                trace.line
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|tr| TestRunReference {
                name: tr.test_run_name,
                utc_date: mantra_schema::test_runs::test_date_from_str(&tr.test_run_date)
                    .expect("Valid test date in database"),
                state: TestState::try_from(tr.state).expect("Valid test state in database"),
            })
            .collect();

            let covering_test_cases: Vec<TestCaseReference> = sqlx::query!(
                r#"
                select distinct
                    tc.test_run_name,
                    tc.test_run_date,
                    tc.test_case_name,
                    ts.state as "state!:i64"
                from
                    TracesCoveredByTestCases tc,
                    ResolvedTestCaseStates ts
                where tc.product_id = $1 and ts.product_id = $1
                and tc.filepath = $2 and tc.file_hash = $3
                and tc.traced_line = $4
                and tc.test_run_name = ts.test_run_name
                and tc.test_run_date = ts.test_run_date
                and tc.test_case_name = ts.test_case_name
                "#,
                product_id,
                trace.filepath,
                trace.file_hash,
                trace.line
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|tc| TestCaseReference {
                test_run_name: tc.test_run_name,
                test_run_date: mantra_schema::test_runs::test_date_from_str(&tc.test_run_date)
                    .expect("Valid test date in database"),
                test_case_name: tc.test_case_name,
                state: TestState::try_from(tc.state).expect("Valid test state in database"),
            })
            .collect();

            let covered_by = if covering_test_runs.is_empty() && covering_test_cases.is_empty() {
                None
            } else {
                Some(CoveredByTestsOverview {
                    test_runs: covering_test_runs,
                    test_cases: covering_test_cases,
                })
            };

            let trace_overview = TraceOverview {
                ids: referenced_ids,
                line: trace.line,
                related_code: None, // TODO
                kind: TraceKind::try_from(trace.kind)?,
                properties: None, // TODO
                covered_by,
            };

            let mut trace_summary = TracesSummary {
                total: 1,
                ..Default::default()
            };
            match trace_overview.kind {
                TraceKind::Clarifies => trace_summary.clarifies.cnt = 1,
                TraceKind::Satisfies => trace_summary.satisfies.cnt = 1,
                TraceKind::Verifies => trace_summary.verifies.cnt = 1,
                TraceKind::Links => trace_summary.links.cnt = 1,
            }

            summary.add(&trace_summary);

            let entry = traces_per_file
                .entry(trace.filepath.clone())
                .and_modify(|entry| {
                    entry.summary.add(&trace_summary);
                })
                .or_insert(TracesPerFile {
                    summary: trace_summary,
                    filepath: RelativePathBuf::from(trace.filepath),
                    file_hash: FmtHash::with_inner(trace.file_hash),
                    traces: vec![],
                });
            entry.traces.push(trace_overview);
        }

        Ok(TracesOverview {
            summary,
            files: traces_per_file.into_values().collect(),
        })
    }

    async fn elements_overview(&mut self) -> Result<ElementsOverview, anyhow::Error> {
        //TODO

        Ok(ElementsOverview { files: Vec::new() })
    }

    async fn coverage_excludes_overview(
        &mut self,
    ) -> Result<CoverageExcludesOverview, anyhow::Error> {
        // TODO

        Ok(CoverageExcludesOverview { files: Vec::new() })
    }

    async fn requirements_overview(&mut self) -> Result<RequirementsOverview, anyhow::Error> {
        let product_id = self.product_id();

        let record = sqlx::query!(
            r#"
            select
                r.id,
                r.title,
                rs.state,
                gt.content as "description?",
                o.content as "origin?",
                bo.content as "base_origin?"
            from Requirements r left join GeneralTexts gt on r.description_hash = gt.hash
            left join GeneralJson o on r.origin_hash = o.hash
            left join GeneralJson bo on r.base_origin_hash = bo.hash,
            RequirementVerificationStates rs
            where r.product_id = $1 and rs.product_id = $1
            and r.id = rs.id
            "#,
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        let mut requirements = Vec::with_capacity(record.len());

        for entry in record {
            let is_optional = sqlx::query!(
                "
                select id from OptionalRequirements
                where product_id = $1 and id = $2
                ",
                product_id,
                entry.id
            )
            .fetch_optional(self.connection_mut())
            .await?
            .is_some();

            let is_manual = sqlx::query!(
                "
                select id from ManualRequirements
                where product_id = $1 and id = $2
                ",
                product_id,
                entry.id
            )
            .fetch_optional(self.connection_mut())
            .await?
            .is_some();

            let children_record = sqlx::query!(
                "
                select r.product_id, r.id, rs.state, r.optional
                from RequirementHierarchies rh, RequirementVerificationStates rs, Requirements r
                where rh.parent_product_id = $1 and rh.parent_req_id = $2
                and rh.child_product_id = rs.product_id and rh.child_req_id = rs.id
                and r.product_id = rs.product_id and r.id = rs.id
                ",
                product_id,
                entry.id
            )
            .fetch_all(self.connection_mut())
            .await?;

            let mut children = Vec::with_capacity(children_record.len());
            for child in children_record {
                let child_product_id = if product_id == child.product_id {
                    None
                } else {
                    Some(child.product_id)
                };

                children.push(RequirementReference {
                    id: child.id,
                    product_id: child_product_id,
                    state: child.state.try_into()?,
                    optional: child.optional,
                })
            }

            let parent_record = sqlx::query!(
                "
                select r.product_id, r.id, rs.state, r.optional
                from RequirementHierarchies rh, RequirementVerificationStates rs, Requirements r
                where rh.child_product_id = $1 and rh.child_req_id = $2
                and rh.parent_product_id = rs.product_id and rh.parent_req_id = rs.id
                and r.product_id = rs.product_id and r.id = rs.id
                ",
                product_id,
                entry.id
            )
            .fetch_all(self.connection_mut())
            .await?;

            let mut parents = Vec::with_capacity(parent_record.len());
            for parent in parent_record {
                let parent_product_id = if product_id == parent.product_id {
                    None
                } else {
                    Some(parent.product_id)
                };

                parents.push(RequirementReference {
                    id: parent.id,
                    product_id: parent_product_id,
                    state: parent.state.try_into()?,
                    optional: parent.optional,
                })
            }

            let direct_traces = sqlx::query!(
                "
                select dt.filepath, dt.file_hash, dt.line, t.kind
                from DirectProductReqTraces dt, Traces t
                where dt.product_id = $1 and dt.req_id = $2
                and dt.file_hash = t.file_hash and dt.line = t.line
                ",
                product_id,
                entry.id
            )
            .fetch_all(self.connection_mut())
            .await?;

            let traces = if direct_traces.is_empty() {
                None
            } else {
                let mut trace_summary = TracesSummary {
                    total: direct_traces.len() as i64,
                    ..Default::default()
                };
                let mut trace_refs = Vec::with_capacity(direct_traces.len());
                for trace in direct_traces {
                    //TODO: don't fail on first error
                    let kind = TraceKind::try_from(trace.kind)?;
                    match kind {
                        TraceKind::Clarifies => trace_summary.clarifies.cnt += 1,
                        TraceKind::Satisfies => trace_summary.satisfies.cnt += 1,
                        TraceKind::Verifies => trace_summary.verifies.cnt += 1,
                        TraceKind::Links => trace_summary.links.cnt += 1,
                    }

                    trace_refs.push(TraceReference {
                        filepath: RelativePathBuf::from(trace.filepath),
                        file_hash: FmtHash::with_inner(trace.file_hash),
                        line: trace.line,
                        kind,
                    });
                }

                trace_summary.clarifies.percentage =
                    percentage!(trace_summary.clarifies.cnt, trace_summary.total);
                trace_summary.satisfies.percentage =
                    percentage!(trace_summary.satisfies.cnt, trace_summary.total);
                trace_summary.verifies.percentage =
                    percentage!(trace_summary.verifies.cnt, trace_summary.total);
                trace_summary.links.percentage =
                    percentage!(trace_summary.links.cnt, trace_summary.total);

                Some(RequirementTracesOverview {
                    summary: trace_summary,
                    all: trace_refs,
                })
            };

            let covering_test_runs: Vec<TestRunReference> = sqlx::query!(
                r#"
                select distinct tr.test_run_name, tr.test_run_date, ts.state as "state!:i64"
                from
                    TracesCoveredByTestRuns tr,
                    DirectProductReqTraces dt,
                    TestRunStates ts
                where tr.product_id = $1 and dt.product_id = $1 and ts.product_id = $1
                and tr.filepath = dt.filepath and tr.file_hash = dt.file_hash
                and tr.traced_line = dt.line and dt.req_id = $2
                and tr.test_run_name = ts.test_run_name
                and tr.test_run_date = ts.test_run_date
                "#,
                product_id,
                entry.id
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|tr| TestRunReference {
                name: tr.test_run_name,
                utc_date: mantra_schema::test_runs::test_date_from_str(&tr.test_run_date)
                    .expect("Valid test date in database"),
                state: TestState::try_from(tr.state).expect("Valid test state in database"),
            })
            .collect();

            let covering_test_cases: Vec<TestCaseReference> = sqlx::query!(
                r#"
                select distinct
                    tc.test_run_name,
                    tc.test_run_date,
                    tc.test_case_name,
                    ts.state as "state!:i64"
                from
                    TracesCoveredByTestCases tc,
                    DirectProductReqTraces dt,
                    ResolvedTestCaseStates ts
                where tc.product_id = $1 and dt.product_id = $1 and ts.product_id = $1
                and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
                and tc.traced_line = dt.line and dt.req_id = $2
                and tc.test_run_name = ts.test_run_name
                and tc.test_run_date = ts.test_run_date
                and tc.test_case_name = ts.test_case_name

                union

                select distinct
                    tv.test_run_name,
                    tv.test_run_date,
                    tv.test_case_name,
                    ts.state as "state!:i64"
                from TestCaseVerifiedRequirements tv, ResolvedTestCaseStates ts
                where tv.product_id = $1 and ts.product_id = $1
                and tv.req_id = $2
                and tv.test_run_name = ts.test_run_name
                and tv.test_run_date = ts.test_run_date
                and tv.test_case_name = ts.test_case_name
                "#,
                product_id,
                entry.id
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|tc| TestCaseReference {
                test_run_name: tc.test_run_name,
                test_run_date: mantra_schema::test_runs::test_date_from_str(&tc.test_run_date)
                    .expect("Valid test date in database"),
                test_case_name: tc.test_case_name,
                state: TestState::try_from(tc.state).expect("Valid test state in database"),
            })
            .collect();

            let reviewed_in: Vec<ReviewReference> = sqlx::query!(
                "
                select review_name, review_date
                from ManuallyVerifiedRequirements
                where product_id = $1 and req_id = $2
                ",
                product_id,
                entry.id
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|r| ReviewReference {
                name: r.review_name,
                utc_date: mantra_schema::reviews::date_from_str(&r.review_date)
                    .expect("Valid review date in database"),
            })
            .collect();

            let children = if children.is_empty() {
                None
            } else {
                Some(children)
            };
            let parents = if parents.is_empty() {
                None
            } else {
                Some(parents)
            };
            let covered_by = if covering_test_runs.is_empty() && covering_test_cases.is_empty() {
                None
            } else {
                Some(CoveredByTestsOverview {
                    test_runs: covering_test_runs,
                    test_cases: covering_test_cases,
                })
            };
            let reviewed_in = if reviewed_in.is_empty() {
                None
            } else {
                Some(reviewed_in)
            };

            requirements.push(RequirementOverview {
                id: entry.id,
                title: entry.title,
                state: entry.state.try_into()?,
                optional: is_optional,
                parents,
                children,
                manual_verification: is_manual,
                description: entry.description,
                origin: entry.origin.and_then(|o| serde_json::from_str(&o).ok()),
                base_origin: entry
                    .base_origin
                    .and_then(|o| serde_json::from_str(&o).ok()),
                traces,
                covered_by,
                reviewed_in,
            })
        }

        let mut summary = RequirementsSummary {
            total: requirements.len() as i64,
            ..Default::default()
        };

        let roots = requirements
            .iter()
            .inspect(|r| {
                match r.state {
                    mantra_schema::report::RequirementState::Failed => summary.failed.cnt += 1,
                    mantra_schema::report::RequirementState::Verified => {
                        summary.verified.cnt += 1;

                        if !r.optional {
                            summary.mandatory_verified.cnt += 1;
                        }
                    }
                    mantra_schema::report::RequirementState::Skipped => summary.skipped.cnt += 1,
                    mantra_schema::report::RequirementState::Unverified => {
                        summary.unverified.cnt += 1
                    }
                    mantra_schema::report::RequirementState::Deprecated => {
                        summary.deprecated.cnt += 1
                    }
                    mantra_schema::report::RequirementState::Ignored => summary.ignored.cnt += 1,
                };
                if r.manual_verification {
                    summary.manual_verification.cnt += 1;
                }
            })
            .filter(|r| r.parents == None)
            .cloned()
            .collect();

        summary.deprecated.percentage = percentage!(summary.deprecated.cnt, summary.total);
        summary.ignored.percentage = percentage!(summary.ignored.cnt, summary.total);
        summary.verified.percentage = percentage!(summary.verified.cnt, summary.total);
        summary.mandatory_verified.percentage =
            percentage!(summary.mandatory_verified.cnt, summary.total);
        summary.failed.percentage = percentage!(summary.failed.cnt, summary.total);
        summary.skipped.percentage = percentage!(summary.skipped.cnt, summary.total);
        summary.unverified.percentage = percentage!(summary.unverified.cnt, summary.total);
        summary.manual_verification.percentage =
            percentage!(summary.manual_verification.cnt, summary.total);

        Ok(RequirementsOverview {
            summary,
            roots,
            all: requirements,
        })
    }

    async fn test_runs_overview(&mut self) -> Result<TestRunsOverview, anyhow::Error> {
        let product_id = self.product_id();

        let test_run_states = sqlx::query!(
            r#"
            select test_run_name, test_run_date, state as "state!:i64"
            from TestRunStates
            where product_id = $1
            "#,
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        let mut summary = TestCasesSummary::default();
        let mut test_runs = Vec::with_capacity(test_run_states.len());

        for tr in test_run_states {
            let test_cases = self
                .test_cases_overview(&tr.test_run_name, &tr.test_run_date)
                .await?;

            // Note: No need to add test cases of child test runs,
            // because all test runs are iterated in this loop.
            if let Some(tests) = &test_cases {
                summary.add(&tests.summary);
            }

            let parents = sqlx::query!(
                r#"
                    select th.parent_name, th.parent_date, ts.state as "state!:i64"
                    from TestRunHierarchies th, TestRunStates ts
                    where th.product_id = $1 and ts.product_id = $1
                    and th.child_name = $2 and ts.test_run_name = $2
                    and th.child_date = $3 and ts.test_run_date = $3
                "#,
                product_id,
                tr.test_run_name,
                tr.test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?;

            let children = sqlx::query!(
                r#"
                    select child_name, child_date, ts.state as "state!:i64"
                    from TestRunHierarchies th, TestRunStates ts
                    where th.product_id = $1 and ts.product_id = $1
                    and th.parent_name = $2 and ts.test_run_name = $2
                    and th.parent_date = $3 and ts.test_run_date = $3
                "#,
                product_id,
                tr.test_run_name,
                tr.test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?;

            // TODO: don't fail on first, but skip + log
            let state = tr.state.try_into()?;

            let related_reqs = self
                .test_related_requirements(&tr.test_run_name, &tr.test_run_date, None)
                .await?;
            let coverage = self
                .test_related_coverage(&tr.test_run_name, &tr.test_run_date, None)
                .await?;

            test_runs.push(TestRunOverview {
                name: tr.test_run_name,
                utc_date: OffsetDateTime::parse(
                    &tr.test_run_date,
                    &mantra_schema::time::format_description::well_known::Iso8601::PARSING,
                )
                .expect("Valid test date in database"),
                state,
                test_cases,
                parents: if parents.is_empty() {
                    None
                } else {
                    Some(
                        parents
                            .into_iter()
                            .map(|p| TestRunReference {
                                name: p.parent_name,
                                utc_date: OffsetDateTime::parse(
                                    &p.parent_date,
                                    &mantra_schema::time::format_description::well_known::Iso8601::PARSING,
                                )
                                .expect("Invalid test date in database"),
                                state: TestState::try_from(p.state).expect("Invalid test state in database"),
                            })
                            .collect(),
                    )
                },
                children: if children.is_empty() {
                    None
                } else {
                    Some(
                        children
                            .into_iter()
                            .map(|c| TestRunReference {
                                name: c.child_name,
                                utc_date: OffsetDateTime::parse(
                                    &c.child_date,
                                    &mantra_schema::time::format_description::well_known::Iso8601::PARSING,
                                )
                                .expect("Valid test date in database"),
                                state: TestState::try_from(c.state).expect("Invalid test state in database"),
                            })
                            .collect(),
                    )
                },
                related_reqs,
                coverage,
            })
        }

        summary.failed.percentage = percentage!(summary.failed.cnt, summary.total);
        summary.passed.percentage = percentage!(summary.passed.cnt, summary.total);
        summary.skipped.percentage = percentage!(summary.skipped.cnt, summary.total);
        summary.unknown.percentage = percentage!(summary.unknown.cnt, summary.total);
        summary.obsolete.percentage = percentage!(summary.obsolete.cnt, summary.total);

        Ok(TestRunsOverview {
            test_cases_summary: summary,
            all: test_runs,
        })
    }

    async fn test_related_requirements(
        &mut self,
        test_run_name: &str,
        test_run_date: &str,
        test_case_name: Option<&str>,
    ) -> Result<Option<Vec<TestRelatedRequirementOverview>>, anyhow::Error> {
        let product_id = self.product_id();

        let traced_reqs = if let Some(test_case) = test_case_name {
            let req_traces = sqlx::query!(
                "
                select distinct dt.product_id, dt.req_id, dt.filepath, dt.file_hash, dt.line, t.kind
                from TraceCoveragePerTestCases tc, DirectProductReqTraces dt, Traces t
                where tc.product_id = $1
                and tc.test_run_name = $2 and tc.test_run_date = $3
                and tc.test_case_name = $4
                and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
                and tc.traced_line = dt.line and dt.file_hash = t.file_hash
                and dt.line = t.line
                ",
                product_id,
                test_run_name,
                test_run_date,
                test_case
            )
            .fetch_all(self.connection_mut())
            .await?;

            let mut traced_reqs: HashMap<(String, String), Vec<TraceReference>> = HashMap::new();

            for trace in req_traces {
                traced_reqs
                    .entry((trace.product_id, trace.req_id))
                    .or_default()
                    .push(TraceReference {
                        filepath: RelativePathBuf::from(trace.filepath),
                        file_hash: FmtHash::with_inner(trace.file_hash),
                        line: trace.line,
                        kind: TraceKind::try_from(trace.kind)?,
                    });
            }

            traced_reqs
        } else {
            let req_traces = sqlx::query!(
                "
                select distinct dt.product_id, dt.req_id, dt.filepath, dt.file_hash, dt.line, t.kind
                from TraceCoveragePerTestRuns tc, DirectProductReqTraces dt, Traces t
                where tc.product_id = $1
                and tc.test_run_name = $2 and tc.test_run_date = $3
                and tc.filepath = dt.filepath and tc.file_hash = dt.file_hash
                and tc.traced_line = dt.line and dt.file_hash = t.file_hash
                and dt.line = t.line
                ",
                product_id,
                test_run_name,
                test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?;

            let mut traced_reqs: HashMap<(String, String), Vec<TraceReference>> = HashMap::new();

            for trace in req_traces {
                traced_reqs
                    .entry((trace.product_id, trace.req_id))
                    .or_default()
                    .push(TraceReference {
                        filepath: RelativePathBuf::from(trace.filepath),
                        file_hash: FmtHash::with_inner(trace.file_hash),
                        line: trace.line,
                        kind: TraceKind::try_from(trace.kind)?,
                    });
            }

            traced_reqs
        };

        let mut related_reqs: Vec<TestRelatedRequirementOverview> = traced_reqs
            .into_iter()
            .map(
                |((req_product_id, req_id), traces)| TestRelatedRequirementOverview {
                    product_id: if req_product_id == product_id {
                        None
                    } else {
                        Some(req_product_id)
                    },
                    id: req_id,
                    kind: TestRelatedRequirementKind::Traced(traces),
                },
            )
            .collect();

        if let Some(test_case) = test_case_name {
            let directly_verified_reqs = sqlx::query!(
                "
                select req_id
                from TestCaseVerifiedRequirements
                where product_id = $1
                and test_run_name = $2 and test_run_date = $3
                and test_case_name = $4
                ",
                product_id,
                test_run_name,
                test_run_date,
                test_case
            )
            .fetch_all(self.connection_mut())
            .await?;

            related_reqs.extend(directly_verified_reqs.into_iter().map(|dr| {
                TestRelatedRequirementOverview {
                    product_id: None,
                    id: dr.req_id,
                    kind: TestRelatedRequirementKind::Direct,
                }
            }));
        }

        Ok(if related_reqs.is_empty() {
            None
        } else {
            Some(related_reqs)
        })
    }

    async fn resolve_line_state(
        &mut self,
        resolved_line: &ResolvedLine,
        test_run_name: &str,
        test_run_date: &str,
        test_case_name: Option<&str>,
    ) -> Result<ResolvedCoveredLineState, anyhow::Error> {
        let line_state = if let Some(block_exlusion) = sqlx::query!(
            "
            select cr.start_line, gt.content
            from CoverageBlockExcludes cr left join GeneralTexts gt on cr.comment_hash = gt.hash
            where cr.file_hash = $1 and cr.start_line = $2
            ",
            resolved_line.file_hash,
            resolved_line.line
        )
        .fetch_optional(self.connection_mut())
        .await?
        {
            ResolvedCoveredLineState::Excluded(Some(ExclusionAnnotationReference {
                def_line: block_exlusion.start_line,
                comment: block_exlusion.content,
            }))
        } else if let Some(line_exlusion) = sqlx::query!(
            "
            select cr.line, gt.content
            from CoverageLineExcludes cr left join GeneralTexts gt on cr.comment_hash = gt.hash
            where cr.file_hash = $1 and cr.line = $2
            ",
            resolved_line.file_hash,
            resolved_line.line
        )
        .fetch_optional(self.connection_mut())
        .await?
        {
            ResolvedCoveredLineState::Excluded(Some(ExclusionAnnotationReference {
                def_line: line_exlusion.line,
                comment: line_exlusion.content,
            }))
        } else if let Some(override_state) = self
            .covered_line_override(resolved_line, test_run_name, test_run_date, test_case_name)
            .await?
        {
            override_state
        } else if let Some(cov_hits) = resolved_line.hits {
            if cov_hits > 0 {
                ResolvedCoveredLineState::Covered(cov_hits)
            } else {
                ResolvedCoveredLineState::Uncovered
            }
        } else {
            ResolvedCoveredLineState::Excluded(None)
        };

        Ok(line_state)
    }

    async fn covered_line_override(
        &mut self,
        resolved_line: &ResolvedLine,
        test_run_name: &str,
        test_run_date: &str,
        test_case_name: Option<&str>,
    ) -> Result<Option<ResolvedCoveredLineState>, anyhow::Error> {
        let product_id = self.product_id();

        if let Some(test_case) = test_case_name {
            if let Some(cov_override) = sqlx::query!(
                "
                select co.review_name, co.review_date, co.hits, gt.content
                from TestCaseLineCoverageOverrides co left join GeneralTexts gt on co.comment_hash = gt.hash
                where co.product_id = $1 and co.test_run_name = $2
                and co.test_run_date = $3 and co.test_case_name = $4
                and co.cov_filepath = $5 and co.cov_line = $6
                ",
                product_id,
                test_run_name,
                test_run_date,
                test_case,
                resolved_line.filepath,
                resolved_line.line
            ).fetch_optional(self.connection_mut()).await? {
                return Ok(Some(ResolvedCoveredLineState::Overriden { review: ReviewReference{ name: cov_override.review_name, utc_date: OffsetDateTime::parse(
                    &cov_override.review_date,
                    &mantra_schema::time::format_description::well_known::Iso8601::PARSING,
                )
                .expect("Valid date in database")}, original_hits: resolved_line.hits, set_hits: cov_override.hits, comment: cov_override.content }));
            }
        } else if let Some(cov_override) = sqlx::query!(
            "
            select co.review_name, co.review_date, co.hits, gt.content
            from TestRunLineCoverageOverrides co left join GeneralTexts gt on co.comment_hash = gt.hash
            where co.product_id = $1 and co.test_run_name = $2
            and co.test_run_date = $3
            and co.cov_filepath = $5 and co.cov_line = $6
            ",
            product_id,
            test_run_name,
            test_run_date,
            resolved_line.filepath,
            resolved_line.line
        ).fetch_optional(self.connection_mut()).await? {
            return Ok(Some(ResolvedCoveredLineState::Overriden { review: ReviewReference{ name: cov_override.review_name, utc_date: mantra_schema::reviews::date_from_str(
                &cov_override.review_date,
            )
            .expect("Valid date in database")}, original_hits: resolved_line.hits, set_hits: cov_override.hits, comment: cov_override.content }));
        }

        Ok(None)
    }

    async fn test_related_coverage(
        &mut self,
        test_run_name: &str,
        test_run_date: &str,
        test_case_name: Option<&str>,
    ) -> Result<Option<TestCoverageOverview>, anyhow::Error> {
        let product_id = self.product_id();

        let coverable_lines_record = sqlx::query!(
            r#"
            select sum(coverable_lines) as "coverable_lines!:i64"
            from CoverableLinesPerFilepath
            where product_id = $1
            "#,
            product_id
        )
        .fetch_one(self.connection_mut())
        .await?;

        let mut test_summary = TestCoverageSummary {
            lines: CoveredLinesSummary {
                total: coverable_lines_record.coverable_lines,
                ..Default::default()
            },
        };

        let (resolved_lines, covered_traces) = if let Some(test_case) = test_case_name {
            let resolved_lines: Vec<ResolvedLine> = sqlx::query!(
                "
                select cov_filepath, cov_file_hash, cov_line, hits
                from ResolvedTestCaseLineCoverage
                where product_id = $1 and test_run_name = $2
                and test_run_date = $3 and test_case_name = $4
                ",
                product_id,
                test_run_name,
                test_run_date,
                test_case
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|l| ResolvedLine {
                filepath: l.cov_filepath,
                file_hash: l.cov_file_hash,
                line: l.cov_line,
                hits: l.hits,
            })
            .collect();

            let covered_traces: Vec<TraceReference> = sqlx::query!(
                "
                select distinct tc.filepath, tc.file_hash, traced_line, kind
                from TraceCoveragePerTestCases tc, Traces t
                where tc.product_id = $1 and tc.test_run_name = $2
                and tc.test_run_date = $3 and tc.test_case_name = $4
                and tc.file_hash = t.file_hash and tc.traced_line = t.line
                ",
                product_id,
                test_run_name,
                test_run_date,
                test_case
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|t| TraceReference {
                filepath: RelativePathBuf::from(t.filepath),
                file_hash: FmtHash::with_inner(t.file_hash),
                line: t.traced_line,
                kind: TraceKind::try_from(t.kind).expect("Valid trace kind in database"),
            })
            .collect();

            (
                resolved_lines,
                if covered_traces.is_empty() {
                    None
                } else {
                    Some(covered_traces)
                },
            )
        } else {
            let resolved_lines = sqlx::query!(
                "
                select cov_filepath, cov_file_hash, cov_line, hits
                from ResolvedTestRunLineCoverage
                where product_id = $1 and test_run_name = $2
                and test_run_date = $3
                ",
                product_id,
                test_run_name,
                test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|l| ResolvedLine {
                filepath: l.cov_filepath,
                file_hash: l.cov_file_hash,
                line: l.cov_line,
                hits: l.hits,
            })
            .collect();

            let covered_traces: Vec<TraceReference> = sqlx::query!(
                "
                select distinct tc.filepath, tc.file_hash, traced_line, kind
                from TraceCoveragePerTestRuns tc, Traces t
                where tc.product_id = $1 and tc.test_run_name = $2
                and tc.test_run_date = $3
                and tc.file_hash = t.file_hash and tc.traced_line = t.line
                ",
                product_id,
                test_run_name,
                test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|t| TraceReference {
                filepath: RelativePathBuf::from(t.filepath),
                file_hash: FmtHash::with_inner(t.file_hash),
                line: t.traced_line,
                kind: TraceKind::try_from(t.kind).expect("Valid trace kind in database"),
            })
            .collect();

            (
                resolved_lines,
                if covered_traces.is_empty() {
                    None
                } else {
                    Some(covered_traces)
                },
            )
        };

        let mut resolved_files =
            HashMap::<String, (Option<String>, HashMap<Line, ResolvedCoveredLine>)>::new();

        for resolved_line in resolved_lines {
            let line_state = self
                .resolve_line_state(&resolved_line, test_run_name, test_run_date, test_case_name)
                .await?;

            let entry = resolved_files
                .entry(resolved_line.filepath.clone())
                .or_default();
            entry.0 = resolved_line.file_hash;
            entry
                .1
                .entry(resolved_line.line)
                .and_modify(|_| {
                    log::warn!(
                        "Multiple resolved line coverage entries for line '{}' in file '{}'",
                        resolved_line.line,
                        resolved_line.filepath
                    )
                })
                .or_insert(ResolvedCoveredLine {
                    nr: resolved_line.line,
                    state: line_state,
                });
        }

        let mut covered_files = Vec::with_capacity(resolved_files.len());

        for (filepath, (file_hash, lines_map)) in resolved_files {
            let lines: Vec<ResolvedCoveredLine> = lines_map.into_values().collect();

            let lines_record = sqlx::query!(
                r#"
                select coverable_lines
                from CoverableLinesPerFilepath
                where product_id = $1 and filepath = $2
                "#,
                product_id,
                filepath
            )
            .fetch_one(self.connection_mut())
            .await?;

            let mut lines_summary = CoveredLinesSummary {
                total: lines_record.coverable_lines,
                ..Default::default()
            };

            for line in &lines {
                match line.state {
                    ResolvedCoveredLineState::Covered(_) => lines_summary.covered.cnt += 1,
                    ResolvedCoveredLineState::Excluded(_) => lines_summary.excluded.cnt += 1,
                    ResolvedCoveredLineState::Overriden {
                        review: _,
                        original_hits: _,
                        set_hits: _,
                        comment: _,
                    } => lines_summary.overridden.cnt += 1,
                    ResolvedCoveredLineState::Uncovered => lines_summary.uncovered.cnt += 1,
                }
            }

            let uncovered_cnt = lines_summary.total
                - (lines_summary.covered.cnt
                    + lines_summary.excluded.cnt
                    + lines_summary.overridden.cnt);
            if lines_summary.uncovered.cnt < uncovered_cnt {
                log::warn!(
                    "Missing line coverage data in file '{}' for '{}' lines.",
                    &filepath,
                    uncovered_cnt - lines_summary.uncovered.cnt
                );
            } else if lines_summary.uncovered.cnt > uncovered_cnt {
                log::warn!(
                    "Too many line coverage entries in file '{}' for '{}' lines.",
                    &filepath,
                    lines_summary.uncovered.cnt - uncovered_cnt
                );
            }

            lines_summary.covered.percentage =
                percentage!(lines_summary.covered.cnt, lines_summary.total);
            lines_summary.excluded.percentage =
                percentage!(lines_summary.excluded.cnt, lines_summary.total);
            lines_summary.overridden.percentage =
                percentage!(lines_summary.overridden.cnt, lines_summary.total);
            lines_summary.uncovered.percentage =
                percentage!(lines_summary.uncovered.cnt, lines_summary.total);

            test_summary.lines.covered.cnt += lines_summary.covered.cnt;
            test_summary.lines.excluded.cnt += lines_summary.excluded.cnt;
            test_summary.lines.overridden.cnt += lines_summary.overridden.cnt;
            test_summary.lines.uncovered.cnt += lines_summary.uncovered.cnt;

            covered_files.push(ResolvedCoveredFile {
                filepath: RelativePathBuf::from(filepath),
                file_hash: file_hash.map(FmtHash::with_inner),
                lines: ResolvedCoveredLines {
                    summary: lines_summary,
                    lines,
                },
            })
        }

        test_summary.lines.covered.percentage =
            percentage!(test_summary.lines.covered.cnt, test_summary.lines.total);
        test_summary.lines.excluded.percentage =
            percentage!(test_summary.lines.excluded.cnt, test_summary.lines.total);
        test_summary.lines.overridden.percentage =
            percentage!(test_summary.lines.overridden.cnt, test_summary.lines.total);
        test_summary.lines.uncovered.percentage =
            percentage!(test_summary.lines.uncovered.cnt, test_summary.lines.total);

        Ok(if covered_files.is_empty() {
            None
        } else {
            // TODO: consider coverage metrics from child test runs and test cases for test runs
            Some(TestCoverageOverview {
                summary: test_summary,
                covered_files,
                covered_traces,
            })
        })
    }

    async fn test_cases_overview(
        &mut self,
        test_run_name: &str,
        test_run_date: &str,
    ) -> Result<Option<TestCasesOverview>, anyhow::Error> {
        let product_id = self.product_id();

        let entries = sqlx::query!(
            "
            select test_case_name, state
            from ResolvedTestCaseStates
            where product_id = $1
            and test_run_name = $2
            and test_run_date = $3
            ",
            product_id,
            test_run_name,
            test_run_date
        )
        .fetch_all(self.connection_mut())
        .await?;

        let mut summary = TestCasesSummary {
            total: entries.len() as i64,
            ..Default::default()
        };
        let mut test_cases = Vec::with_capacity(entries.len());

        for entry in entries {
            let state = TestState::try_from(entry.state)?;

            match state {
                TestState::Failed => summary.failed.cnt += 1,
                TestState::Passed => summary.passed.cnt += 1,
                TestState::Skipped => summary.skipped.cnt += 1,
                TestState::Unknown => summary.unknown.cnt += 1,
                TestState::Obsolete => summary.obsolete.cnt += 1,
            }

            let location = sqlx::query!(
                "
                select filepath, file_hash, line
                from TestCaseLocations
                where product_id = $1 and test_run_name = $2
                and test_run_date = $3 and test_case_name = $4
                ",
                product_id,
                test_run_name,
                test_run_date,
                entry.test_case_name
            )
            .fetch_optional(self.connection_mut())
            .await?
            .map(|l| TestCaseLocation {
                filepath: RelativePathBuf::from(l.filepath),
                file_hash: l.file_hash.map(|f| FmtHash::with_inner(f)),
                line: l.line,
            });

            let related_reqs = self
                .test_related_requirements(
                    test_run_name,
                    test_run_date,
                    Some(&entry.test_case_name),
                )
                .await?;
            let coverage = self
                .test_related_coverage(test_run_name, test_run_date, Some(&entry.test_case_name))
                .await?;

            test_cases.push(TestCaseOverview {
                name: entry.test_case_name,
                state,
                location,
                related_reqs,
                coverage,
            })
        }

        summary.failed.percentage = percentage!(summary.failed.cnt, summary.total);
        summary.obsolete.percentage = percentage!(summary.obsolete.cnt, summary.total);
        summary.passed.percentage = percentage!(summary.passed.cnt, summary.total);
        summary.skipped.percentage = percentage!(summary.skipped.cnt, summary.total);
        summary.unknown.percentage = percentage!(summary.unknown.cnt, summary.total);

        Ok(if test_cases.is_empty() {
            None
        } else {
            Some(TestCasesOverview {
                summary,
                all: test_cases,
            })
        })
    }

    async fn reviews_overview(&mut self) -> Result<ReviewsOverview, anyhow::Error> {
        let product_id = self.product_id();

        let reviews = sqlx::query!(
            "
            select r.name, r.utc_date, gt.content
            from Reviews r left join GeneralTexts gt on r.description_hash = gt.hash
            where r.product_id = $1
            ",
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        let nr_manual_requirements = sqlx::query!(
            r#"
            select count(*) as "nr!:i64"
            from ManualRequirements
            where product_id = $1
            "#,
            product_id
        )
        .fetch_one(self.connection_mut())
        .await?
        .nr;

        let manual_reqs_verified = sqlx::query!(
            r#"
            select count(*) as "nr!:i64"
            from ManualRequirements
            where product_id = $1
            and id in (
                select req_id
                from ManuallyVerifiedRequirements
                where product_id = $1
            )
            "#,
            product_id
        )
        .fetch_one(self.connection_mut())
        .await?
        .nr;

        let summary = ReviewsSummary {
            total: reviews.len() as i64,
            // TODO: change once obsolete check is added
            valid: Aggregated {
                cnt: reviews.len() as i64,
                percentage: 100.0,
            },
            mandatory_requirements_verified: Aggregated {
                cnt: manual_reqs_verified,
                percentage: percentage!(manual_reqs_verified, nr_manual_requirements),
            },
            ..Default::default()
        };
        let mut review_overviews = Vec::with_capacity(reviews.len());

        for review in reviews {
            let authors = sqlx::query!(
                "
                select author from ReviewAuthors
                where product_id = $1
                and review_name = $2
                and review_date = $3
                ",
                product_id,
                review.name,
                review.utc_date
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|a| a.author)
            .collect();

            let requirements: Vec<VerifiedRequirementOverview> = sqlx::query!(
                "
                select vr.req_id, gt.content
                from ManuallyVerifiedRequirements vr left join GeneralTexts gt on vr.comment_hash = gt.hash
                where vr.product_id = $1
                and vr.review_name = $2
                and vr.review_date = $3",
                product_id,
                review.name,
                review.utc_date
            )
            .fetch_all(self.connection_mut())
            .await?
            .into_iter()
            .map(|r|
                VerifiedRequirementOverview{ id: r.req_id, comment: r.content }
            )
            .collect();

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
                product_id,
                review.name,
                review.utc_date
            )
            .fetch_all(self.connection_mut())
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
                    product_id,
                    test_run.test_run_name,
                    test_run.test_run_date
                )
                .fetch_all(self.connection_mut())
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
                        product_id,
                        test_run.test_run_name,
                        test_run.test_run_date,
                        test_case.test_case_name
                    )
                    .fetch_optional(self.connection_mut())
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
                        product_id,
                        test_run.test_run_name,
                        test_run.test_run_date,
                        test_case.test_case_name
                    )
                    .fetch_all(self.connection_mut())
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
                            product_id,
                            test_run.test_run_name,
                            test_run.test_run_date,
                            test_case.test_case_name,
                            filepath.cov_filepath
                        )
                        .fetch_all(self.connection_mut())
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
                        product_id,
                        test_run.test_run_name,
                        test_run.test_run_date,
                    )
                    .fetch_one(self.connection_mut())
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
                    product_id,
                    test_run.test_run_name,
                    test_run.test_run_date
                )
                .fetch_all(self.connection_mut())
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
                        product_id,
                        test_run.test_run_name,
                        test_run.test_run_date,
                        filepath.cov_filepath
                    )
                    .fetch_all(self.connection_mut())
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

            review_overviews.push(ReviewOverview {
                name: review.name,
                utc_date: mantra_schema::reviews::date_from_str(&review.utc_date)
                    .expect("Valid date in database"),
                authors,
                requirements: if requirements.is_empty() {
                    None
                } else {
                    Some(requirements)
                },
                test_run_overrides: if test_run_overrides.is_empty() {
                    None
                } else {
                    Some(test_run_overrides)
                },
                description: review.content,
            });
        }

        Ok(ReviewsOverview {
            summary,
            all: review_overviews,
        })
    }
}

struct ResolvedLine {
    filepath: String,
    file_hash: Option<String>,
    line: Line,
    hits: Option<i64>,
}
