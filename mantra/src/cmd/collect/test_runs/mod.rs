use std::path::PathBuf;

use glob::Pattern;
use ignore::{
    WalkBuilder, WalkState,
    types::{Types, TypesBuilder},
};
use mantra_schema::{
    FmtHash, Origin, Properties,
    path::RelativePath,
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
    test_runs::convert::{
        ShallowTestRun, WellKnownCoverageConversion, WellKnownCoverageData, WellKnownTestConversion,
    },
    walker,
};

mod convert;
pub mod db;

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
    let cfg_file_dir_path = collection.abs_cfg_file_parent_path()?;
    let mut shallow_test_runs = Vec::new();
    let mut coverage_data = Vec::new();

    let (well_known_sender, mut well_known_rx) = tokio::sync::mpsc::unbounded_channel();
    let start_path = path.to_logical_path(&cfg_file_dir_path);
    let general_glob_pattern = pattern.and_then(|p| glob::Pattern::new(p).ok());
    let test_glob_pattern = glob::Pattern::new(&test.pattern)?;
    let coverage_glob_pattern = glob::Pattern::new(&coverage.pattern)?;

    let well_known_collection: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        let mut walk_builder = base_test_run_walker(start_path, general_glob_pattern);
        walk_builder.types(allowed_types(&test.format, &coverage.format)?);

        walk_builder.build_parallel().run(|| {
            let sender = well_known_sender.clone();
            let root_path = cfg_file_dir_path.clone();
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
                                && let Ok(content) = std::fs::read_to_string(filepath)
                            {
                                if matches_test_format {
                                    // TODO: proper logging + error handling
                                    match test_format.to_test_run(&root_path, extension, &content) {
                                        Ok(shallow_test_run) => {
                                            let _ = sender.send(CollectedWellKnown::Test(shallow_test_run));
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
                                            let _ = sender.send(CollectedWellKnown::Coverage(coverage_data));
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

    while let Some(well_known) = well_known_rx.recv().await {
        match well_known {
            CollectedWellKnown::Test(shallow_test_run) => {
                shallow_test_runs.push(shallow_test_run);
            }
            CollectedWellKnown::Coverage(well_known_coverage_data) => {
                coverage_data.push(well_known_coverage_data);
            }
        }
    }

    let _ = well_known_collection.await?;

    // merge shallow test runs + coverage data to proper test run
    if shallow_test_runs.is_empty() {
        eprintln!("No well-known test outputs found.");
        return Ok(());
    }

    let (covered_files, coverage_timestamp) = merge_well_known_coverage_data(coverage_data);

    let test_run = if shallow_test_runs.len() == 1 {
        // only one test run => place all coverage data into it
        let shallow_test_run = shallow_test_runs
            .into_iter()
            .next()
            .expect("Checked above that one test run was collected");
        shallow_test_run.to_test_run(covered_files, coverage_timestamp)
    } else {
        // unclear which test run maps to which coverage data => create new test run whith the collected ones as children
        let (name, utc_date, nr_test_cases, test_runs) =
            to_sub_test_runs(shallow_test_runs, coverage_timestamp);

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

    let test_run_schema = TestRunSchema {
        version: None,
        test_runs: vec![test_run],
        test_run_properties,
        test_case_properties,
        origin,
    };

    collection
        .update_per_test_run_schema(test_run_schema)
        .await?;

    Ok(())
}

fn to_sub_test_runs(
    shallow_test_runs: Vec<ShallowTestRun>,
    coverage_timestamp: Option<OffsetDateTime>,
) -> (String, OffsetDateTime, u32, Vec<TestRun>) {
    let mut test_run_names = Vec::new();
    let mut earliest_utc_date = None;
    let mut nr_test_cases = 0;

    let test_runs = shallow_test_runs
        .into_iter()
        .map(|s| {
            test_run_names.push(s.name.clone());
            nr_test_cases += s.nr_of_test_cases;

            if s.utc_date.is_some()
                && (earliest_utc_date.is_none() || earliest_utc_date > s.utc_date)
            {
                earliest_utc_date = s.utc_date;
            }

            s.to_test_run(vec![], coverage_timestamp)
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

    (name, utc_date, nr_test_cases, test_runs)
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

async fn collect_schema<'db>(
    collection: &mut Collection<'db>,
    path: &RelativePath,
    origin: Option<Origin>,
    test_run_properties: Option<Properties>,
    test_case_properties: Option<Properties>,
    pattern: Option<&str>,
) -> Result<(), anyhow::Error> {
    let cfg_file_dir_path = collection.abs_cfg_file_parent_path()?;

    let (schema_sender, mut schema_rx) = tokio::sync::mpsc::unbounded_channel();
    let start_path = path.to_logical_path(&cfg_file_dir_path);
    let glob_pattern = pattern.and_then(|p| glob::Pattern::new(p).ok());
    let schema_collection: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        let mut walk_builder = base_test_run_walker(start_path, glob_pattern);
        walk_builder.types(walker::base_schema_types()?);

        let collect_fn = walker::content_to_schema::<TestRunSchema>;

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

    while let Some(schema) = schema_rx.recv().await {
        collection.update_per_test_run_schema(schema).await?;
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
