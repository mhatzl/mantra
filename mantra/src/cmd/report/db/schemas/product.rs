use std::str::FromStr;

use mantra_schema::{
    product::ProductId,
    report::{
        product::{ProductReportSchema, ProductSummary},
        requirement::{RequirementReference, RequirementState},
        review::ReviewReference,
        test_run::TestRunReference,
        tests::TestState,
    },
    requirements::ReqId,
};

use crate::db::MantraTransaction;

pub async fn generate_product_schemas<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_ids: Option<&[ProductId]>,
) -> Result<Vec<ProductReportSchema>, anyhow::Error> {
    struct ProductRecord {
        last_collected_date: String,
        id: String,
        name: String,
        base: Option<String>,
        version: Option<String>,
        homepage: Option<String>,
        repository: Option<String>,
        license: Option<String>,
        description: Option<String>,
    }

    let mut product_records = Vec::new();

    match product_ids {
        Some(expected_ids) => {
            for id in expected_ids {
                let product_record = sqlx::query_as!(
                    ProductRecord,
                    r#"
                        select
                            c.collected_at_utc as "last_collected_date",
                            p.id,
                            p.name,
                            p.base,
                            p.version,
                            p.homepage,
                            p.repository,
                            p.license,
                            gt.content as "description?"
                        from Products p left join GeneralTexts gt on p.description_hash = gt.hash,
                            Collections c
                        where p.id = $1 and p.latest_collection = c.id
                    "#,
                    id
                )
                .fetch_optional(transaction.as_mut())
                .await?;

                match product_record {
                    Some(record) => {
                        product_records.push(record);
                    }
                    None => anyhow::bail!("No data collected for product '{}'", id),
                }
            }
        }
        None => {
            for record in sqlx::query_as!(
                ProductRecord,
                r#"
                    select
                        c.collected_at_utc as "last_collected_date",
                        p.id,
                        p.name,
                        p.base,
                        p.version,
                        p.homepage,
                        p.repository,
                        p.license,
                        gt.content as "description?"
                    from Products p left join GeneralTexts gt on p.description_hash = gt.hash,
                        Collections c
                    where p.latest_collection = c.id
                "#
            )
            .fetch_all(transaction.as_mut())
            .await?
            {
                product_records.push(record);
            }
        }
    }

    let mut products = Vec::with_capacity(product_records.len());

    for product_record in product_records {
        let product_id: ProductId = product_record.id.clone().try_into()?;
        let last_collected_date =
            mantra_schema::reviews::date_from_str(&product_record.last_collected_date)?;

        let root_requirements = product_root_requirements(transaction, &product_id).await?;
        let root_test_runs = product_root_test_runs(transaction, &product_id).await?;
        let reviews = product_reviews(transaction, &product_id).await?;

        let mut product_properties = serde_json::Map::new();

        for product_property in sqlx::query!(
            r#"
                select p.property_key, gj.content
                from ProductProperties p, GeneralJson gj
                where p.product_id = $1 and gj.hash = p.value_hash
            "#,
            product_record.id
        )
        .fetch_all(transaction.as_mut())
        .await?
        {
            product_properties.insert(
                product_property.property_key,
                serde_json::Value::from_str(&product_property.content)?,
            );
        }

        let product_properties = if product_properties.is_empty() {
            None
        } else {
            Some(product_properties)
        };

        products.push(ProductReportSchema {
            schema_version: Some(mantra_schema::SCHEMA_VERSION.to_owned()),
            last_collected_date,
            // Note: summary is populated in `create_product_structure`
            summary: ProductSummary::default(),
            root_requirements,
            root_test_runs,
            reviews,
            id: product_id,
            name: product_record.name,
            base: product_record.base,
            version: product_record.version,
            homepage: product_record.homepage,
            repository: product_record.repository,
            license: product_record.license,
            description: product_record.description,
            properties: product_properties,
        });
    }

    Ok(products)
}

pub async fn product_root_requirements<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
) -> Result<Vec<RequirementReference>, anyhow::Error> {
    let root_requirements = sqlx::query!(
        r#"
        select
            rs.product_id,
            rs.req_id,
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
            ) as "manual!:bool"
        from RequirementVerificationStates rs, RootRequirements rr
        where rs.product_id = $1
        and rr.product_id = rs.product_id
        and rr.req_id = rs.req_id
        "#,
        product_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| RequirementReference {
        product_id: product_id.clone(),
        id: ReqId::new(r.req_id).expect("Valid requirement ID in database"),
        state: RequirementState::try_from(r.state).expect("Valid requirement state in database"),
        optional: r.optional,
        manual: r.manual,
    })
    .collect();

    Ok(root_requirements)
}

pub async fn product_root_test_runs<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
) -> Result<Vec<TestRunReference>, anyhow::Error> {
    let root_test_runs = sqlx::query!(
        r#"
        select ts.product_id, ts.test_run_name, ts.state as "state!:i64"
        from TestRunStates ts
        where ts.product_id = $1 and not exists (
            select th.parent_name
            from TestRunHierarchies th
            where th.product_id = $1
            and th.child_name = ts.test_run_name
        )
        "#,
        product_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|tr| TestRunReference {
        product_id: product_id.clone(),
        name: tr.test_run_name,
        state: TestState::try_from(tr.state).expect("Valid test run state in database"),
    })
    .collect();

    Ok(root_test_runs)
}

pub async fn product_reviews<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_id: &ProductId,
) -> Result<Vec<ReviewReference>, anyhow::Error> {
    let reviews = sqlx::query!(
        r#"
        select product_id, review_name, state
        from ReviewStates
        where product_id = $1
        "#,
        product_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| ReviewReference {
        product_id: product_id.clone(),
        name: r.review_name,
        state: mantra_schema::report::review::ReviewState::try_from(r.state)
            .expect("Valid review state in database"),
    })
    .collect();

    Ok(reviews)
}
