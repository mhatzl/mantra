use std::path::PathBuf;

use anyhow::Context;
use glob::Pattern;
use ignore::{
    WalkBuilder, WalkState,
    types::{Types, TypesBuilder},
};
use mantra_schema::{
    FmtHash,
    path::{PathExt, RelativePath, RelativePathBuf},
    test_runs::{CoveredFile, TestRun, TestRunSchema},
    time::OffsetDateTime,
};
use tokio::task::JoinHandle;

use crate::cmd::collect::{
    cfg::{
        CollectTestRunsConfig, TestRunSourceVariant, WellKnownCoverage, WellKnownCoverageFormat,
        WellKnownTest, WellKnownTestFormat,
    },
    collector::CollectableFile,
    product_collection::ProductCollection,
    test_runs::convert::{
        ShallowTestRun, WellKnownCoverageConversion, WellKnownCoverageData, WellKnownTestConversion,
    },
    walker,
};

mod convert;
pub mod db;

#[cfg(test)]
mod tests;

pub(super) async fn collect<'db, 'c>(
    collection: &mut ProductCollection<'db, 'c>,
    cfgs: Vec<(i64, CollectTestRunsConfig)>,
) -> Result<(), anyhow::Error> {
    if cfgs.is_empty() {
        return Ok(());
    }

    for (cfg_nr, cfg) in cfgs {
        match cfg.source {
            TestRunSourceVariant::WellKnown { test, coverage } => collect_well_known(
                collection,
                cfg_nr,
                &cfg.path,
                cfg.pattern.as_deref(),
                test,
                coverage,
            )
            .await
            .with_context(|| {
                format!("Failed collecting well-known test data from '{}'", cfg.path)
            })?,
            TestRunSourceVariant::Schema => {
                collect_schema(collection, cfg_nr, &cfg.path, cfg.pattern.as_deref())
                    .await
                    .with_context(|| {
                        format!(
                            "Failed collecting schema-based test data from '{}'",
                            cfg.path
                        )
                    })?;
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn collect_well_known<'db, 'c>(
    collection: &mut ProductCollection<'db, 'c>,
    cfg_nr: i64,
    path: &RelativePath,
    pattern: Option<&str>,
    test: WellKnownTest,
    coverage: WellKnownCoverage,
) -> Result<(), anyhow::Error> {
    let abs_cfg_file_dir_path = collection.abs_cfg_file_parent_path();
    let mut shallow_test_run_data = Vec::new();
    let mut coverage_data = Vec::new();

    let (well_known_sender, mut well_known_rx) = tokio::sync::mpsc::unbounded_channel();
    let start_path = path.to_logical_path(&abs_cfg_file_dir_path);
    let general_glob_pattern = pattern.and_then(|p| glob::Pattern::new(p).ok());
    let test_glob_pattern = glob::Pattern::new(&test.pattern)?;
    let coverage_glob_pattern = glob::Pattern::new(&coverage.pattern)?;

    let well_known_collection: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        let mut walk_builder = base_test_run_walker(start_path, general_glob_pattern);
        walk_builder.types(allowed_types(&test.format, &coverage.format)?);

        walk_builder.build_parallel().run(|| {
            let sender = well_known_sender.clone();
            let root_path = abs_cfg_file_dir_path.clone();
            let test_format = test.format;
            let coverage_format = coverage.format;
            let test_glob_pattern = test_glob_pattern.clone();
            let coverage_glob_pattern = coverage_glob_pattern.clone();

            Box::new(move |path_res| {
                if let Ok(path) = path_res {
                    let filepath = path.path();
                    if filepath.is_file() {
                        let matches_test_format = test_glob_pattern.matches_path(filepath);
                        let matches_coverage_format = coverage_glob_pattern.matches_path(filepath);

                        if (matches_test_format || matches_coverage_format) && let Some(ext) = filepath.extension()
                                && let Some(extension) = ext.to_str()
                                && let Ok(content) = crate::io::sync_read_encoding_independent(filepath)
                        {
                            let rel_filepath = filepath.relative_to(&root_path)
                                .expect("Creating relative path succeeds, because root path for walker is absolute.");
                            let file_hash = FmtHash::new(&content);

                            if matches_test_format {
                                // TODO: proper error handling
                                match test_format.to_shallow_test_run(&root_path, extension, &content) {
                                    Ok(shallow_test_run) => {
                                        let data = SentWellKnownData {
                                            data: CollectedWellKnown::Test(Box::new(shallow_test_run)),
                                            filepath: rel_filepath,
                                            file_hash,
                                            content,
                                        };
                                        let _ = sender.send(data);
                                    }
                                    Err(err) => log::error!(
                                        "Failed collecting test run data from well-known test output '{}'. Err: {err}",
                                        filepath.display()
                                    ),
                                }
                            } else {
                                // must match coverage format
                                match coverage_format.to_well_known_coverage(&root_path, extension, &content) {
                                    Ok(coverage_data) => {
                                        let data = SentWellKnownData {
                                            data: CollectedWellKnown::Coverage(coverage_data),
                                            filepath: rel_filepath,
                                            file_hash,
                                            content,
                                        };
                                        let _ = sender.send(data);
                                    }
                                    Err(err) => log::error!(
                                        "Failed collecting coverage data from well-known coverage output '{}'. Err: {err}",
                                        filepath.display()
                                    ),
                                }
                            }
                        }
                    }
                }

                WalkState::Continue
            })
        });

        Ok(())
    });

    let mut schema_sources = Vec::new();

    while let Some(sent_data) = well_known_rx.recv().await {
        collection
            .insert_collected_file(
                &sent_data.filepath,
                &sent_data.file_hash,
                Some(&sent_data.content),
            )
            .await
            .with_context(|| {
                format!(
                    "Failed inserting the collected file '{}'",
                    sent_data.filepath
                )
            })?;
        schema_sources.push(sent_data.filepath);

        match sent_data.data {
            CollectedWellKnown::Test(shallow_test_run) => {
                shallow_test_run_data.push(shallow_test_run);
            }
            CollectedWellKnown::Coverage(well_known_coverage_data) => {
                coverage_data.push(well_known_coverage_data);
            }
        }
    }

    let _ = well_known_collection
        .await
        .context("Failed collecting well-known test data")?;

    // merge shallow test runs + coverage data to proper test run
    if shallow_test_run_data.is_empty() {
        log::warn!("No well-known test outputs found.");
        return Ok(());
    }

    let (covered_files, coverage_timestamp) = merge_well_known_coverage_data(coverage_data);

    let test_run = if shallow_test_run_data.len() == 1 {
        // only one test run => place all coverage data into it
        let shallow_data = shallow_test_run_data
            .into_iter()
            .next()
            .expect("Checked above that one test run was collected");
        let test_run = shallow_data.into_test_run(covered_files, coverage_timestamp);

        test_run
    } else {
        // unclear which test run maps to which coverage data => create new test run whith the collected ones as children
        let (name, utc_date, nr_test_cases, test_runs) =
            to_sub_test_runs(shallow_test_run_data, coverage_timestamp);

        TestRun {
            name,
            utc_date,
            description: None,
            revisions: None,
            origin: None,
            nr_of_test_cases: nr_test_cases,
            properties: None,
            duration_sec: None,
            logs: None,
            test_cases: vec![],
            covered_files,
            test_runs,
            media_type: None,
        }
    };

    let test_run_schema = TestRunSchema {
        schema_version: None,
        product_id: None,
        test_runs: vec![test_run],
        test_run_properties: None,
        test_case_properties: None,
        origin: None,
    };

    let schema_hash = collection
        .insert_schema_multi_sources(&test_run_schema, &schema_sources, cfg_nr)
        .await?;

    collection
        .collect_test_run_schema(&test_run_schema, &schema_hash)
        .await
        .context("Failed updating test run data collected from well-known formats")?;

    Ok(())
}

fn to_sub_test_runs(
    shallow_test_run_data: Vec<Box<ShallowTestRun>>,
    coverage_timestamp: Option<OffsetDateTime>,
) -> (String, OffsetDateTime, u32, Vec<TestRun>) {
    let mut test_run_names = Vec::new();
    let mut earliest_utc_date = None;
    let mut nr_test_cases = 0;

    let test_run_data = shallow_test_run_data
        .into_iter()
        .map(|s| {
            test_run_names.push(s.name.clone());
            nr_test_cases += s.nr_of_test_cases;

            if s.utc_date.is_some()
                && (earliest_utc_date.is_none() || earliest_utc_date > s.utc_date)
            {
                earliest_utc_date = s.utc_date;
            }

            s.into_test_run(vec![], coverage_timestamp)
        })
        .collect();

    let name = FmtHash::from(&test_run_names).to_string();
    let utc_date = match earliest_utc_date.or(coverage_timestamp) {
        Some(timestamp) => timestamp,
        None => {
            log::warn!(
                "No timestamp collected in any collected well-known test output. Using local timestamp."
            );
            OffsetDateTime::now_utc()
        }
    };

    (name, utc_date, nr_test_cases, test_run_data)
}

fn merge_well_known_coverage_data(
    coverage_data: Vec<WellKnownCoverageData>,
) -> (Vec<CoveredFile>, Option<OffsetDateTime>) {
    if coverage_data.is_empty() {
        (vec![], None)
    } else if coverage_data.len() == 1 {
        let coverage = coverage_data
            .into_iter()
            .next()
            .expect("Checked above that one coverage element was collected");
        (coverage.covered_files, coverage.timestamp)
    } else {
        let mut covered_files = Vec::new();
        let mut timestamp = None;

        for coverage in coverage_data {
            covered_files.extend(coverage.covered_files);

            if timestamp.is_none()
                && let Some(coverage_timestamp) = coverage.timestamp
            {
                timestamp = Some(coverage_timestamp);
            }
        }

        (covered_files, timestamp)
    }
}

struct SentWellKnownData {
    data: CollectedWellKnown,
    filepath: RelativePathBuf,
    file_hash: FmtHash,
    content: String,
}

enum CollectedWellKnown {
    /// A test run collected from a well-known test output format.
    /// This will not contain coverage information yet, because no well-known test format contains this information.
    Test(Box<ShallowTestRun>),
    Coverage(WellKnownCoverageData),
}

fn allowed_types(
    test_format: &WellKnownTestFormat,
    coverage_format: &WellKnownCoverageFormat,
) -> Result<Types, anyhow::Error> {
    let mut builder = TypesBuilder::new();
    match test_format {
        WellKnownTestFormat::Junit => {
            builder.add("xml", "*.xml")?;
            builder.select("xml");
        }
    }
    match coverage_format {
        WellKnownCoverageFormat::CoberturaLoose => {
            builder.add("xml", "*.xml")?;
            builder.select("xml");
            builder.add("json", "*.json")?;
            builder.select("json");
            builder.add("json5", "*.json5")?;
            builder.select("json5");
        }
    }

    Ok(builder.build()?)
}

struct SentSchemaData {
    schema: TestRunSchema,
    filepath: RelativePathBuf,
    file_hash: FmtHash,
    content: String,
}

async fn collect_schema<'db, 'c>(
    collection: &mut ProductCollection<'db, 'c>,
    cfg_nr: i64,
    path: &RelativePath,
    pattern: Option<&str>,
) -> Result<(), anyhow::Error> {
    let product_id = collection.product_id().clone();
    let abs_cfg_file_dir_path = collection.abs_cfg_file_parent_path();

    let (schema_sender, mut schema_rx) = tokio::sync::mpsc::unbounded_channel();
    let start_path = path.to_logical_path(&abs_cfg_file_dir_path);
    let glob_pattern = pattern.and_then(|p| glob::Pattern::new(p).ok());
    let schema_collection: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        let mut walk_builder = base_test_run_walker(start_path, glob_pattern);
        walk_builder.types(walker::base_schema_types()?);

        let collect_fn = walker::content_to_schema::<TestRunSchema>;

        walk_builder.build_parallel().run(|| {
            let pid = product_id.clone();
            let root_path = abs_cfg_file_dir_path.clone();
            let sender = schema_sender.clone();
            Box::new(move |path_res| {
                if let Ok(path) = path_res {
                    let filepath = path.path();
                    if filepath.is_file()
                        && let Ok(content) = crate::io::sync_read_encoding_independent(filepath)
                        && let Ok(rel_filepath) = filepath.relative_to(&root_path)
                    {
                        let file_hash = FmtHash::new(&content);
                        let file = CollectableFile::new(&rel_filepath, &file_hash, &content);

                        // TODO: proper error handling
                        match collect_fn(&pid, &file) {
                            Ok(Some(schema)) => {
                                let data = SentSchemaData {
                                    schema,
                                    filepath: rel_filepath,
                                    file_hash,
                                    content,
                                };
                                let _ = sender.send(data);
                            }
                            Ok(None) => {
                                log::info!("Nothing collected from file '{}'", filepath.display());
                            }
                            Err(err) => log::error!(
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

    while let Some(mut sent_data) = schema_rx.recv().await {
        collection
            .insert_collected_file(
                &sent_data.filepath,
                &sent_data.file_hash,
                Some(&sent_data.content),
            )
            .await
            .with_context(|| {
                format!(
                    "Failed inserting the collected file '{}'",
                    sent_data.filepath
                )
            })?;

        let schema_hash = collection
            .insert_schema_single_sources(&sent_data.schema, &sent_data.filepath, cfg_nr)
            .await?;

        collection
            .collect_test_run_schema(&sent_data.schema, &schema_hash)
            .await
            .with_context(|| {
                format!(
                    "Failed updating test run data collected from file '{}'",
                    sent_data.filepath
                )
            })?;
    }

    let _ = schema_collection.await?;

    Ok(())
}

pub(super) fn base_test_run_walker(
    start_path: PathBuf,
    glob_pattern: Option<Pattern>,
) -> WalkBuilder {
    let mut walker = walker::base_mantra_walker(start_path, glob_pattern);
    walker.add_custom_ignore_filename(".mantraignore-test_runs");
    // test output is typically located in folders that are excluded from git
    walker.git_ignore(false);
    walker
}
