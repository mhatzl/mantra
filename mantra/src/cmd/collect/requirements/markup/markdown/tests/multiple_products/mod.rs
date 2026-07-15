use std::collections::HashMap;

use mantra_schema::{
    product::ProductId,
    requirements::{ReqId, RequirementPk},
};

use crate::{cmd::collect::test_setup::db_from_dir, db::MantraPool};

#[sqlx::test]
async fn multiple_products(pool: MantraPool) {
    let db = db_from_dir!(pool, "./").unwrap();

    let reqs: Vec<RequirementPk> = sqlx::query!(
        "
            select id, product_id
            from Requirements
            "
    )
    .fetch_all(
        db.connection()
            .await
            .expect("Failed to get a connection")
            .as_mut(),
    )
    .await
    .unwrap()
    .into_iter()
    .map(|r| RequirementPk {
        id: r.id.try_into().unwrap(),
        product_id: Some(r.product_id.try_into().unwrap()),
    })
    .collect();

    let mut hierarchy: HashMap<RequirementPk, Vec<RequirementPk>> = HashMap::new();

    for req in reqs {
        let req_pid = req.product_id.clone().unwrap();

        let child_records = sqlx::query!(
            "
        select child_req_id, child_product_id
        from RequirementHierarchies
        where parent_req_id = $1 and parent_product_id = $2
        ",
            req.id,
            req_pid
        )
        .fetch_all(
            db.connection()
                .await
                .expect("Failed to get a connection")
                .as_mut(),
        )
        .await
        .unwrap();

        for child in child_records {
            let entry = hierarchy.entry(req.clone()).or_default();

            entry.push(RequirementPk {
                id: child.child_req_id.try_into().unwrap(),
                product_id: Some(child.child_product_id.try_into().unwrap()),
            });
        }
    }

    let product_a_req_1_hierarchy = hierarchy
        .get(&RequirementPk {
            id: ReqId::new("req-1".to_string()).unwrap(),
            product_id: Some(ProductId::new("A".to_string()).unwrap()),
        })
        .expect("Product A must contain req-1");

    assert_eq!(
        product_a_req_1_hierarchy.len(),
        1,
        "Product A req-1 must have exactly one child"
    );
    assert!(
        product_a_req_1_hierarchy.contains(&RequirementPk {
            id: ReqId::new("req-1".to_string()).unwrap(),
            product_id: Some(ProductId::new("B".to_string()).unwrap())
        }),
        "Product B req-1 must be a child of Product A req-1"
    );

    let product_a_req_2_hierarchy = hierarchy
        .get(&RequirementPk {
            id: ReqId::new("req-2".to_string()).unwrap(),
            product_id: Some(ProductId::new("A".to_string()).unwrap()),
        })
        .expect("Product A must contain req-2");

    assert_eq!(
        product_a_req_2_hierarchy.len(),
        1,
        "Product A req-2 must have exactly one child"
    );
    assert!(
        product_a_req_2_hierarchy.contains(&RequirementPk {
            id: ReqId::new("req-2".to_string()).unwrap(),
            product_id: Some(ProductId::new("B".to_string()).unwrap())
        }),
        "Product B req-2 must be a child of Product A req-2"
    );

    let product_b_req_1_hierarchy = hierarchy
        .get(&RequirementPk {
            id: ReqId::new("req-1".to_string()).unwrap(),
            product_id: Some(ProductId::new("B".to_string()).unwrap()),
        })
        .expect("Product B must contain req-1");

    assert_eq!(
        product_b_req_1_hierarchy.len(),
        1,
        "Product B req-1 must have exactly one child"
    );
    assert!(
        product_b_req_1_hierarchy.contains(&RequirementPk {
            id: ReqId::new("req-2".to_string()).unwrap(),
            product_id: Some(ProductId::new("B".to_string()).unwrap())
        }),
        "Product B req-2 must be a child of Product B req-1"
    );

    assert_eq!(
        hierarchy.get(&RequirementPk {
            id: ReqId::new("req-2".to_string()).unwrap(),
            product_id: Some(ProductId::new("B".to_string()).unwrap()),
        }),
        None,
        "Product B req-2 must have no children"
    );
}
