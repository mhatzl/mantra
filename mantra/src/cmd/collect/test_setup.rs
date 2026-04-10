use std::path::Path;

use crate::{
    cfg::MantraConfigFile,
    cmd::collect::cfg::{CollectArguments, CollectConfig, CollectEnvironmentVariables},
    db::test_stub::TestDb,
};

pub(super) async fn collect_test_data(mantra_cfg: &Path) -> Result<TestDb, anyhow::Error> {
    let db = crate::db::test_stub::TestDb::new().await?;

    for cfg in test_collect_cfgs(mantra_cfg).await? {
        super::collect(db.db(), cfg).await?;
    }

    Ok(db)
}

pub(super) async fn test_collect_cfgs(
    mantra_cfg: &Path,
) -> Result<Vec<CollectConfig>, anyhow::Error> {
    let cfg_file: MantraConfigFile = crate::io::async_deserialize_from_path(mantra_cfg).await?;
    cfg_file.inheritable_product_cfg.check_validity()?;

    let mut collect_cfgs = Vec::new();

    for product_cfg in cfg_file.products {
        collect_cfgs.push(CollectConfig {
            cfg_filepath: mantra_cfg.to_path_buf(),
            args: CollectArguments::default(),
            envs: CollectEnvironmentVariables {},
            product: product_cfg
                .product
                .to_product(&cfg_file.inheritable_product_cfg)?,
            requirements: product_cfg.requirements,
            annotations: product_cfg.annotations,
            test_runs: product_cfg.test_runs,
            reviews: product_cfg.reviews,
            lsif: product_cfg.lsif,
        })
    }

    Ok(collect_cfgs)
}

macro_rules! db_from_cfg_file {
    ($cfg_file:expr) => {{
        let cfg_filepath = $crate::cmd::collect::test_setup::testdata_dir!($cfg_file);
        $crate::cmd::collect::test_setup::collect_test_data(&cfg_filepath).await
    }};
}

macro_rules! testdata_dir {
    ($dir:expr) => {{
        let rel_src_filepath = std::path::PathBuf::from(file!());
        let rel_crate_path = rel_src_filepath
            .strip_prefix("mantra")
            .expect("file!() always starts with crate name.");

        let crate_dir = std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR must be set. Are you testing outside Cargo?"),
        );

        let abs_src_filepath = crate_dir.join(rel_crate_path);
        let abs_dir_path = abs_src_filepath
            .parent()
            .expect("Parent part of absolute src filepath must exist.");

        abs_dir_path.join($dir)
    }};
}

macro_rules! db_from_dir {
    ($dir:expr) => {{ db_from_dir!($dir, "mantra.json5") }};
    ($dir:expr, $cfg_file:expr) => {{
        let tmp_dir = $crate::cmd::collect::test_setup::testdata_dir!($dir);

        let cfg_tmp_filepath = tmp_dir.join($cfg_file);

        crate::cmd::collect::test_setup::collect_test_data(&cfg_tmp_filepath).await
    }};
}

pub(super) use db_from_cfg_file;
pub(super) use db_from_dir;
pub(super) use testdata_dir;
