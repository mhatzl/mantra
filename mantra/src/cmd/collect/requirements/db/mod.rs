use mantra_schema::{
    FmtHash, Properties,
    requirements::{Requirement, RequirementSchema},
};

use crate::cmd::collect::Collection;
use crate::cmd::collect::merge_local_and_base_properties;

impl<'db> Collection<'db> {
    pub(crate) async fn update_requirements(
        &mut self,
        req_schemas: Vec<RequirementSchema>,
    ) -> Result<(), anyhow::Error> {
        for reqs in req_schemas {
            self.update_per_req_schema(reqs).await?;
        }

        Ok(())
    }

    pub(crate) async fn update_per_req_schema(
        &mut self,
        req_schema: RequirementSchema,
    ) -> Result<(), anyhow::Error> {
        let base_origin_hash = req_schema.origin.as_ref().map(FmtHash::from);

        if let Some(hash) = &base_origin_hash
            && let Some(origin) = req_schema.origin
        {
            self.insert_general_json(&hash, origin.clone()).await?;
        }

        // TODO: do not stop at first collect error

        for req in req_schema.requirements {
            self.update_requirement(req, &base_origin_hash, &req_schema.properties)
                .await?;
        }

        Ok(())
    }

    async fn update_requirement(
        &mut self,
        req: Requirement,
        base_origin_hash: &Option<FmtHash>,
        base_props: &Option<Properties>,
    ) -> Result<(), anyhow::Error> {
        // TODO: optimize by checking src-hash first and skip if unchanged

        let collect_nr = self.collect_nr();
        let product_id = &self.product_id();
        let src_hash = FmtHash::from(&serde_json::json!({
            "base_origin_hash": base_origin_hash,
            "base_props": base_props,
            "req": &req
        }));

        let origin_hash = FmtHash::from(&req.origin);
        self.insert_general_json(&origin_hash, req.origin).await?;
        let description_hash = req.description.as_ref().map(FmtHash::from);
        if let Some(hash) = &description_hash
            && let Some(description) = req.description
        {
            self.insert_general_text(hash, description).await?;
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
                title,
                base_origin_hash,
                origin_hash,
                description_hash,
                src_hash
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
                $11
            )
            on conflict (id, product_id)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                manual_verification = excluded.manual_verification,
                deprecated = excluded.deprecated,
                ignore = excluded.ignore,
                title = excluded.title,
                base_origin_hash = excluded.base_origin_hash,
                origin_hash = excluded.origin_hash,
                description_hash = excluded.description_hash,
                src_hash = excluded.src_hash
            ",
            collect_nr,
            req.id,
            product_id,
            req.manual_verification,
            req.deprecated,
            req.ignore,
            req.title,
            base_origin_hash,
            origin_hash,
            description_hash,
            src_hash
        )
        .execute(&mut *self.connection())
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
                .execute(&mut *self.connection())
                .await?;
            }
        }

        if let Some(parents) = req.parents {
            for parent in parents {
                let parent_product_id = parent.product_id.unwrap_or(self.product_id());

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
                .execute(&mut *self.connection())
                .await?;
            }
        }

        Ok(())
    }
}

// impl<'db> CollectTransaction<'db> {
//     pub async fn update_requirements(
//         &'db mut self,
//         product_id: &ProductId,
//         requirements: &[Requirement],
//     ) -> Result<FmtHash, anyhow::Error> {
//         let section_hash = requirements.fmt_hash();
//         let section_ref = &section_hash;

//         sqlx::query!(
//             "
//             insert or ignore into CollectedSections (hash)
//             values ($1)
//             ",
//             section_ref
//         )
//         .execute(&mut *self.connection())
//         .await?;

//         for req in requirements {
//             let fmt_title_hash = &req.title.hash().finalize();
//             let fmt_origin_hash = &req.origin.hash().finalize();
//             let fmt_description_hash = &req.description.hash().finalize();
//             let details_hash = req_details_hash(req);
//             let fmt_details_hash = &details_hash.clone().finalize();
//             let properties_hash = req_properties_hash(req);
//             let fmt_properties_hash = &properties_hash.clone().finalize();
//             let hierarchies_hash = req_hierarchies_hash(req);
//             let fmt_hierarchies_hash = &hierarchies_hash.clone().finalize();
//             let content_hash = req_content_hash(details_hash, properties_hash, hierarchies_hash);
//             let fmt_content_hash = &content_hash.finalize();

//             sqlx::query!(
//                 "
//                 insert or ignore into Requirements (id, product_id)
//                 values ($1, $2)
//                 ",
//                 req.id,
//                 product_id,
//             )
//             .execute(&mut *self.connection())
//             .await?;

//             sqlx::query!(
//                 "
//                 insert or ignore into RequirementCollections (
//                     section_hash,
//                     product_id,
//                     req_id,
//                     req_content_hash
//                 )
//                 values (
//                     $1,
//                     $2,
//                     $3,
//                     $4
//                 )
//                 ",
//                 section_ref,
//                 product_id,
//                 req.id,
//                 fmt_content_hash
//             )
//             .execute(&mut *self.connection())
//             .await?;

