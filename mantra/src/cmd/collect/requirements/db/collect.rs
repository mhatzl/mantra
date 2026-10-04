use std::str::FromStr;

use anyhow::Context;
use mantra_schema::{
    FmtHash,
    product::ProductId,
    requirements::{ReqId, Requirement, RequirementSchema},
};

use crate::cmd::collect::product_collection::ProductCollection;

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn update_per_req_schema(
        &mut self,
        req_schema: &RequirementSchema,
        schema_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        let schema_collected = sqlx::query!(
            "
            select *
            from Schemas
            where content_hash = $1
            ",
            schema_hash
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to get collected schemas")?
        .is_some();

        if !schema_collected && let Some(props) = &req_schema.properties {
            for (key, value) in props {
                let value_hash = FmtHash::from(&value);

                self.insert_general_json(&value_hash, &value)
                    .await
                    .with_context(|| {
                        format!("Inserting value for requirement schema property '{key}'")
                    })?;

                sqlx::query!(
                    "
                    insert into SchemaRequirementProperties (
                        schema_hash,
                        property_key,
                        value_hash
                    )
                    values (
                        $1,
                        $2,
                        $3
                    )
                    ",
                    schema_hash,
                    key,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Inserting requirement schema property '{key}'"))?;
            }
        }

        // TODO: do not stop at first collect error

        for req in &req_schema.requirements {
            let req_id = req.id.clone();
            self.update_requirement(&schema_hash, req)
                .await
                .with_context(|| format!("Failed to update requirement '{}'", req_id))?;
        }

        Ok(())
    }

    pub(crate) async fn update_req_dot_parent(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        // Only update newly collected requirements that have dots '.' in their ID.
        let records = sqlx::query!(
            "
                select id from Requirements
                where product_id = $1 and collect_nr = $2
                and instr(id, '.') > 0
            ",
            product_id,
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Failed to get collected dot-requirements")?;

        let mut missing_parent = Vec::new();

        for record in records {
            if let Some(parent_id) = self
                .get_dot_parent(
                    &product_id,
                    collect_nr,
                    &ReqId::from_str(&record.id).with_context(|| {
                        format!("Received invalid requirement ID '{}'", record.id)
                    })?,
                )
                .await
            {
                sqlx::query!(
                    "
                    insert into RequirementParents (
                        collect_nr,
                        product_id,
                        req_id,
                        parent_product_id,
                        parent_req_id,
                        optional
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5,
                        false
                    )
                    ",
                    collect_nr,
                    product_id,
                    record.id,
                    product_id,
                    parent_id
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| {
                    format!(
                        "Failed updating dot-hierarchy for requirement '{}'",
                        record.id
                    )
                })?;
            } else {
                missing_parent.push(record.id);
            }
        }

        if !missing_parent.is_empty() {
            for bad in missing_parent {
                log::error!("Parent of requirement '{bad}' was not collected!");
            }
            anyhow::bail!("Missing parent requirement!");
        }

        Ok(())
    }

    async fn update_requirement(
        &mut self,
        schema_hash: &FmtHash,
        req: &Requirement,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        if let Some(dupl_req) = sqlx::query!(
            "
                select r.id, gj.content
                from Requirements r, GeneralJson gj
                where r.collect_nr = $1 and r.product_id = $2
                and r.id = $3 and r.origin_hash = gj.hash
                ",
            collect_nr,
            product_id,
            req.id
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Failed to check for duplicate requirement entries")?
        {
            anyhow::bail!(
                "Duplicate requirement ID '{}' found in the same collection! Previous origin: {}",
                req.id,
                dupl_req.content // origin JSON
            );
        }

        let data_hash = FmtHash::from(&req);

        let origin_hash = FmtHash::from(&req.origin);
        self.insert_general_json(&origin_hash, &req.origin)
            .await
            .context("Failed to insert the requirement origin")?;

        let description_hash = req.description.as_ref().map(FmtHash::from);
        if let Some(hash) = &description_hash
            && let Some(description) = &req.description
        {
            self.insert_general_text(hash, description)
                .await
                .context("Failed to insert the requirement description")?;
        }

        sqlx::query!(
            "
            insert into Requirements (
                collect_nr,
                id,
                product_id,
                manual_verification,
                deprecated,
                exclude,
                optional,
                title,
                origin_hash,
                description_hash,
                data_hash,
                schema_hash,
                media_type
            )
            values (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10,
                $11,
                $12,
                $13
            )
            ",
            collect_nr,
            req.id,
            product_id,
            req.manual_verification,
            req.deprecated,
            req.exclude,
            req.optional,
            req.title,
            origin_hash,
            description_hash,
            data_hash,
            schema_hash,
            req.media_type
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update the requirement base data")?;

        if let Some(props) = &req.properties {
            for prop in props {
                let key = &prop.0;
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1)
                    .await
                    .with_context(|| format!("Failed to insert content for property '{}'", key))?;

                sqlx::query!(
                    "
                    insert into RequirementProperties (
                        collect_nr,
                        req_id,
                        product_id,
                        property_key,
                        value_hash
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5
                    )
                    ",
                    collect_nr,
                    req.id,
                    product_id,
                    key,
                    value_hash
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| format!("Failed to update requirement property '{}'", key))?;
            }
        }

        if let Some(parents) = &req.parents {
            for parent in parents {
                let parent_product_id = parent.product_id.clone().unwrap_or(product_id.clone());

                // Safe to insert, because no foreign key constraints for parent requirements.
                // Will be checked during aggregation.
                sqlx::query!(
                    "
                    insert into RequirementParents (
                        collect_nr,
                        product_id,
                        req_id,
                        parent_product_id,
                        parent_req_id
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5
                    )
                    ",
                    collect_nr,
                    product_id,
                    req.id,
                    parent_product_id,
                    parent.id
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| {
                    format!(
                        "Failed to update relationship with parent requirement '{}'",
                        parent.id
                    )
                })?;
            }
        }

        if let Some(children) = &req.children {
            for child in children {
                let child_product_id = child.product_id.clone().unwrap_or(product_id.clone());

                // Safe to insert, because no foreign key constraints for parent requirements.
                // Will be checked during aggregation.
                sqlx::query!(
                    "
                    insert into RequirementChildren (
                        collect_nr,
                        product_id,
                        req_id,
                        child_product_id,
                        child_req_id
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4,
                        $5
                    )
                    ",
                    collect_nr,
                    product_id,
                    req.id,
                    child_product_id,
                    child.id
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| {
                    format!(
                        "Failed to update relationship with child requirement '{}'",
                        child.id
                    )
                })?;
            }
        }

        if let Some(replaced_reqs) = &req.replaces {
            for replaced_req in replaced_reqs {
                sqlx::query!(
                    "
                    insert into RequirementReplacements (
                        collect_nr,
                        product_id,
                        req_id,
                        replaced_req_id
                    )
                    values (
                        $1,
                        $2,
                        $3,
                        $4
                    )
                    ",
                    collect_nr,
                    product_id,
                    req.id,
                    replaced_req
                )
                .execute(self.connection_mut())
                .await
                .with_context(|| {
                    format!(
                        "Failed to update requirement replacement for requirement '{}'",
                        replaced_req
                    )
                })?;
            }
        }

        Ok(())
    }

    async fn get_dot_parent(
        &mut self,
        product_id: &ProductId,
        collect_nr: i64,
        req_id: &ReqId,
    ) -> Option<ReqId> {
        let mut req_id = req_id.as_str();
        while let Some((parent, _)) = req_id.rsplit_once('.') {
            let parent_exists = self
                .req_exists(product_id, collect_nr, &ReqId::from_str(parent).ok()?)
                .await;

            if parent_exists {
                return ReqId::from_str(parent).ok();
            } else {
                req_id = parent;
            }
        }

        None
    }

    async fn req_exists(
        &mut self,
        product_id: &ProductId,
        collect_nr: i64,
        req_id: &ReqId,
    ) -> bool {
        sqlx::query!(
            "
            select id from Requirements
            where product_id = $1 and id = $2
            and collect_nr = $3
            ",
            product_id,
            req_id,
            collect_nr
        )
        .fetch_one(self.connection_mut())
        .await
        .is_ok()
    }
}
