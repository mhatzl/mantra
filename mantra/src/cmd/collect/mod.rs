use mantra_schema::{
    FmtHash, Properties, path::RelativePath, product::ProductId, time::OffsetDateTime,
};
use sqlx::migrate::Migrate;
use std::path::PathBuf;

use crate::{
    cmd::collect::{cfg::CollectConfig, collector::SingleFileCollector},
    db::{MantraConnection, MantraDb, MantraTransaction},
};

// pub mod db;
pub mod annotations;
pub mod cfg;
pub mod collector;
pub mod products;
pub mod requirements;
pub mod reviews;
pub mod test_runs;
pub mod walker;

pub async fn collect<'db>(db: &'db MantraDb, cfg: CollectConfig) -> Result<(), anyhow::Error> {
    let mut collection = collect_data(db, cfg).await?;

    collection.aggregate_requirements_data().await?;
    collection.aggregate_annotations_data().await?;

    collection.commit().await?;

    // requirements
    // - dot-notation hierarchy setup after all reqs are collected
    //   => needed to *jump* over non-existent parent IDs (put out warning)
    // annotations
    // test runs
    // reviews

    // delete olds:
    // remove from all *data*-tables with collect-nr & product-id
    // the entries that are not at current collect-nr
    // except tables related to test runs and reviews
    // => means entries for requirement and product information, which did not get collected this time
    //
    // data-tables are all non-history tables containing data that cannot be computed from other tables
    // test run and review related tables are kept, because not every collect run might add test/review info
    // due to timestamp, test run and review entries are better set to *oboslete*
    //
    // deletion separate from this fn to allow use of the collect fn for LSPs.
    // LSPs will only add/update infos per file, so older collected data unrelated to the file is likely still accurate
    // old info related to the file must however be deleted. e.g. requirement properties and hierarchy

    // checks
    // - req cycles
    // - test run cycles

    // update aggregated-tables (everything related to prod-id)
    // - with history + collect-nr
    //   - allows to see changes in aggregated tables
    // - deprecated requirements
    //   - either marked directly
    //   - any children of it
    // - ignored requirements
    //   - either marked directly
    //   - any children of it
    //   - do not list such reqs in table of req-traces for product-id
    // - manual requirements
    //   - either marked directly
    //   - any children of it
    // - traces directly covered by test (run)
    //   - apply overrides for coverage from reviews
    //     - new table for coverage with overrides applied
    //     - do not consider review if obsolete: review date older than test run => warn if so
    //   - traced line in statement coverage of test (run) with hit > 0
    //   - trace linked to element and at least one line of element span in statement coverage with hit > 0
    //   - calc percentage of statement lines that link to trace and have hit > 0 to get accurate requirements coverage
    //     not just covered/uncovered, but covered to xx% statement coverage (adding other coverage in future)
    // - test states
    //   - apply overrides from reviews if not obsolete (same as for coverage)
    //   - test runs fail if one or more test cases or child test runs fail
    // - satisfied requirements
    //   - either satisfies-trace or review for manual req
    //   - must not be deprecated => warn if so
    //   - ignored reqs must not be considered => info if so
    //     - since ignored for traces, could only be through reviews
    // - verified requirements
    //   - either via passed test or review
    //   - fail verification if one or more tests fail that would verify req
    //     state from review overrides wins of actual test state
    //   - detect obsolete tests and reviews per req-id
    //     - obsolete if:
    //       - requirement histories (several tables) contain at least one entry
    //         that is younger than the test or review
    //       - test (run) covers lines in files that had changes since test (run) execution
    //       - make a "WouldVerifyReq" intermediate table for test (runs) to see if it would verify
    //         and then check if test (run) is obsolete due to changed files
    //   - test must at least have one verifies-trace that is covered by it
    //     - if at least one satisfies-trace exists, it must also be passed by the test
    //     - test must not be obsolete
    //   - review only counts for manual reqs and must not be obsolete
    //   - must not be deprecated => warn if so
    //   - ignored reqs must not be considered => info if so
    //     - since ignored for traces, could only be through reviews or test runs
    //   - indirectly satisfied/verified
    //     - all children are satisfied/verified
    //

    Ok(())
}

async fn collect_data<'db>(
    db: &'db MantraDb,
    cfg: CollectConfig,
) -> Result<Collection<'db>, anyhow::Error> {
    let mut collection = Collection::new(db, &cfg).await?;
    collection.update_product(cfg.product).await?;

    let req_collector = SingleFileCollector::new(collection);
    let mut collection = req_collector.collect(cfg.requirements).await?;
    // Note: dot-hierarchy updated explicitely after collecting all requirements,
    // because not all *dot-parts* may have been added as requirements.
    // e.g. top.missing.lead-id => skipping "missing" if not available as requirement
    collection.update_req_dot_hierarchy().await?;
    collection.delete_outdated_reqs().await?;

    let annotation_collector = SingleFileCollector::new(collection);
    let mut collection = annotation_collector.collect(cfg.annotations).await?;
    collection.delete_outdated_annotations().await?;

    test_runs::collect(&mut collection, cfg.test_runs).await?;
    collection.delete_outdated_test_runs().await?;

    let review_collector = SingleFileCollector::new(collection);
    let mut collection = review_collector.collect(cfg.reviews).await?;
    collection.delete_outdated_reviews().await?;

    // product cleanup after all other data was collected,
    // because product data may get updated from any source.
    collection.delete_outdated_product_info().await?;

    Ok(collection)
}

