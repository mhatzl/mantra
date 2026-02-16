use std::path::PathBuf;

use clap::Parser;
use ignore::{WalkBuilder, types::TypesBuilder};
use mantra::{
    cfg::MantraConfigFile,
    cmd::collect::{
        self,
        cfg::{CollectArguments, CollectConfig, CollectEnvironmentVariables},
    },
    db::MantraDb,
};
use mantra_schema::path::PathExt;

#[tokio::main]
async fn main() {
    // let root_path = "./"; // std::path::absolute("./").unwrap();

    // let mut types_builder = TypesBuilder::new();
    // // types_builder.add("xml", "*.xml").unwrap();
    // // types_builder.select("xml");
    // types_builder.add_defaults();
    // let types = types_builder.build().unwrap();

    // let mut walk_builder = WalkBuilder::new(&root_path);
    // walk_builder.types(types);
    // walk_builder.follow_links(true);
    // let walker = walk_builder.build();

    // for entry in walker {
    //     if let Ok(entry) = entry
    //         && entry.path().is_file()
    //     {
    //         let rel_path = entry.path().relative_to(&root_path).unwrap();
    //         // println!("{rel_path}");
    //         println!("{}", entry.path().display());
    //     }
    // }

    let cfg_path = PathBuf::from("./mantra.json5");
    let cfg_content = tokio::fs::read_to_string(&cfg_path).await.unwrap();
    let cfg = json5::from_str::<MantraConfigFile>(&cfg_content).unwrap();

    // let cfg = json5::from_str::<collect::cfg::CollectTestRunsConfig>(&cfg_content).unwrap();

    let db = MantraDb::new(Some("sqlite://mantra_test.db?mode=rwc"))
        .await
        .unwrap();

    for product_cfg in cfg.products {
        let collect_cfg = CollectConfig {
            cfg_filepath: cfg_path.clone(),
            args: CollectArguments {
                replace_hashed: false,
            },
            envs: CollectEnvironmentVariables {},
            product: product_cfg.product,
            requirements: product_cfg.requirements,
            annotations: product_cfg.annotations,
            test_runs: product_cfg.test_runs,
            reviews: product_cfg.reviews,
        };

        collect::collect(&db, collect_cfg).await.unwrap();
    }

    // let cfg = mantra::cfg::Config::parse();

    // env_logger::builder()
    //     .filter_level(log::LevelFilter::Info)
    //     .format_target(false)
    //     .init();

    // if let Err(err) = mantra::run(cfg).await {
    //     println!("{err}");
    //     std::process::exit(-1);
    // }
}
