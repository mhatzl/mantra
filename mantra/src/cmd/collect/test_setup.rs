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

        $crate::cmd::collect::test_setup::collect_test_data(&cfg_tmp_filepath).await
    }};
}

macro_rules! db_from_dir {
    ($dir:expr) => {{ db_from_dir!($dir, "mantra.json5") }};
    ($dir:expr, $cfg_file:expr) => {{
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

        let test_path = abs_dir_path.join($dir);

        let tmp_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        for entry in walkdir::WalkDir::new(&test_path) {
            let entry = entry.unwrap_or_else(|e| panic!("Failed to access entry due to '{e}'"));
            let entry_path = entry.path();

            if entry_path.is_file() {
                std::fs::copy(entry_path, tmp_dir.path().join(entry.file_name())).unwrap_or_else(
                    |e| {
                        panic!(
                            "Failed to copy file '{}' due to '{}'",
                            entry_path.display(),
                            e
                        )
                    },
                );
            } else {
                let rel_tmp_path =
                    mantra_schema::path::PathExt::relative_to(entry_path, &test_path)
                        .expect("Relative path creation failed");
                // we don't care if dir creation fails, because we only really care for files,
                // and missing parent dir would fail file copy above.
                let _ = std::fs::create_dir(tmp_dir.path().join(rel_tmp_path.as_str()));
            }
        }

        let cfg_tmp_filepath = tmp_dir.path().join($cfg_file);

        crate::cmd::collect::test_setup::collect_test_data(&cfg_tmp_filepath).await
    }};
}

pub(super) use db_from_cfg_file;
pub(super) use db_from_dir;