struct Collection<'db> {
    transaction: MantraTransaction<'db>,
    cfg_filepath: PathBuf,
    nr: i64,
    product_id: ProductId,
    collected_at_utc: OffsetDateTime,
    replace_hashed: bool,
}

impl<'db> Collection<'db> {
    async fn new(db: &'db MantraDb, cfg: &CollectConfig) -> Result<Self, anyhow::Error> {
        let collected_at_utc = OffsetDateTime::now_utc();
        let mut transaction = db.start_transaction().await?;
        let config_value = serde_json::json!({
            "product": cfg.product,
            "req-cfg": cfg.requirements,
            "annotation-cfg": cfg.annotations,
            "test-run-cfg": cfg.test_runs,
            "review-cfg": cfg.reviews
        });
        let config_hash = FmtHash::from(&config_value);

        insert_general_json(
            &mut transaction,
            &config_hash,
            config_value,
            cfg.args.replace_hashed,
        )
        .await?;

        // TODO: add args and env data to table

        sqlx::query!(
            "
            insert into Collections (
                collected_at_utc,
                config_hash,
                arguments_hash,
                env_vars_hash
            )
            values (
                $1,
                $2,
                null,
                null
            )
            ",
            collected_at_utc,
            config_hash
        )
        .execute(&mut *transaction.as_mut())
        .await?;

        let nr = sqlx::query!(r#"select max(nr) as "nr!" from Collections"#)
            .fetch_one(&mut *transaction.as_mut())
            .await?
            .nr;

        Ok(Self {
            transaction,
            cfg_filepath: cfg.cfg_filepath.clone(),
            nr,
            product_id: cfg.product.id(),
            collected_at_utc,
            replace_hashed: cfg.args.replace_hashed,
        })
    }

    fn connection_mut(&mut self) -> &mut MantraConnection {
        self.transaction.as_mut()
    }

    fn collect_nr(&self) -> i64 {
        self.nr
    }

    fn product_id(&self) -> ProductId {
        self.product_id.clone()
    }

    async fn commit(self) -> Result<(), anyhow::Error> {
        Ok(self.transaction.commit().await?)
    }

    async fn insert_general_json(
        &mut self,
        hash: &FmtHash,
        content: serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        insert_general_json(&mut self.transaction, hash, content, self.replace_hashed).await
    }

    async fn insert_general_text(
        &mut self,
        hash: &FmtHash,
        content: String,
    ) -> Result<(), anyhow::Error> {
        if self.replace_hashed {
            sqlx::query!(
                "
                insert or replace into GeneralTexts (
                    hash,
                    content
                )
                values (
                    $1,
                    $2
                )
                ",
                hash,
                content
            )
            .execute(self.connection_mut())
            .await?;
        } else {
            sqlx::query!(
                "
                insert or ignore into GeneralTexts (
                    hash,
                    content
                )
                values (
                    $1,
                    $2
                )
                ",
                hash,
                content
            )
            .execute(self.connection_mut())
            .await?;
        }

        Ok(())
    }

    async fn insert_file_hash(
        &mut self,
        filepath: &RelativePath,
        file_hash: &FmtHash,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            insert or ignore into FileHashes (
                hash
            )
            values (
                $1
            )
            ",
            file_hash
        )
        .execute(self.connection_mut())
        .await?;

        let filepath = filepath.as_str();
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        sqlx::query!(
            "
            insert into ProductRelatedFiles (
                last_collect_nr,
                product_id,
                filepath,
                file_hash
            )
            values (
                $1,
                $2,
                $3,
                $4
            )
            on conflict (product_id, filepath)
            do update set
                last_collect_nr = excluded.last_collect_nr,
                file_hash = excluded.file_hash
            ",
            collect_nr,
            product_id,
            filepath,
            file_hash
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    /// Returns the absolute path to the directory the used mantra config file is located in.
    fn abs_cfg_file_parent_path(&self) -> std::io::Result<PathBuf> {
        std::path::absolute(
            self.cfg_filepath
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or(PathBuf::from("./")),
        )
    }
}

async fn insert_general_json<'db>(
    transaction: &mut MantraTransaction<'db>,
    hash: &FmtHash,
    content: serde_json::Value,
    replace_hashed: bool,
) -> Result<(), anyhow::Error> {
    if replace_hashed {
        sqlx::query!(
            "
            insert or replace into GeneralJson (
                hash,
                content
            )
            values (
                $1,
                $2
            )
            ",
            hash,
            content
        )
        .execute(&mut *transaction.as_mut())
        .await?;
    } else {
        sqlx::query!(
            "
            insert or ignore into GeneralJson (
                hash,
                content
            )
            values (
                $1,
                $2
            )
            ",
            hash,
            content
        )
        .execute(&mut *transaction.as_mut())
        .await?;
    }

    Ok(())
}

fn merge_local_and_base_properties(
    local_props: Option<Properties>,
    base_props: &Option<Properties>,
) -> Option<Properties> {
    if local_props.is_none() && base_props.is_none() {
        return None;
    }

    let mut props = local_props.unwrap_or_default();

    if let Some(base_props) = base_props {
        for base_prop in base_props {
            if !props.contains_key(base_prop.0) {
                props.insert(base_prop.0.clone(), base_prop.1.clone());
            }
        }
    }

    Some(props)
}
