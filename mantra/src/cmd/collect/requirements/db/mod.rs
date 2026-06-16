use std::str::FromStr;

use anyhow::bail;
use mantra_schema::{
    FmtHash, Properties,
    path::RelativePath,
    product::ProductId,
    requirements::{ReqId, Requirement, RequirementSchema},
};

use crate::cmd::collect::Collection;
use crate::cmd::collect::merge_local_and_base_properties;

pub mod aggregate;

impl<'db> Collection<'db> {
    pub(crate) async fn update_per_req_schema(
        &mut self,
        filepath: &RelativePath,
        req_schema: RequirementSchema,
    ) -> Result<(), anyhow::Error> {
        let base_origin_hash = req_schema.origin.as_ref().map(FmtHash::from);

        if let Some(hash) = &base_origin_hash
            && let Some(origin) = req_schema.origin
        {
            self.insert_general_json(hash, origin.clone()).await?;
        }

        // TODO: do not stop at first collect error

        for req in req_schema.requirements {
            self.update_requirement(filepath, req, &base_origin_hash, &req_schema.properties)
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn update_req_dot_hierarchy(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        // Only update newly collected requirements that have dots '.' in their ID.
        let records = sqlx::query!(
            "
                select id from Requirements
                where product_id = $1 and last_collect_nr = $2
                and instr(id, '.') > 0
            ",
            product_id,
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await?;

        let mut missing_parent = Vec::new();

        for record in records {
            // TODO: log missing dot-parent
            // Only allow parent to be collected in the same (latest) collection.
            if let Some(parent_id) = self
                .get_dot_parent(&product_id, collect_nr, &ReqId::from_str(&record.id)?)
                .await
            {
                sqlx::query!(
                    "
                    insert into RequirementHierarchies (
                        last_collect_nr,
                        child_product_id,
                        child_req_id,
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
                    on conflict (child_product_id, child_req_id, parent_product_id, parent_req_id)
                    do update set
                        last_collect_nr = excluded.last_collect_nr
                    ",
                    collect_nr,
                    product_id,
                    record.id,
                    product_id,
                    parent_id
                )
                .execute(self.connection_mut())
                .await?;
            } else {
                missing_parent.push(record.id);
            }
        }

        if !missing_parent.is_empty() {
            for bad in missing_parent {
                eprintln!("Parent of requirement '{bad}' was not collected!");
            }
            anyhow::bail!("Missing parent requirement!");
        }

        Ok(())
    }

    pub(crate) async fn delete_outdated_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        let updated_records = sqlx::query!(
            "
                select id from Requirements
                where product_id = $1 and last_collect_nr = $2
            ",
            product_id,
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await?;

        // Note: always deleting outdated data for collected reqs,
        // because this means that the data got removed in the original source.
        for record in updated_records {
            sqlx::query!(
                "
                delete from RequirementProperties
                where product_id = $1 and req_id = $2
                and last_collect_nr < $3
            ",
                product_id,
                record.id,
                collect_nr
            )
            .execute(self.connection_mut())
            .await?;

            sqlx::query!(
                "
                delete from RequirementHierarchies
                where ((child_product_id = $1 and child_req_id = $2)
                or (parent_product_id = $1 and parent_req_id = $2))
                and last_collect_nr < $3
            ",
                product_id,
                record.id,
                collect_nr
            )
            .execute(self.connection_mut())
            .await?;
        }

        // Check bad req-hierarchies before deleting olds.
        // Only parent IDs relevant, because the hierarchy is entered when collecting the child
        // Bad ones are:
        // - same product id, but parent was not collected in a run before the child
        //   indicates that the parent got deleted
        // - different product id, but the parent req was not collected in latest run for the product
        //   indicates that the parent got deleted
        let bad_hierarchies = sqlx::query!(
            "
                select rh.child_req_id, rh.parent_req_id
                from RequirementHierarchies rh, Requirements r
                where r.id = rh.parent_req_id
                and r.product_id = rh.parent_product_id
                and r.last_collect_nr < rh.last_collect_nr
                and rh.child_product_id = $1
                and rh.parent_product_id = $1
                union
                select rh.child_req_id, rh.parent_req_id
                from RequirementHierarchies rh, Requirements r, Products p
                where r.id = rh.parent_req_id
                and r.product_id = rh.parent_product_id
                and r.last_collect_nr != rh.last_collect_nr
                and rh.child_product_id = $1
                and rh.parent_product_id != rh.child_product_id
                and p.id = rh.parent_product_id
                and r.last_collect_nr != p.last_collect_nr
            ",
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        if !bad_hierarchies.is_empty() {
            for bad in bad_hierarchies {
                eprintln!(
                    "Bad requirement hierarchy! Child '{}' references deleted parent '{}'",
                    bad.child_req_id, bad.parent_req_id
                );
            }
            anyhow::bail!("Bad requirement hierarchy detected!");
        }

        // Note: due to cascade rules, deletions in the base Requirements table
        // cascade to the other tables
        sqlx::query!(
            "
            delete from Requirements
            where product_id = $1 and last_collect_nr < $2
        ",
            product_id,
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_requirement(
        &mut self,
        filepath: &RelativePath,
        req: Requirement,
        base_origin_hash: &Option<FmtHash>,
        base_props: &Option<Properties>,
    ) -> Result<(), anyhow::Error> {
        // TODO: optimize by checking src-hash first and skip if unchanged

        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();

        if let Some(record) = sqlx::query!(
            "
            select id, data_filepath
            from Requirements
            where last_collect_nr = $1 and product_id = $2
            and id = $3
            ",
            collect_nr,
            product_id,
            req.id
        )
        .fetch_optional(self.connection_mut())
        .await?
        {
            bail!(
                "Duplicate requirement ID '{}' found in the same collection! Duplicate definition in '{}'; Previous definition in '{}'.",
                req.id,
                filepath,
                record.data_filepath
            );
        }

        let data_hash = FmtHash::from(&serde_json::json!({
            "base_origin_hash": base_origin_hash,
            "base_props": base_props,
            "req": &req
        }));
        let data_filepath = filepath.as_str();

        let origin_hash = FmtHash::from(&req.origin);
        self.insert_general_json(&origin_hash, req.origin).await?;
        let description_hash = req.description.as_ref().map(FmtHash::from);
        if let Some(hash) = &description_hash
            && let Some(description) = req.description
        {
            self.insert_general_text(hash, description, None).await?;
        }

        sqlx::query!(
            "
            insert into Requirements (
                last_collect_nr,
                id,
                product_id,
                manual_verification,
                deprecated,
                ignore,
                optional,
                title,
                base_origin_hash,
                origin_hash,
                description_hash,
                data_hash,
                data_filepath
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
            on conflict (id, product_id)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                manual_verification = excluded.manual_verification,
                deprecated = excluded.deprecated,
                ignore = excluded.ignore,
                optional = excluded.optional,
                title = excluded.title,
                base_origin_hash = excluded.base_origin_hash,
                origin_hash = excluded.origin_hash,
                description_hash = excluded.description_hash,
                data_hash = excluded.data_hash,
                data_filepath = excluded.data_filepath
            ",
            collect_nr,
            req.id,
            product_id,
            req.manual_verification,
            req.deprecated,
            req.ignore,
            req.optional,
            req.title,
            base_origin_hash,
            origin_hash,
            description_hash,
            data_hash,
            data_filepath
        )
        .execute(self.connection_mut())
        .await?;

        if let Some(props) = merge_local_and_base_properties(req.properties, base_props) {
            for prop in props {
                let value_hash = FmtHash::from(&prop.1);
                self.insert_general_json(&value_hash, prop.1).await?;

                sqlx::query!(
                    "
                    insert into RequirementProperties (
                        last_collect_nr,
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
                    on conflict (req_id, product_id, property_key)
                    do update set
                        last_collect_nr = excluded.last_collect_nr,
                        value_hash = excluded.value_hash
                    ",
                    collect_nr,
                    req.id,
                    product_id,
                    prop.0,
                    value_hash
                )
                .execute(self.connection_mut())
                .await?;
            }
        }

        if let Some(parents) = req.parents {
            for parent in parents {
                let parent_product_id = parent.product_id.unwrap_or(self.product_id());

                // Safe to insert, because foreign key constraints are deferred for the hierarchy.
                // Constraint will be checked when committing the transaction
                sqlx::query!(
                    "
                    insert into RequirementHierarchies (
                        last_collect_nr,
                        child_product_id,
                        child_req_id,
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
                    on conflict (child_product_id, child_req_id, parent_product_id, parent_req_id)
                    do update set
                        last_collect_nr = excluded.last_collect_nr
                    ",
                    collect_nr,
                    product_id,
                    req.id,
                    parent_product_id,
                    parent.id
                )
                .execute(self.connection_mut())
                .await?;
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
                    and last_collect_nr = $3
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
