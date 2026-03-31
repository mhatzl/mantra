use mantra_schema::report::{
    RequirementReference,
    short::{RequirementOverview, ReviewOverview, ShortProductReport, TestRunOverview},
};

use crate::cmd::report::ProductReporter;

impl<'t, 'db> ProductReporter<'t, 'db> {
    pub async fn short_report(mut self) -> Result<ShortProductReport, anyhow::Error> {
        let requirements = self.requirements_overview().await?;
        let test_runs = self.test_runs_overview().await?;
        let reviews = self.reviews_overview().await?;

        Ok(ShortProductReport {
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
        Ok(vec![])
    }

    async fn reviews_overview(&mut self) -> Result<Vec<ReviewOverview>, anyhow::Error> {
        Ok(vec![])
    }
}
