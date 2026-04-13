use mantra_schema::{
    report::{
        RequirementReference, TestRunReference,
        overview::{
            ProductOverviewReport, RequirementOverview, RequirementsOverview, ReviewOverview,
            TestCaseOverview, TestRunOverview,
        },
    },
    time::OffsetDateTime,
};

use crate::cmd::report::ProductReporter;

impl<'t, 'db> ProductReporter<'t, 'db> {
    pub async fn short_report(mut self) -> Result<ProductOverviewReport, anyhow::Error> {
        let all_requirements = self.requirements_overview().await?;
        let roots = all_requirements
            .iter()
            .filter(|r| r.parents == None)
            .cloned()
            .collect();
        let requirements = RequirementsOverview {
            roots,
            all: all_requirements,
        };

        let test_runs = self.test_runs_overview().await?;
        let reviews = self.reviews_overview().await?;

        Ok(ProductOverviewReport {
            product: self.product,
            requirements,
            test_runs,
            reviews,
        })
    }

    async fn requirements_overview(&mut self) -> Result<Vec<RequirementOverview>, anyhow::Error> {
        let product_id = self.product_id();

        let record = sqlx::query!(
            "
            select
                r.id,
                r.title,
                rs.state,
                r.optional
            from Requirements r, RequirementVerificationStates rs
            where r.product_id = $1 and rs.product_id = $1
            and r.id = rs.id
            ",
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        let mut requirements = Vec::with_capacity(record.len());

        for entry in record {
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

            requirements.push(RequirementOverview {
                id: entry.id,
                title: entry.title,
                state: entry.state.try_into()?,
                optional: entry.optional,
                parents,
                children,
            })
        }

        Ok(requirements)
    }

    async fn test_runs_overview(&mut self) -> Result<Vec<TestRunOverview>, anyhow::Error> {
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

        let mut test_runs = Vec::with_capacity(test_run_states.len());

        for tr in test_run_states {
            let test_cases = sqlx::query!(
                "
                select test_case_name, state
                from ResolvedTestCaseStates
                where product_id = $1
                and test_run_name = $2
                and test_run_date = $3
                ",
                product_id,
                tr.test_run_name,
                tr.test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?;

            let parents = sqlx::query!(
                "select parent_name, parent_date
                from TestRunHierarchies
                where product_id = $1
                and child_name = $2
                and child_date = $3",
                product_id,
                tr.test_run_name,
                tr.test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?;

            let children = sqlx::query!(
                "select child_name, child_date
                from TestRunHierarchies
                where product_id = $1
                and parent_name = $2
                and parent_date = $3",
                product_id,
                tr.test_run_name,
                tr.test_run_date
            )
            .fetch_all(self.connection_mut())
            .await?;

            test_runs.push(TestRunOverview {
                name: tr.test_run_name,
                utc_date: OffsetDateTime::parse(
                    &tr.test_run_date,
                    &mantra_schema::time::format_description::well_known::Iso8601::PARSING,
                )
                .expect("Valid test date in database"),
                state: tr.state.try_into().expect("Valid test state in database"),
                test_cases: if test_cases.is_empty() {
                    None
                } else {
                    Some(
                        test_cases
                            .into_iter()
                            .map(|tc| TestCaseOverview {
                                name: tc.test_case_name,
                                state: tc
                                    .state
                                    .try_into()
                                    .expect("Test state in the database must be valid"),
                            })
                            .collect(),
                    )
                },
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
                                .expect("Valid test date in database"),
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
                            })
                            .collect(),
                    )
                },
            })
        }

        Ok(test_runs)
    }

    async fn reviews_overview(&mut self) -> Result<Vec<ReviewOverview>, anyhow::Error> {
        Ok(vec![])
    }
}
