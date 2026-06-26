use ignore::{WalkState, types::TypesBuilder};
use mantra_schema::{
    FmtHash,
    path::{PathExt, RelativePathBuf},
};
use tokio::task::JoinSet;

use crate::cmd::collect::{
    Collection, cfg::CollectLsifConfig, lsif::lsif_graph::LsifGraph, walker,
};

pub mod db;
pub mod lsif_graph;

impl<'db> Collection<'db> {
    pub(super) async fn resolve_element_identifier(
        &mut self,
        lsif_cfgs: Vec<CollectLsifConfig>,
    ) -> Result<(), anyhow::Error> {
        if lsif_cfgs.is_empty() {
            return Ok(());
        }

        let abs_cfg_file_dir_path = self.abs_cfg_file_parent_path();

        let (lsif_tx, mut lsif_rx) = tokio::sync::mpsc::unbounded_channel();
        let root = abs_cfg_file_dir_path.clone();
        let lsif_collection = tokio::spawn(async move {
            let mut task_set: JoinSet<Result<(), anyhow::Error>> = JoinSet::new();
            for cfg in lsif_cfgs {
                let lsif_sender = lsif_tx.clone();
                let root_path = root.clone();
                let start_path = cfg.path.to_logical_path(&root);
                let glob_pattern = cfg
                    .pattern
                    .as_deref()
                    .and_then(|p| glob::Pattern::new(p).ok());
                task_set.spawn(async move {
                    let mut walk_builder = walker::base_mantra_walker(start_path, glob_pattern);
                    walk_builder.add_custom_ignore_filename(".mantraignore-lsif");
                    walk_builder.types({
                        let mut types_builder = TypesBuilder::new();
                        types_builder.add("json", "*.json")?;
                        types_builder.select("json");
                        types_builder.add("jsonl", "*.jsonl")?;
                        types_builder.select("jsonl");
                        types_builder.build()?
                    });

                    walk_builder.build_parallel().run(|| {
                        let root_path = root_path.clone();
                        let sender = lsif_sender.clone();
                        Box::new(move |path_res| {
                            if let Ok(path) = path_res {
                                let filepath = path.path();
                                if filepath.is_file()
                                    && let Ok(content) =
                                        crate::io::sync_read_encoding_independent(filepath)
                                    && let Ok(rel_filepath) = filepath.relative_to(&root_path)
                                {
                                    let file_hash = FmtHash::new(&content);

                                    match LsifGraph::create(&content) {
                                        Ok(lsif_graph) => {
                                            let data = SentData {
                                                lsif_graph,
                                                filepath: rel_filepath,
                                                file_hash,
                                            };
                                            let _ = sender.send(data);
                                        }
                                        Err(err) => eprintln!(
                                            "Failed reading schema from '{}'. Err: {err}",
                                            filepath.display()
                                        ),
                                    }
                                }
                            }

                            WalkState::Continue
                        })
                    });

                    Ok(())
                });
            }

            let _ = task_set.join_all().await;
        });

        let mut lsif_graphs = Vec::new();

        while let Some(sent_data) = lsif_rx.recv().await {
            self.insert_file_hash(&sent_data.filepath, &sent_data.file_hash)
                .await?;
            lsif_graphs.push(sent_data.lsif_graph);
        }

        lsif_collection.await?;

        let elements = self.elements_missing_idents().await?;

        // update idents and filter those with ident == None;

        if !elements.is_empty() {
            self.update_element_idents(vec![]).await?;
        }

        Ok(())
    }
}

struct SentData {
    lsif_graph: LsifGraph,
    filepath: RelativePathBuf,
    file_hash: FmtHash,
}
