use std::marker::PhantomData;

use ignore::{WalkBuilder, WalkState};
use mantra_schema::path::{RelativePath, RelativePathBuf};
use tokio::task::JoinSet;

use crate::cmd::collect::{Collection, walker};

pub(super) struct SingleFileCollector<'db, T, C: SingleFileCollectable<'db, T>> {
    collection: Collection<'db>,
    cfgs: PhantomData<C>,
    schema: PhantomData<T>,
}

pub(super) trait SingleFileCollectable<'db, T> {
    fn path(&self) -> &RelativePath;
    fn pattern(&self) -> Option<&str>;
    fn custom_ignore_filename(&self) -> &'static str;
    fn modify_walker(&self, builder: &mut WalkBuilder) -> Result<(), anyhow::Error>;
    fn collect_fn(&self) -> Result<fn(&str, &str) -> Result<T, anyhow::Error>, anyhow::Error>;
    async fn update_db(collection: &mut Collection<'db>, schema: T) -> Result<(), anyhow::Error>;
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

        let cfg_file_dir_path = self.collection.abs_cfg_file_parent_path()?;

        let (schema_tx, mut schema_rx) = tokio::sync::mpsc::unbounded_channel();
        let root_path = cfg_file_dir_path.clone();
        let schema_collection = tokio::spawn(async move {
            let mut task_set: JoinSet<Result<(), anyhow::Error>> = JoinSet::new();
            for cfg in cfgs {
                let schema_sender = schema_tx.clone();
                let start_path = cfg.path().to_logical_path(&root_path);
                let glob_pattern = cfg.pattern().and_then(|p| glob::Pattern::new(p).ok());
                task_set.spawn(async move {
                    let mut walk_builder = walker::base_mantra_walker(start_path);
                    walk_builder.add_custom_ignore_filename(cfg.custom_ignore_filename());
                    if let Some(pattern) = glob_pattern {
                        walk_builder.filter_entry(move |entry| {
                            entry.path().is_dir()
                                || match RelativePathBuf::from_path(entry.path()) {
                                    Ok(rel_path) => pattern.matches(rel_path.as_str()),
                                    Err(_) => false,
                                }
                        });
                    }

                    cfg.modify_walker(&mut walk_builder)?;

                    let collect_fn = cfg.collect_fn()?;

                    // let collect_fn: fn(&str, &str) -> Result<RequirementSchema, anyhow::Error> =
                    //     match cfg.source {
                    //         RequirementSourceVariant::Markup => {
                    //             walk_builder.types(TypesBuilder::new().select("markdown").build()?);
                    //             |extension: &str, content: &str| todo!()
                    //         }
                    //         RequirementSourceVariant::Schema => {
                    //             walk_builder.types(walker::base_schema_types()?);
                    //             walker::content_to_schema::<RequirementSchema>
                    //         }
                    //     };

                    walk_builder.build_parallel().run(|| {
                        let sender = schema_sender.clone();
                        Box::new(move |path_res| {
                            if let Ok(path) = path_res {
                                let filepath = path.path();
                                if filepath.is_file() {
                                    if let Some(ext) = filepath.extension()
                                        && let Some(extension) = ext.to_str()
                                        && let Ok(content) = std::fs::read_to_string(filepath)
                                    {
                                        // TODO: proper logging + error handling
                                        match collect_fn(extension, &content) {
                                            Ok(schema) => {
                                                let _ = sender.send(schema);
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

        while let Some(schema) = schema_rx.recv().await {
            C::update_db(&mut self.collection, schema).await?;
        }

        schema_collection.await?;

        Ok(self.collection)
    }
}
