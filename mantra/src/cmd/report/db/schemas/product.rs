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
                        where id = $1 and p.last_collect_nr = c.nr
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
                    where p.last_collect_nr = c.nr
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
            rs.id,
            rs.state,
            case when exists (
                select id
                from OptionalRequirements o
                where o.product_id = $1 and o.id = rs.id
            ) then true
            else false
            end as "optional!:bool"
        from RequirementVerificationStates rs
        where rs.product_id = $1 and not exists (
            select rh.parent_req_id
            from RequirementHierarchies rh
            where rh.child_product_id = rs.product_id
            and rh.child_req_id = rs.id
            and rh.parent_product_id = $1 -- child to other product is fine
        )
        "#,
        product_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| RequirementReference {
        product_id: product_id.clone(),
        id: ReqId::new(r.id).expect("Valid requirement ID in database"),
        state: RequirementState::try_from(r.state).expect("Valid requirement state in database"),
        optional: r.optional,
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
        select ts.product_id, ts.test_run_name, ts.test_run_date, ts.state as "state!:i64"
        from TestRunStates ts
        where ts.product_id = $1 and not exists (
            select th.parent_name, th.parent_date
            from TestRunHierarchies th
            where th.product_id = $1
            and th.child_name = ts.test_run_name
            and th.child_date = ts.test_run_date
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
        utc_date: mantra_schema::test_runs::test_date_from_str(&tr.test_run_date)
            .expect("Valid test run date in database"),
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
        "
        select product_id, name, utc_date
        from Reviews
        where product_id = $1
        ",
        product_id
    )
    .fetch_all(transaction.as_mut())
    .await?
    .into_iter()
    .map(|r| ReviewReference {
        product_id: product_id.clone(),
        name: r.name,
        utc_date: mantra_schema::reviews::date_from_str(&r.utc_date)
            .expect("Valid review date in database"),
        state: mantra_schema::report::review::ReviewState::Valid, // TODO: replace with accurate state
    })
    .collect();

    Ok(reviews)
}
