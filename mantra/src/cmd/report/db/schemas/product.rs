use std::str::FromStr;

use mantra_schema::{
    product::ProductId,
    report::product::{ProductReportSchema, ProductSummary},
};

use crate::db::MantraTransaction;

pub async fn generate_product_schemas<'db>(
    transaction: &mut MantraTransaction<'db>,
    product_ids: Option<&[ProductId]>,
) -> Result<Vec<ProductReportSchema>, anyhow::Error> {
    struct ProductRecord {
        last_collected_date: String,
        id: ProductId,
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
        let last_collected_date =
            mantra_schema::reviews::date_from_str(&product_record.last_collected_date)?;

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
            summary: ProductSummary::default(),
            root_requirements: Vec::new(),
            root_test_runs: Vec::new(),
            reviews: Vec::new(),
            id: product_record.id,
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
