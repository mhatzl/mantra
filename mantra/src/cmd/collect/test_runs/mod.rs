use std::path::PathBuf;

use glob::Pattern;
use ignore::{
    WalkBuilder, WalkState,
    types::{Types, TypesBuilder},
};
use mantra_schema::{
    FmtHash, Origin, Properties,
    path::{PathExt, RelativePath, RelativePathBuf},
    test_runs::{CoveredFile, TestRun, TestRunSchema},
    time::OffsetDateTime,
};
use tokio::task::JoinHandle;

use crate::cmd::collect::{
    Collection,
    cfg::{
        CollectTestRunsConfig, TestRunSourceVariant, WellKnownCoverage, WellKnownCoverageFormat,
        WellKnownTest, WellKnownTestFormat,
    },
    collector::CollectableFile,
    test_runs::convert::{
        ShallowTestRun, WellKnownCoverageConversion, WellKnownCoverageData, WellKnownTestConversion,
    },
    walker,
};

mod convert;
pub mod db;

#[cfg(test)]
mod tests;

pub(super) async fn collect<'db>(
    collection: &mut Collection<'db>,
    cfgs: Vec<CollectTestRunsConfig>,
) -> Result<(), anyhow::Error> {
    if cfgs.is_empty() {
        return Ok(());
    }

    for cfg in cfgs {
        match cfg.source {
            TestRunSourceVariant::WellKnown { test, coverage } => {
                collect_well_known(
                    collection,
                    &cfg.path,
                    cfg.origin,
                    cfg.test_run_properties,
                    cfg.test_case_properties,
                    cfg.pattern.as_deref(),
                    test,
                    coverage,
                )
                .await?
            }
            TestRunSourceVariant::Schema => {
                collect_schema(
                    collection,
                    &cfg.path,
                    cfg.origin,
                    cfg.test_run_properties,
                    cfg.test_case_properties,
                    cfg.pattern.as_deref(),
                )
                .await?;
            }
        }
    }

    Ok(())
}