//             let hierarchies_hash = if req.parents.is_some() {
//                 Some(fmt_hierarchies_hash)
//             } else {
//                 None
//             };
//             sqlx::query!(
//                 "
//                 insert or ignore into RequirementContents (
//                     hash,
//                     req_details_hash,
//                     req_properties_hash,
//                     req_hierarchies_hash,
//                     manual_verification,
//                     deprecated
//                 )
//                 values (
//                     $1,
//                     $2,
//                     $3,
//                     $4,
//                     $5,
//                     $6
//                 )
//                 ",
//                 fmt_content_hash,
//                 fmt_details_hash,
//                 fmt_properties_hash,
//                 hierarchies_hash,
//                 req.manual_verification,
//                 req.deprecated
//             )
//             .execute(&mut *self.connection())
//             .await?;

//             sqlx::query!(
//                 "
//                 insert or ignore into GeneralContents (
//                     hash,
//                     content
//                 )
//                 values (
//                     $1,
//                     $2
//                 )
//                 ",
//                 fmt_title_hash,
//                 req.title
//             )
//             .execute(&mut *self.connection())
//             .await?;

//             if let Some(description) = &req.description {
//                 sqlx::query!(
//                     "
//                     insert or ignore into GeneralContents (
//                         hash,
//                         content
//                     )
//                     values (
//                         $1,
//                         $2
//                     )
//                     ",
//                     fmt_description_hash,
//                     description
//                 )
//                 .execute(&mut *self.connection())
//                 .await?;
//             }

//             sqlx::query!(
//                 "
//                 insert or ignore into GeneralJson (
//                     hash,
//                     data
//                 )
//                 values (
//                     $1,
//                     $2
//                 )
//                 ",
//                 fmt_origin_hash,
//                 req.origin
//             )
//             .execute(&mut *self.connection())
//             .await?;

//             let description_hash = if req.description.is_some() {
//                 Some(fmt_description_hash)
//             } else {
//                 None
//             };
//             sqlx::query!(
//                 "
//                 insert or ignore into RequirementDetails (
//                     hash,
//                     title_hash,
//                     origin_hash,
//                     description_hash
//                 )
//                 values (
//                     $1,
//                     $2,
//                     $3,
//                     $4
//                 )
//                 ",
//                 fmt_details_hash,
//                 fmt_title_hash,
//                 fmt_origin_hash,
//                 description_hash
//             )
//             .execute(&mut *self.connection())
//             .await?;

//             sqlx::query!(
//                 "
//                 insert or ignore into RequirementPropertiesHashes (
//                     hash
//                 )
//                 values (
//                     $1
//                 )
//                 ",
//                 fmt_properties_hash
//             )
//             .execute(&mut *self.connection())
//             .await?;

//             for prop in &req.properties {
//                 let fmt_prop_val_hash = prop.value.hash().finalize();

//                 sqlx::query!(
//                     "
//                     insert or ignore into GeneralJson (
//                         hash,
//                         data
//                     )
//                     values (
//                         $1,
//                         $2
//                     )
//                     ",
//                     fmt_prop_val_hash,
//                     prop.value
//                 )
//                 .execute(&mut *self.connection())
//                 .await?;

//                 sqlx::query!(
//                     "
//                     insert or ignore into RequirementProperties (
//                         req_properties_hash,
//                         property_key,
//                         value_hash
//                     )
//                     values (
//                         $1,
//                         $2,
//                         $3
//                     )
//                     ",
//                     fmt_properties_hash,
//                     prop.key,
//                     fmt_prop_val_hash
//                 )
//                 .execute(&mut *self.connection())
//                 .await?;
//             }

//             if let Some(parents) = &req.parents {
//                 sqlx::query!(
//                     "
//                     insert or ignore into RequirementHierarchiesHashes (
//                         hash
//                     )
//                     values (
//                         $1
//                     )
//                     ",
//                     fmt_hierarchies_hash
//                 )
//                 .execute(&mut *self.connection())
//                 .await?;

//                 for parent in parents {
//                     let parent_product_id = parent.product_id.as_ref().unwrap_or(product_id);

//                     sqlx::query!(
//                         "
//                         insert or ignore into RequirementHierarchies (
//                             req_hierarchies_hash,
//                             child_product_id,
//                             child_req_id,
//                             parent_product_id,
//                             parent_req_id
//                         )
//                         values (
//                             $1,
//                             $2,
//                             $3,
//                             $4,
//                             $5
//                         )
//                         ",
//                         fmt_hierarchies_hash,
//                         product_id,
//                         req.id,
//                         parent_product_id,
//                         parent.id,
//                     )
//                     .execute(&mut *self.connection())
//                     .await?;
//                 }
//             }
//         }

//         Ok(section_hash)
//     }
// }

// fn req_content_hash(details_hash: Hash, properties_hash: Hash, hierarchies_hash: Hash) -> Hash {
//     let mut hash = details_hash;
//     hash.extend(properties_hash);
//     hash.extend(hierarchies_hash);
//     hash
// }

// fn req_details_hash(req: &Requirement) -> Hash {
//     let mut hash = req.title.hash();
//     hash.extend(req.origin.hash());
//     hash.extend(req.description.hash());
//     hash
// }

// fn req_properties_hash(req: &Requirement) -> Hash {
//     req.properties.hash()
// }

// fn req_hierarchies_hash(req: &Requirement) -> Hash {
//     req.parents.hash()
// }
