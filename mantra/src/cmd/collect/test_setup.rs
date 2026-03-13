use std::path::Path;

use crate::{
    cfg::MantraConfigFile,
    cmd::collect::cfg::{CollectArguments, CollectConfig, CollectEnvironmentVariables},
    db::test_stub::TestConnection,
};

pub(super) async fn collect_test_data(mantra_cfg: &Path) -> Result<TestConnection, anyhow::Error> {
    let db = crate::db::test_stub::test_db().await;

    for cfg in test_collect_cfgs(mantra_cfg).await? {
        super::collect(&db, cfg).await?;
    }

    db.test_connection().await
}

async fn test_collect_cfgs(mantra_cfg: &Path) -> Result<Vec<CollectConfig>, anyhow::Error> {
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
    ($cfg_file:literal) => {{
        let cfg_file_content = include_str!($cfg_file);
        let cfg_filepath = std::path::PathBuf::from($cfg_file);
        let cfg_file_extension = cfg_filepath
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("json5");
        let tmp_file =
            tempfile::NamedTempFile::with_suffix(format!(".{}", cfg_file_extension)).unwrap();
        let cfg_tmp_filepath = tmp_file.path();
        std::fs::write(cfg_tmp_filepath, cfg_file_content).unwrap();

        $crate::cmd::collect::test_setup::collect_test_data(&cfg_tmp_filepath)
            .await
            .unwrap()
    }};
}

pub(super) use db_from_cfg_file;
