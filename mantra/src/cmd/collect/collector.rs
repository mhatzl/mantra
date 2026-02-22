use std::marker::PhantomData;

use ignore::{WalkBuilder, WalkState};
use mantra_schema::{
    FmtHash,
    path::{PathExt, RelativePath, RelativePathBuf},
};
use tokio::task::JoinSet;

use crate::cmd::collect::{Collection, sync_read_encoding_independent, walker};

pub(super) struct SingleFileCollector<'db, T, C: SingleFileCollectable<'db, T>> {
    collection: Collection<'db>,
    cfgs: PhantomData<C>,
    schema: PhantomData<T>,
}

pub(super) struct CollectableFile<'a> {
    pub(super) filepath: RelativePathBuf,
    pub(super) file_hash: FmtHash,
    pub(super) content: &'a str,
}

impl<'a> CollectableFile<'a> {
    pub(super) fn new(filepath: &RelativePath, file_hash: &FmtHash, content: &'a str) -> Self {
        Self {
            filepath: filepath.to_relative_path_buf(),
            file_hash: file_hash.clone(),
            content,
        }
    }

    pub(super) fn extension(&self) -> Option<&str> {
        self.filepath.extension()
    }
}

pub(super) trait SingleFileCollectable<'db, T> {
    fn path(&self) -> &RelativePath;
    fn pattern(&self) -> Option<&str>;
    fn custom_ignore_filename(&self) -> &'static str;
    fn modify_walker(&self, builder: &mut WalkBuilder) -> Result<(), anyhow::Error>;
    fn collect_fn(&self)
    -> Result<fn(&CollectableFile) -> Result<T, anyhow::Error>, anyhow::Error>;
    async fn update_db(
        collection: &mut Collection<'db>,
        filepath: &RelativePath,
        schema: T,
    ) -> Result<(), anyhow::Error>;
}

struct SentData<T> {
    schema: T,
    filepath: RelativePathBuf,
    file_hash: FmtHash,
}

impl<'db, T: Send + 'static, C: SingleFileCollectable<'db, T> + Send + 'static>
    SingleFileCollector<'db, T, C>
{
    pub fn new(collection: Collection<'db>) -> Self {
        Self {
            collection,
            cfgs: PhantomData,
            schema: PhantomData,
        }
    }

    pub(super) async fn collect(mut self, cfgs: Vec<C>) -> Result<Collection<'db>, anyhow::Error> {
        if cfgs.is_empty() {
            return Ok(self.collection);
        }

        let abs_cfg_file_dir_path = self.collection.abs_cfg_file_parent_path();

        let (schema_tx, mut schema_rx) = tokio::sync::mpsc::unbounded_channel();
        let root = abs_cfg_file_dir_path.clone();
        let schema_collection = tokio::spawn(async move {
            let mut task_set: JoinSet<Result<(), anyhow::Error>> = JoinSet::new();
            for cfg in cfgs {
                let schema_sender = schema_tx.clone();
                let root_path = root.clone();
                let start_path = cfg.path().to_logical_path(&root);
                let glob_pattern = cfg.pattern().and_then(|p| glob::Pattern::new(p).ok());
                task_set.spawn(async move {
                    let mut walk_builder = walker::base_mantra_walker(start_path, glob_pattern);
                    walk_builder.add_custom_ignore_filename(cfg.custom_ignore_filename());

                    cfg.modify_walker(&mut walk_builder)?;

                    let collect_fn = cfg.collect_fn()?;

                    walk_builder.build_parallel().run(|| {
                        let root_path = root_path.clone();
                        let sender = schema_sender.clone();
                        Box::new(move |path_res| {
                            if let Ok(path) = path_res {
                                let filepath = path.path();
                                if filepath.is_file() {
                                    if let Ok(content) = sync_read_encoding_independent(filepath)
                                        && let Ok(rel_filepath) = filepath.relative_to(&root_path)
                                    {
                                        let file_hash = FmtHash::new(&content);
                                        let file = CollectableFile::new(
                                            &rel_filepath,
                                            &file_hash,
                                            &content,
                                        );

                                        // TODO: proper logging + error handling
                                        match collect_fn(&file) {
                                            Ok(schema) => {
                                                let data = SentData {
                                                    schema,
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
                            }

                            WalkState::Continue
                        })
                    });

                    Ok(())
                });
            }

            let _ = task_set.join_all().await;
        });

        while let Some(sent_data) = schema_rx.recv().await {
            self.collection
                .insert_file_hash(&sent_data.filepath, &sent_data.file_hash)
                .await?;
            C::update_db(&mut self.collection, &sent_data.filepath, sent_data.schema).await?;
        }

        schema_collection.await?;

        Ok(self.collection)
    }
}
