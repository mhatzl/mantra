use std::path::Path;

use crate::{
    cfg::MantraConfigFile,
    cmd::collect::cfg::{CollectArguments, CollectConfig, CollectEnvironmentVariables},
    db::MantraDb,
};

pub(super) async fn collect_test_data(
    db: &MantraDb,
    mantra_cfg: &Path,
) -> Result<(), anyhow::Error> {
    super::collect(db, collect_test_cfg(mantra_cfg).await?).await
}

pub(super) async fn collect_test_cfg(mantra_cfg: &Path) -> Result<CollectConfig, anyhow::Error> {
    let cfg_file: MantraConfigFile = crate::io::async_deserialize_from_path(mantra_cfg).await?;
    cfg_file.inheritable_product_cfg.check_validity()?;

    let mut product_cfgs = Vec::new();

    for product_cfg in cfg_file.products {
        product_cfgs.push(product_cfg.resolve(&cfg_file.inheritable_product_cfg)?);
    }

    Ok(CollectConfig {
        cfg_filepath: mantra_cfg.to_path_buf(),
        args: CollectArguments::default(),
        envs: CollectEnvironmentVariables {},
        product_cfgs,
    })
}

macro_rules! db_from_cfg_file {
    ($db_pool:ident, $cfg_file:expr) => {{
        let cfg_filepath = $crate::cmd::collect::test_setup::testdata_dir!($cfg_file);
        let db = $crate::db::MantraDb::new_with_pool($db_pool);

        $crate::cmd::collect::test_setup::collect_test_data(&db, &cfg_filepath)
            .await
            .and(Ok(db))
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
    ($db_pool:ident, $dir:expr) => {{ db_from_dir!($db_pool, $dir, "mantra.json5") }};
    ($db_pool:ident, $dir:expr, $cfg_file:expr) => {{
        let test_dir = $crate::cmd::collect::test_setup::testdata_dir!($dir);
        let test_cfg_filepath = test_dir.join($cfg_file);
        let db = $crate::db::MantraDb::new_with_pool($db_pool);

        crate::cmd::collect::test_setup::collect_test_data(&db, &test_cfg_filepath)
            .await
            .and(Ok(db))
    }};
}

pub(super) use db_from_cfg_file;
pub(super) use db_from_dir;
pub(super) use testdata_dir;
