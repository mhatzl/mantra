use std::path::PathBuf;

use mantra_lang_tracing::path::SlashPathBuf;
use mantra_schema::{requirements::{ReqId, Requirement}, traces::{FileTraceInfo, ItemEntry, TraceEntry}, Line};

use super::{DbError, MantraDb};

#[derive(Debug, Clone)]
pub struct RequirementUpdate {
    pub old: Requirement,
    pub new: Requirement,
}

#[derive(Debug, Default, Clone)]
pub struct RequirementChanges {
    pub updated: Vec<RequirementUpdate>,
    pub inserted: Vec<Requirement>,
    pub unchanged_cnt: usize,
}

impl RequirementChanges {
    pub fn merge(&mut self, other: &mut Self) {
        self.updated.append(&mut other.updated);
        self.inserted.append(&mut other.inserted);
        self.unchanged_cnt += other.unchanged_cnt;
    }
}

impl std::fmt::Display for RequirementChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.updated.is_empty() && self.inserted.is_empty() {
            if self.unchanged_cnt == 0 {
                writeln!(f, "No requirements found.")?;
            } else {
                writeln!(f, "'{}' requirements kept.", self.unchanged_cnt)?;
            }
        } else {
            if !self.updated.is_empty() {
                writeln!(f, "'{}' requirements updated:", self.updated.len())?;
                for req in &self.updated {
                    writeln!(f, "- `{}`", req.new.id)?;
                }
            }

            if !self.inserted.is_empty() {
                writeln!(f, "'{}' requirements added:", self.inserted.len())?;
                for req in &self.inserted {
                    writeln!(f, "- `{}`", req.id)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ItemEntryUpdate {
    pub old: ItemEntry,
    pub new: ItemEntry,
}

#[derive(Debug, Clone)]
pub struct ItemTraceUpdate {
    pub traced_line: Line,
    pub old_item_start: Line,
    pub new_item_start: Line,
}

#[derive(Debug, Default, Clone)]
pub struct FileTraceInfoChanges {
    pub inserted_traces: Vec<TraceEntry>,
    pub inserted_items: Vec<ItemEntry>,
    pub updated_item_traces: Vec<ItemTraceUpdate>,
    pub updated_items: Vec<ItemEntryUpdate>,
    pub unchanged_traces_cnt: usize,
    pub unchanged_items_cnt: usize,
}

impl FileTraceInfoChanges {
    pub fn merge(&mut self, other: &mut Self) {
        self.inserted_traces.append(&mut other.inserted_traces);
        self.inserted_items.append(&mut other.inserted_items);
        self.updated_item_traces.append(&mut other.updated_item_traces);
        self.updated_items.append(&mut other.updated_items);
        self.unchanged_traces_cnt += other.unchanged_traces_cnt;
        self.unchanged_items_cnt += other.unchanged_items_cnt;
    }
}

impl std::fmt::Display for FileTraceInfoChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.inserted_traces.is_empty() && self.updated_item_traces.is_empty() {
            if self.unchanged_traces_cnt == 0 {
                writeln!(f, "No traces found.")?;
            } else {
                writeln!(f, "'{}' traces kept.", self.unchanged_traces_cnt)?;
            }
        } else {
            if !self.inserted_traces.is_empty() {
                writeln!(f, "'{}' traces added:", self.inserted_traces.len())?;
                for trace in &self.inserted_traces {
                    writeln!(f, "- {}", trace)?;
                }
            }
            
            if !self.updated_item_traces.is_empty() {
                writeln!(f, "'{}' trace to item relations updated:", self.updated_item_traces.len())?;
                for updated_trace in &self.updated_item_traces {
                    writeln!(f, "- trace at line '{}' changed associated item line from '{}' to '{}'", updated_trace.traced_line, updated_trace.old_item_start, updated_trace.new_item_start)?;
                }
            }
        }

        if self.inserted_items.is_empty() && self.updated_items.is_empty() {
            if self.unchanged_items_cnt == 0 {
                writeln!(f, "No items found.")?;
            } else {
                writeln!(f, "'{}' items kept.", self.unchanged_items_cnt)?;
            }
        } else {
            if !self.inserted_items.is_empty() {
                writeln!(f, "'{}' items added:", self.inserted_items.len())?;
                for item in &self.inserted_items {
                    writeln!(f, "- {}", item)?;
                }
            }
            
            if !self.updated_items.is_empty() {
                writeln!(f, "'{}' items updated:", self.updated_items.len())?;
                for item_update in &self.updated_items {
                    writeln!(f, "- `{}` -> `{}`", item_update.old, item_update.new)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct FileTraceChange {
    pub filepath: PathBuf,
    pub changes: FileTraceInfoChanges,
}

impl std::fmt::Display for FileTraceChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Changes in file '{}':\n{}", self.filepath.display(), self.changes)
    }}


impl MantraDb {
    pub async fn add_reqs(
        &self,
        reqs: &[Requirement],
        check_time: time::OffsetDateTime,
    ) -> Result<RequirementChanges, DbError> {
        let mut changes = RequirementChanges::default();

        for req in reqs {
            let existing_parents = self.find_req_parents(&req.id).await;

            if let Ok(existing_record) = sqlx::query!(
                "select id, content_hash, title, origin, data, manual, deprecated from Requirements where id = $1",
                req.id
            )
            .fetch_one(&self.pool)
            .await
            {
                let existing_req = Requirement {
                    id: existing_record.id,
                    content_hash: record.content_hash,
                    title: existing_record.title,
                    origin: existing_record.origin,
                    data: existing_record.data.map(|a| {
                        serde_json::to_value(a).expect("Requirement info must be valid JSON.")
                    }),
                    manual: existing_record.manual,
                    deprecated: existing_record.deprecated,
                    parents: existing_parents,
                };
                if req != &existing_req {
                    changes.updated.push(RequirementUpdate {
                        old: existing_req,
                        new: req.clone(),
                    });

                    let _ = sqlx::query!(
                        "update Requirements set last_checked_at = $2, last_modified_at = $2, content_hash = $3, title = $4, origin = $5, data = $6, manual = $7, deprecated = $8 where id = $1",
                        req.id,
                        check_time,
                        req.content_hash,
                        req.title,
                        req.origin,
                        req.data,
                        req.manual,
                        req.deprecated
                    )
                    .execute(&self.pool)
                    .await;
                } else {
                    changes.unchanged_cnt += 1;

                    let _ = sqlx::query!(
                        "update Requirements set last_checked_at = $2 where id = $1",
                        req.id,
                        check_time
                    )
                    .execute(&self.pool)
                    .await;
                }
            } else {
                let res = sqlx::query!(
                    "insert into Requirements (id, last_checked_at, last_modified_at, content_hash, title, origin, data, manual, deprecated) values ($1, $2, $2, $3, $4, $5, $6, $7, $8)",
                    req.id,
                    check_time,
                    req.content_hash,
                    req.title,
                    req.origin,
                    req.data,
                    req.manual,
                    req.deprecated
                )
                .execute(&self.pool)
                .await;

                if let Err(err) = res {
                    log::error!(
                        "Adding requirement '{}' failed with error: {}",
                        &req.id,
                        err
                    );
                } else {
                    changes.inserted.push(req.clone());
                }
            }
        }

        for req in &changes.inserted {
            if let Some((parent, _)) = req.id.rsplit_once('.') {
                let parent_exists =
                    sqlx::query!("select id from requirements where id = $1", parent)
                        .fetch_one(&self.pool)
                        .await
                        .is_ok();

                let existing_parent = if parent_exists {
                    parent.to_string()
                } else {
                    self.get_req_id_parent(parent)
                        .await
                        .ok_or(DbError::Insert(format!(
                            "Parent is missing for child='{}'.",
                            req.id
                        )))?
                };

                let res = sqlx::query!(
                    "insert or ignore into RequirementHierarchies (parent_id, child_id) values ($1, $2)",
                    existing_parent,
                    req.id,
                )
                .execute(&self.pool)
                .await;

                if let Err(err) = res {
                    return Err(DbError::Insert(format!(
                        "Adding requirement hierarchy for parent='{}' and child='{}' failed with error: {}",
                        existing_parent, req.id, err
                    )));
                }
            }

            if let Some(parents) = &req.parents {
                for parent in parents {
                    let res = sqlx::query!(
                        "insert or ignore into RequirementHierarchies (parent_id, child_id) values ($1, $2)",
                        parent,
                        req.id,
                    )
                    .execute(&self.pool)
                    .await;

                    if let Err(err) = res {
                        return Err(DbError::Insert(format!(
                            "Adding requirement hierarchy for parent='{}' and child='{}' failed with error: {}",
                            parent, req.id, err
                        )));
                    }
                }
            }
        }

        Ok(changes)
    }

    async fn find_req_parents(&self, req_id: &ReqId) -> Option<Vec<ReqId>> {
        sqlx::query!(
            "select parent_id from RequirementHierarchies where child_id = $1",
            req_id
        )
        .fetch_all(&self.pool)
        .await
        .ok()
    }

    async fn get_req_id_parent(&self, mut id: &str) -> Option<String> {
        while let Some((parent, _)) = id.rsplit_once('.') {
            let parent_exists = sqlx::query!("select id from requirements where id = $1", parent)
                .fetch_one(&self.pool)
                .await
                .is_ok();

            if parent_exists {
                return Some(parent.to_string());
            } else {
                id = parent;
            }
        }

        None
    }

    pub async fn add_trace_info(
        &self,
        traced_files: &[FileTraceInfo],
        check_time: time::OffsetDateTime,
    ) -> Result<Vec<FileTraceChange>, DbError> {
        let mut changes = Vec::new();

        for traced_file in traced_files {
            let mut file_changes = FileTraceChange { filepath: traced_file.filepath.clone(), changes: FileTraceInfoChanges::default() };

            let file = SlashPathBuf::from(traced_file.filepath);
            let file_str = file.to_string();

            if let Ok(existing_hash) = sqlx::query!("select content_hash from TraceableFiles where filepath = $1", file_str).fetch_one(&self.pool).await {
                if existing_hash == traced_file.content_hash {
                    // file hash unchanged => related trace and item info must have stayed the same
                    let _ = sqlx::query!("update TraceableFiles set last_checked_at = $2 where filepath = $1",
                            file_str,
                            check_time
                        ).execute(&self.pool).await;

                    continue
                }
            }

            let _ = sqlx::query!("insert into TraceableFiles (filepath, content_hash, last_modified_at, last_checked_at) values ($1, $2, $3, $3)",
                            file_str,
                            traced_file.content_hash,
                            check_time,
                            check_time,
                        ).execute(&self.pool).await;

            // Note: items must be handled before traces due to key contraints in db schema
            for item in &traced_file.items {
                if let Some(existing_item) = sqlx::query!("select ident, filepath, start_line, end_line from Items where filepath = $1 and start_line = $2", file_str, item.span.start).fetch_one(&self.pool).await {
                    //TODO: handle test items

                    if existing_item.ident != item.ident || existing_item.end_line != item.span.end {
                        // item changed
                        let _ = sqlx::query!("update Items set ident = $3, end_line = $4 where filepath = $1 and start_line = $2",
                            file_str,
                            item.span.start,
                            item.ident,
                            item.span.end
                        ).execute(&self.pool).await;
                    } else {
                        let _ = sqlx::query!("insert into Items (ident, filepath, start_line, end_line) values ($1, $2, $3, $4)",
                            item.ident,
                            file_str,
                            item.span.start,
                            item.span.end
                        ).execute(&self.pool).await;
                    }
                }
            }

            for trace in &traced_file.traces {
                let _ = sqlx::query!("insert or ignore into TracedLines (filepath, line) values ($1, $2)",
                            file_str,
                            trace.line
                        ).execute(&self.pool).await;

                if let Some(item_start) = trace.item_start_line {
                    // TODO: handle change info
                    let _ = sqlx::query!("insert or ignore into DirectTracedItems (filepath, traced_line, item_start_line) values ($1, $2, $3)",
                            file_str,
                            trace.line,
                            item_start
                        ).execute(&self.pool).await;
                }

                let mut inserted_trace_entry = TraceEntry { ids: Vec::new(), line: trace.line, item_start_line: trace.item_start_line };

                for id in &trace.ids {

                    if (sqlx::query!("select req_id, filepath, line from DirectReqTraces where req_id = $1 and filepath = $2 and line = $3", id, file_str, trace.line).fetch_one(&self.pool).await).is_ok() {
                        file_changes.changes.unchanged_traces_cnt += 1;
                    } else {
                        let res = sqlx::query!(
                            "insert into DirectReqTraces (req_id, filepath, line) values ($1, $2, $3)",
                            id,
                            file_str,
                            trace.line
                        )
                        .execute(&self.pool)
                        .await;

                        if let Err(sqlx::Error::Database(err)) = res {
                            if err.kind() == sqlx::error::ErrorKind::ForeignKeyViolation {
                                log::warn!("Unrelated trace. No requirement with id `{}` found for trace at file='{}', line='{}",
                                    id, file_str, trace.line);
                                
                                    let res = sqlx::query!(
                                        "insert or ignore into UnrelatedDirectReqTraces (req_id, filepath, line) values ($1, $2, $3)",
                                        id,
                                        file_str,
                                        trace.line,
                                    )
                                    .execute(&self.pool)
                                    .await;

                                if let Err(err) = res {
                                    log::error!("Adding unrelated trace for id=`{}`, file='{}', line='{}' failed with error: {}",
                                    id, file_str, trace.line, err);
                                }
                            } else {
                                log::error!("Adding trace for id=`{}`, file='{}', line='{}' failed with error: {}",
                                    id, file_str, trace.line, err);
                            }
                        } else {
                            inserted_trace_entry.ids.push(id.clone());
                        }
                    }
                }

                if !inserted_trace_entry.ids.is_empty() {
                    file_changes.changes.inserted_traces.push(inserted_trace_entry);
                }
            }
            
            changes.push(file_changes);
        }

        Ok(changes)
    }
}
