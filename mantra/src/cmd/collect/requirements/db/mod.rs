use mantra_schema::{product::ProductId, requirements::Requirement};

use crate::{
    cmd::collect::db::CollectTransaction,
    db::{ContentHash, FmtHash, Hash},
};

#[cfg(test)]
mod test;

impl<'db> CollectTransaction<'db> {
    pub async fn update_requirements(
        &'db mut self,
        product_id: &ProductId,
        requirements: &[Requirement],
    ) -> Result<FmtHash, anyhow::Error> {
        let section_hash = requirements.fmt_hash();
        let section_ref = &section_hash;

        sqlx::query!(
            "
            insert or ignore into CollectedSections (hash)
            values ($1)
            ",
            section_ref
        )
        .execute(&mut *self.connection())
        .await?;

        for req in requirements {
            let fmt_title_hash = &req.title.hash().finalize();
            let fmt_origin_hash = &req.origin.hash().finalize();
            let fmt_description_hash = &req.description.hash().finalize();
            let details_hash = req_details_hash(req);
            let fmt_details_hash = &details_hash.clone().finalize();
            let properties_hash = req_properties_hash(req);
            let fmt_properties_hash = &properties_hash.clone().finalize();
            let hierarchies_hash = req_hierarchies_hash(req);
            let fmt_hierarchies_hash = &hierarchies_hash.clone().finalize();
            let content_hash = req_content_hash(details_hash, properties_hash, hierarchies_hash);
            let fmt_content_hash = &content_hash.finalize();

            sqlx::query!(
                "
                insert or ignore into Requirements (id, product_id)
                values ($1, $2)
                ",
                req.id,
                product_id,
            )
            .execute(&mut *self.connection())
            .await?;

            sqlx::query!(
                "
                insert or ignore into RequirementCollections (
                    section_hash,
                    product_id,
                    req_id,
                    req_content_hash
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4
                )
                ",
                section_ref,
                product_id,
                req.id,
                fmt_content_hash
            )
            .execute(&mut *self.connection())
            .await?;

            let hierarchies_hash = if req.parents.is_some() {
                Some(fmt_hierarchies_hash)
            } else {
                None
            };
            sqlx::query!(
                "
                insert or ignore into RequirementContents (
                    hash,
                    req_details_hash,
                    req_properties_hash,
                    req_hierarchies_hash,
                    manual_verification,
                    deprecated
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6
                )
                ",
                fmt_content_hash,
                fmt_details_hash,
                fmt_properties_hash,
                hierarchies_hash,
                req.manual_verification,
                req.deprecated
            )
            .execute(&mut *self.connection())
            .await?;

            sqlx::query!(
                "
                insert or ignore into GeneralContents (
                    hash,
                    content
                )
                values (
                    $1,
                    $2
                )
                ",
                fmt_title_hash,
                req.title
            )
            .execute(&mut *self.connection())
            .await?;

            if let Some(description) = &req.description {
                sqlx::query!(
                    "
                    insert or ignore into GeneralContents (
                        hash,
                        content
                    )
                    values (
                        $1,
                        $2
                    )
                    ",
                    fmt_description_hash,
                    description
                )
                .execute(&mut *self.connection())
                .await?;
            }

            sqlx::query!(
                "
                insert or ignore into GeneralJson (
                    hash,
                    data
                )
                values (
                    $1,
                    $2
                )
                ",
                fmt_origin_hash,
                req.origin
            )
            .execute(&mut *self.connection())
            .await?;

            let description_hash = if req.description.is_some() {
                Some(fmt_description_hash)
            } else {
                None
            };
            sqlx::query!(
                "
                insert or ignore into RequirementDetails (
                    hash,
                    title_hash,
                    origin_hash,
                    description_hash
                )
                values (
                    $1,
                    $2,
                    $3,
                    $4
                )
                ",
                fmt_details_hash,
                fmt_title_hash,
                fmt_origin_hash,
                description_hash
            )
            .execute(&mut *self.connection())
            .await?;

            sqlx::query!(
                "
                insert or ignore into RequirementPropertiesHashes (
                    hash
                )
                values (
                    $1
                )
                ",
                fmt_properties_hash
            )
            .execute(&mut *self.connection())
            .await?;

            for prop in &req.properties {
                let fmt_prop_val_hash = prop.value.hash().finalize();

                sqlx::query!(
                    "
                    insert or ignore into GeneralJson (
                        hash,
                        data
                    )
                    values (
                        $1,
                        $2
                    )
                    ",
                    fmt_prop_val_hash,
                    prop.value
                )
                .execute(&mut *self.connection())
                .await?;

                sqlx::query!(
                    "
                    insert or ignore into RequirementProperties (
                        req_properties_hash,
                        property_key,
                        value_hash
                    )
                    values (
                        $1,
                        $2,
                        $3
                    )
                    ",
                    fmt_properties_hash,
                    prop.key,
                    fmt_prop_val_hash
                )
                .execute(&mut *self.connection())
                .await?;
            }

            if let Some(parents) = &req.parents {
                sqlx::query!(
                    "
                    insert or ignore into RequirementHierarchiesHashes (
                        hash
                    )
                    values (
                        $1
                    )
                    ",
                    fmt_hierarchies_hash
                )
                .execute(&mut *self.connection())
                .await?;

                for parent in parents {
                    let parent_product_id = parent.product_id.as_ref().unwrap_or(product_id);

                    sqlx::query!(
                        "
                        insert or ignore into RequirementHierarchies (
                            req_hierarchies_hash,
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
                        ",
                        fmt_hierarchies_hash,
                        product_id,
                        req.id,
                        parent_product_id,
                        parent.id,
                    )
                    .execute(&mut *self.connection())
                    .await?;
                }
            }
        }

        Ok(section_hash)
    }
}

fn req_content_hash(details_hash: Hash, properties_hash: Hash, hierarchies_hash: Hash) -> Hash {
    let mut hash = details_hash;
    hash.extend(properties_hash);
    hash.extend(hierarchies_hash);
    hash
}

fn req_details_hash(req: &Requirement) -> Hash {
    let mut hash = req.title.hash();
    hash.extend(req.origin.hash());
    hash.extend(req.description.hash());
    hash
}

fn req_properties_hash(req: &Requirement) -> Hash {
    req.properties.hash()
}

fn req_hierarchies_hash(req: &Requirement) -> Hash {
    req.parents.hash()
}