async fn collect_well_known<'db>(
    collection: &mut Collection<'db>,
    path: &RelativePath,
    origin: Option<Origin>,
    test_run_properties: Option<Properties>,
    test_case_properties: Option<Properties>,
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

                        if matches_test_format || matches_coverage_format {
                            if let Some(ext) = filepath.extension()
                                && let Some(extension) = ext.to_str()
                                && let Ok(content) = crate::io::sync_read_encoding_independent(filepath)
                            {
                                let rel_filepath = filepath.relative_to(&root_path)
                                    .expect("Creating relative path succeeds, because root path for walker is absolute.");
                                let file_hash = FmtHash::new(&content);

                                if matches_test_format {
                                    // TODO: proper logging + error handling
                                    match test_format.to_shallow_test_run(&root_path, extension, &content) {
                                        Ok(shallow_test_run) => {
                                            let data = SentWellKnownData {
                                                data: CollectedWellKnown::Test(shallow_test_run),
                                                filepath: rel_filepath,
                                                file_hash,
                                                content,
                                            };
                                            let _ = sender.send(data);
                                        }
                                        Err(err) => eprintln!(
                                            "Failed reading from well-known test format '{}'. Err: {err}",
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
                                        Err(err) => eprintln!(
                                            "Failed reading from well-known coverage format '{}'. Err: {err}",
                                            filepath.display()
                                        ),
                                    }
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

    while let Some(sent_data) = well_known_rx.recv().await {
        collection
            .insert_file_hash(&sent_data.filepath, &sent_data.file_hash)
            .await?;
        collection
            .insert_file_content(
                &sent_data.filepath,
                &sent_data.file_hash,
                &sent_data.content,
            )
            .await?;

        match sent_data.data {
            CollectedWellKnown::Test(shallow_test_run) => {
                shallow_test_run_data.push((sent_data.filepath, shallow_test_run));
            }
            CollectedWellKnown::Coverage(well_known_coverage_data) => {
                coverage_data.push((
                    sent_data.filepath,
                    sent_data.file_hash,
                    well_known_coverage_data,
                ));
            }
        }
    }

    let _ = well_known_collection.await?;

    // merge shallow test runs + coverage data to proper test run
    if shallow_test_run_data.is_empty() {
        eprintln!("No well-known test outputs found.");
        return Ok(());
    }

    let (coverage_source_files, covered_files, coverage_timestamp) =
        merge_well_known_coverage_data(coverage_data);

    let test_run = if shallow_test_run_data.len() == 1 {
        // only one test run => place all coverage data into it
        let shallow_data = shallow_test_run_data
            .into_iter()
            .next()
            .expect("Checked above that one test run was collected");
        let test_run = shallow_data
            .1
            .to_test_run(covered_files, coverage_timestamp);
        collection
            .insert_test_run_data_filepaths(&test_run.name, &test_run.utc_date, &shallow_data.0)
            .await?;

        test_run
    } else {
        // unclear which test run maps to which coverage data => create new test run whith the collected ones as children
        let (name, utc_date, nr_test_cases, test_run_data) =
            to_sub_test_runs(shallow_test_run_data, coverage_timestamp);

        let mut test_runs = Vec::with_capacity(test_run_data.len());
        for data in test_run_data {
            collection
                .insert_test_run_data_filepaths(&data.1.name, &data.1.utc_date, &data.0)
                .await?;
            test_runs.push(data.1);
        }

        TestRun {
            name,
            utc_date,
            description: None,
            revisions: None,
            origin: None,
            nr_of_test_cases: nr_test_cases,
            properties: None,
            duration: None,
            logs: None,
            test_cases: vec![],
            covered_files,
            test_runs,
        }
    };

    for source_file in coverage_source_files {
        collection
            .insert_test_run_data_filepaths(&test_run.name, &test_run.utc_date, &source_file.0)
            .await?;
    }

    let test_run_schema = TestRunSchema {
        schema_version: None,
        test_runs: vec![test_run],
        test_run_properties,
        test_case_properties,
        origin,
    };

    // Note: filepaths for collected well-known data has been inserted above
    collection
        .update_per_test_run_schema(None, test_run_schema)
        .await?;

    Ok(())
}

fn to_sub_test_runs(
    shallow_test_run_data: Vec<(RelativePathBuf, ShallowTestRun)>,
    coverage_timestamp: Option<OffsetDateTime>,
) -> (String, OffsetDateTime, u32, Vec<(RelativePathBuf, TestRun)>) {
    let mut test_run_names = Vec::new();
    let mut earliest_utc_date = None;
    let mut nr_test_cases = 0;

    let test_run_data = shallow_test_run_data
        .into_iter()
        .map(|s| {
            test_run_names.push(s.1.name.clone());
            nr_test_cases += s.1.nr_of_test_cases;

            if s.1.utc_date.is_some()
                && (earliest_utc_date.is_none() || earliest_utc_date > s.1.utc_date)
            {
                earliest_utc_date = s.1.utc_date;
            }

            (s.0, s.1.to_test_run(vec![], coverage_timestamp))
        })
        .collect();

    let name = FmtHash::from(&test_run_names).to_string();
    let utc_date = match earliest_utc_date.or(coverage_timestamp) {
        Some(timestamp) => timestamp,
        None => {
            // TODO: proper logging
            eprintln!("No timestamp collected. Using local timestamp.");
            OffsetDateTime::now_utc()
        }
    };

    (name, utc_date, nr_test_cases, test_run_data)
}

fn merge_well_known_coverage_data(
    coverage_data: Vec<(RelativePathBuf, FmtHash, WellKnownCoverageData)>,
) -> (
    Vec<(RelativePathBuf, FmtHash)>,
    Vec<CoveredFile>,
    Option<OffsetDateTime>,
) {
    if coverage_data.is_empty() {
        (vec![], vec![], None)
    } else if coverage_data.len() == 1 {
        let coverage = coverage_data
            .into_iter()
            .next()
            .expect("Checked above that one coverage element was collected");
        (
            vec![(coverage.0, coverage.1)],
            coverage.2.covered_files,
            coverage.2.timestamp,
        )
    } else {
        let mut source_files = Vec::with_capacity(coverage_data.len());
        let mut covered_files = Vec::new();
        let mut timestamp = None;

        for coverage in coverage_data {
            source_files.push((coverage.0, coverage.1));
            covered_files.extend(coverage.2.covered_files);

            if timestamp.is_none()
                && let Some(coverage_timestamp) = coverage.2.timestamp
            {
                timestamp = Some(coverage_timestamp);
            }
        }

        (source_files, covered_files, timestamp)
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
    Test(ShallowTestRun),
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

async fn collect_schema<'db>(
    collection: &mut Collection<'db>,
    path: &RelativePath,
    origin: Option<Origin>,
    test_run_properties: Option<Properties>,
    test_case_properties: Option<Properties>,
    pattern: Option<&str>,
) -> Result<(), anyhow::Error> {
    let abs_cfg_file_dir_path = collection.abs_cfg_file_parent_path();

    let (schema_sender, mut schema_rx) = tokio::sync::mpsc::unbounded_channel();
    let start_path = path.to_logical_path(&abs_cfg_file_dir_path);
    let glob_pattern = pattern.and_then(|p| glob::Pattern::new(p).ok());
    let schema_collection: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        let mut walk_builder = base_test_run_walker(start_path, glob_pattern);
        walk_builder.types(walker::base_schema_types()?);

        let collect_fn = walker::content_to_schema::<TestRunSchema>;

        walk_builder.build_parallel().run(|| {
            let root_path = abs_cfg_file_dir_path.clone();
            let sender = schema_sender.clone();
            Box::new(move |path_res| {
                if let Ok(path) = path_res {
                    let filepath = path.path();
                    if filepath.is_file() {
                        if let Ok(content) = crate::io::sync_read_encoding_independent(filepath)
                            && let Ok(rel_filepath) = filepath.relative_to(&root_path)
                        {
                            let file_hash = FmtHash::new(&content);
                            let file = CollectableFile::new(&rel_filepath, &file_hash, &content);

                            // TODO: proper logging + error handling
                            match collect_fn(&file) {
                                Ok(schema) => {
                                    let data = SentSchemaData {
                                        schema,
                                        filepath: rel_filepath,
                                        file_hash,
                                        content,
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

    while let Some(data) = schema_rx.recv().await {
        collection
            .insert_file_hash(&data.filepath, &data.file_hash)
            .await?;
        collection
            .insert_file_content(&data.filepath, &data.file_hash, &data.content)
            .await?;
        collection
            .update_per_test_run_schema(Some(&data.filepath), data.schema)
            .await?;
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
