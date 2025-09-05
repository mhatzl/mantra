-- Base table used to track changes over multiple `mantra collect` runs.
-- [req("lifecycle.versioning", "changes.track")]
create table Collections (
    -- SHA256 hash over all data that was collected when running `mantra collect`.
    hash text not null primary key,
    -- UTC timestamp from the first execution of `mantra collect` whose collected data matched this hash.
    added_at_utc text not null,
    -- UTC timestamp from the last execution of `mantra collect` whose collected data matched this hash.
    updated_at_utc text not null,
    constraint ch_times check (added_at_utc <= updated_at_utc)
);

-- Additional metadata that may be set in `mantra.toml` when running `mantra collect`.
--
-- TODO: Currently no requirement. decide if table is needed.
create table CollectionMetadata (
    collect_hash text not null primary key references Collections (hash) on delete cascade,
    data text
);

-- Table contains projects that were collected via `mantra collect`.
-- [req("lifecycle.versioning.id", "report.project_data")]
create table Projects (
    -- Name of a project.
    name text not null,
    -- Version of a project.
    version text not null,
    -- Optional URL to the project's homepage.
    homepage text,
    -- Optional URL to the project's repository.
    repository text,
    -- Optional license of the project.
    license text,
    -- Optional metadata of the project.
    data text,
    primary key (name, version)
);

-- Table to link between projects and collections.
-- [req("lifecycle.versioning.id")]
create table ProjectCollections (
    -- Hash of the data collected via `mantra collect`.
    collect_hash text not null references Collections (hash) on delete cascade,
    -- Project name that was set for the collected data.
    project_name text not null,
    -- Project version that was set for the collected data.
    project_version text not null,
    foreign key (project_name, project_version) references Projects (name, version) on delete cascade,
    primary key (project_name, project_version, collect_hash)
);

-- Table containing all requirement IDs collected by mantra.
-- [req("req.id", "changes.track.reqs.id")]
create table Requirements (id text not null primary key);

-- Table to link between collections and requirements.
-- [req("lifecycle.versioning", "changes.track")]
create table RequirementCollections (
    -- Hash of the data collected via `mantra collect`.
    collect_hash text not null references Collections (hash) on delete cascade,
    -- The requirement ID that maps to the content hash in the particular collection.
    req_id text not null references Requirements (id) on delete cascade,
    -- The requirement content hash that maps to general information about a requirement.
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    -- The relative source filepath this data was collected from.
    source_filepath text not null,
    -- The hash of the source file.
    source_file_hash text not null,
    primary key (collect_hash, req_id),
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- Stores general requirements content such as title and description.
--
-- **Note:** Multiple IDs may have the same content.
-- However, this likely indicates a rename of the requirement ID.
-- [req("changes.track.reqs")]
create table RequirementContents (
    -- The SAH256 hash of the requirement content.
    hash text not null primary key,
    -- The title of the requirement.
    -- [req("req.title")]
    title text not null,
    -- Optional description of the requirement.
    -- [req("req.description")]
    description text,
    -- Flag indicating whether the requirement requires manual verification.
    -- `true`: The requirement requires manual verification.
    -- [req("req.manual")]
    manual_verification bool not null,
    -- Flag indicating whether the requirement is deprecated.
    -- `true`: The requirement is deprecated.
    -- [req("req.deprecated")]
    deprecated bool not null
);

-- Table to store custom properties of requirements.
-- [req("req.properties")]
create table CustomRequirementProperties (
    -- The hash of the requirement content.
    req_content_hash text not null primary key,
    -- Custom property of the trace. e.g. "critical"
    property text not null,
    foreign key (req_content_hash) references RequirementContents (hash) on delete cascade
);

-- Table to store the wiki origins of requirement definitions.
-- [req("req.origin.wiki")]
create table RequirementWikiOrigins (
    -- The hash of the requirement content.
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    -- The relative filepath to the wiki page that defines the requirement.
    -- Relative from the root directory set in `mantra.toml` to the file that defines the requirement.
    filepath text not null,
    -- The line number in the file where the requirement is defined.
    line int not null,
    -- Optional URL to the repository of the wiki.
    repo_url text,
    -- Optional URL to the rendered view of the wiki.
    rendered_url text,
    primary key (req_content_hash)
);

-- Table to store external origins of requirements.
-- [req("req.origin.external")]
create table RequirementsExternalOrigins (
    -- The hash of the requirement content.
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    -- The URL a requirement is defined at externally to mantra.
    url text not null,
    primary key (req_content_hash)
);

-- Table to represent the requirement hierarchy per requirement content.
--
-- **Note:** Per requirement content, because the parent IDs are part of the content.
-- [req("req.hierarchy", "changes.track.reqs")]
create table RequirementHierarchies (
    -- The hash of the requirement content.
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    -- The ID of the requirement, whose content referenced the parent ID.
    req_id text not null references Requirements (id) on delete cascade,
    -- The ID of the parent requirement.
    parent_id text not null references Requirements (id) on delete cascade,
    primary key (req_content_hash, req_id, parent_id)
);

-- Table to store hashes of files containing content that is stored in the database.
-- [req("changes.track.traces.files")]
create table FileHashes (
    -- Filepath of a file that containes content that is stored in the database.
    filepath text not null,
    -- SHA256 hash of the file to be able to map to a *snapshot* of the file content.
    hash text not null,
    primary key (filepath, hash)
);

-- Table to map file hashes to `mantra collect` runs.
-- [req("changes.track")]
create table CollectedFileHashes (
    -- Hash of the collected content.
    collect_hash text not null references Collections (hash) on delete cascade,
    -- Filepath of a file content was collected from.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    primary key (collect_hash, filepath, file_hash),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- Table to store all traces.
-- [req("trace.origin", "changes.track")]
create table Traces (
    -- Filepath of the file the trace was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at in the file.
    line integer not null,
    -- Trace kind (0 = clarifies, 1 = satisfies, 2 = verifies, 3 = links).
    -- [req("trace.kind")]
    kind integer not null,
    primary key (filepath, file_hash, line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- Table to store custom properties of traces.
-- [req("trace.properties")]
create table CustomTraceProperties (
    -- File the trace was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    -- Custom property of the trace. e.g. "critical"
    property text not null,
    primary key (filepath, file_hash, line, property),
    foreign key (filepath, file_hash, line) references Traces (filepath, file_hash, line) on delete cascade
);

-- Table to store relations between traces and requirements.
-- [req("trace.id", "trace.mult_reqs")]
create table DirectReqTraces (
    -- Requirement ID that is directly set on the trace.
    req_id text not null references Requirements (id) on delete cascade,
    -- File the trace to the requirement was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    primary key (req_id, filepath, file_hash, line),
    foreign key (filepath, file_hash, line) references Traces (filepath, file_hash, line) on delete cascade
);

-- Table to store language elements such as functions, tests, structs, enums, classes, ...
--
-- Note: Elements are uniquely identifiable by filepath and line number.
-- Due to feature flags or language semantics, idents may be declared multiple times, and are therefore not unique.
-- [req("trace.element")]
create table Elements (
    -- Identifier for the element.
    ident text not null,
    -- File the element is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the element is defined at.
    definition_line integer not null,
    -- Line the element span starts.
    -- [req("trace.element.span")]
    start_line integer not null,
    -- Line the element span ends.
    -- [req("trace.element.span")]
    end_line integer not null,
    -- Type of the element.
    -- [req("trace.element.kind")]
    kind integer not null,
    -- Hash of the content of the element.
    content_hash text not null references CodeContents (content_hash),
    primary key (filepath, file_hash, definition_line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade,
    constraint start_le_end check (start_line <= end_line),
    constraint def_in_span check (start_line <= definition_line <= end_line)
);

-- Table to store language code blocks that are linked to traces.
-- [req("trace.code_block")]
create table CodeBlocks (
    -- File the code block is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the code block span starts.
    -- [req("trace.code_block.span")]
    start_line integer not null,
    -- Line the code block span ends.
    -- [req("trace.code_block.span")]
    end_line integer not null,
    -- Hash of the content of the code block.
    content_hash text not null references CodeContents (content_hash),
    primary key (filepath, file_hash, start_line),
    foreign key (filepath, file_hash, start_line) references Traces (filepath, file_hash, line) on delete cascade,
    constraint start_le_end check (start_line <= end_line)
);

-- Table to store the content of elements and code blocks.
-- [req("report.coverage.content", "trace.element", "trace.code_block")]
create table CodeContents (
    -- The hash of the content.
    content_hash text not null primary key,
    -- The element or code block content.
    content text not null
);

-- Table to store direct links between elements and traces.
--
-- ```rust
-- #[derive(Debug)]         ... <- element start line
-- #[req("trace.element")]  ... <- traced line
-- fn foo() {               ... <- definition line
--   //...
-- }                        ... <- end line
-- ```
--
-- [req("trace.element")]
create table DirectTracedElements (
    -- File the element is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace related to the element was detected at.
    traced_line integer not null,
    -- Line the element is defined at.
    element_definition_line integer not null,
    primary key (
        filepath,
        file_hash,
        traced_line,
        element_definition_line
    ),
    foreign key (filepath, file_hash, element_definition_line) references Elements (filepath, file_hash, definition_line) on delete cascade,
    foreign key (filepath, file_hash, traced_line) references Traces (filepath, file_hash, line) on delete cascade
);

-- Table to store where an element is referenced.
-- [req("testcov.static_approx")]
create table DirectElementReferences (
    -- File the element is defined in.
    origin_filepath text not null,
    -- Hash of the file the element is defined in.
    origin_file_hash text not null,
    -- Line the element is defined at.
    origin_definition_line integer not null,
    -- File the element is referenced in.
    ref_filepath text not null,
    -- Hash of the file the element is referenced in.
    ref_file_hash text not null,
    -- Line the element is referenced at.
    ref_line integer not null,
    primary key (
        origin_filepath,
        origin_file_hash,
        origin_definition_line,
        ref_filepath,
        ref_file_hash,
        ref_line
    ),
    foreign key (
        origin_filepath,
        origin_file_hash,
        origin_definition_line
    ) references Elements (filepath, file_hash, definition_line) on delete cascade,
    foreign key (ref_filepath, ref_file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- Table to store traces to requirements that were not part of the database
-- when the trace was added via `mantra collect`.
-- [req("analyze.validate.store_invalid")]
create table UnrelatedDirectReqTraces (
    -- Hash of the collected content.
    collect_hash text not null references Collections (hash) on delete cascade,
    -- The requirement ID that was not part of the requirements table at collection time.
    req_id text not null,
    -- File the trace to the requirement was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    primary key (collect_hash, req_id, filepath, file_hash, line),
    foreign key (filepath, file_hash, line) references Traces (filepath, file_hash, line) on delete cascade
);

-- Table to store test runs that executed one or more test cases.
-- [req("testcov.test_run", "changes.track.test_runs")]
create table TestRuns (
    -- The name of the test run.
    name text not null,
    -- The UTC date and time at which the test run was executed.
    utc_date text not null,
    -- Indicates the revision of a test run to track retrospective changes.
    revision integer not null,
    -- The number of expected test cases mapped to the test run.
    -- Meaning, if there are fewer associated test cases in the `TestCases` table,
    -- not all test cases were executed.
    nr_of_test_cases integer not null,
    primary key (name, utc_date, revision)
);

-- Table to store the mapping between a `mantra collect` invocation
-- and the collected test run.
-- [req("changes.track")]
create table TestRunCollections (
    -- Hash of the collected content.
    collect_hash text not null references Collections (hash) on delete cascade,
    -- The name of the test run.
    test_run_name text not null,
    -- The UTC date and time at which the test run was executed.
    test_run_utc_date text not null,
    -- Indicates the revision of a test run to track retrospective changes.
    test_run_revision integer not null,
    -- Hash of the test run content for this collection.
    content_hash text not null,
    -- File the test run data was collected from.
    source_filepath text not null,
    -- Hash of the file content the test run was collected from.
    source_file_hash text not null,
    primary key (collect_hash, test_run_name, test_run_utc_date, test_run_revision),
    foreign key (test_run_name, test_run_utc_date, test_run_revision) references TestRuns (name, utc_date, revision) on delete cascade,
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, file_hash) on delete cascade
);

-- Table to store retrospective changes to a test run.
-- [req("changes.track.test_runs")]
create table TestRunChanges (
    -- The name of the test run.
    test_run_name text not null,
    -- The UTC date and time at which the test run was executed.
    test_run_utc_date text not null,
    -- Indicates the revision of a test run to track retrospective changes.
    test_run_revision integer not null,
    -- The comment explaining the changes.
    comment text not null,
    -- The authors resonsiple for the changes.
    authors text not null,
    primary key (test_run_name, test_run_utc_date, test_run_revision),
    foreign key (test_run_name, test_run_utc_date, test_run_revision) references TestRuns (name, utc_date, revision) on delete cascade
);

-- Table to store test run hierarchies.
-- This allows to have nested test runs,
-- while each test run may additionally have regular test cases.
-- [req("testcov.test_run.nested")]
create table TestRunHierarchies (
    -- The name of the parent test run.
    parent_name text not null,
    -- The UTC date and time of the parent test run.
    parent_utc_date text not null,
    -- The revision of the parent test run.
    parent_revision integer not null,
    -- The name of the child test run.
    child_name text not null,
    -- The UTC date and time of the child test run.
    child_utc_date text not null,
    -- The revision of the child test run.
    child_revision integer not null,
    primary key (
        parent_name,
        parent_utc_date,
        parent_revision,
        child_name,
        child_utc_date,
        child_revision
    ),
    foreign key (parent_name, parent_utc_date, parent_revision) references TestRuns (name, utc_date, revision) on delete cascade,
    foreign key (child_name, child_utc_date, child_revision) references TestRuns (name, utc_date, revision) on delete cascade
);

-- Table to store metadata for test runs.
-- [req("testcov.test_run.metadata")]
create table TestRunMetadata (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- JSON formatted metadata of a test run.
    data text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (name, utc_date, revision) on delete cascade
);

-- Table to store logs that were captured during test run execution.
--
-- **Note:** Separate table to `TestRunMetadata`, because logs at test run level should be rare,
-- which would lead to a field next to `data` that is mostly `null`.
-- [req("testcov.test_case.metadata")]
create table TestRunLogs (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Logs captured during the test run execution.
    logs text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (name, utc_date, revision) on delete cascade
);

-- Table to store statement coverage per test run.
-- [req("testcov.cov.lines"])
create table TestRunStatementCoverage (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    hits integer not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (name, utc_date, revision) on delete cascade
);

-- Table to store test case results.
-- [req("testcov.test_case")]
create table TestCases (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    name text not null,
    -- State of the test case.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed
    -- [req("testcov.test_case.state")]
    state integer not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (name, utc_date, revision) on delete cascade
);

-- Table to store metadata of a test case.
-- [req("testcov.test_case.metadata")]
create table TestCaseMetadata (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- JSON formatted metadata of the test case.
    data text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store logs of a test case.
--
-- **Note:** Logs separate to metadata table, because test cases likely have no metadata
-- besides logs, so the `data` field would be mostly `null` if logs and metadata would be in one table.
-- The test run tables are split, because there `logs` is assumed to be mostly `null.
-- [req("testcov.test_case.metadata")]
create table TestCaseLogs (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Logs that were captured during the execution of the test case.
    logs text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store the locations of test cases where the location
-- can be mapped to a file tracked in the database.
-- [req("testcov.test_case.origin", "changes.track")]
create table TestCaseTrackedLocations (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- File the test case is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the test case is defined at.
    line integer not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade,
    foreign key (
        filepath,
        file_hash
    ) references FileHashes (
        filepath,
        file_hash
    )
);

-- Table to store the locations of test cases, in case the location
-- cannot be tracked to files stored in the database.
--
-- **Note:** This table does not link to `FileHashes`, because test cases
-- may be defined in files that are not tracked by *mantra*.
-- Furthermore, the hash of the file content is seldomly part of test report formats.
-- [req("testcov.test_case.origin")]
create table TestCaseUntrackedLocations (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- File the test case is defined in.
    filepath text not null,
    -- Line the test case is defined at.
    line integer not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store the reason for the state of a test case.
-- This is mostly needed for *skipped* test cases.
-- [req("testcov.test_case.state")]
create table TestCaseStateReason (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- The reason for the state of a test case.
    reason text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store statement coverage per test case.
-- [req("testcov.cov.lines"])
create table TestCaseStatementCoverage (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during the test case execution.
    hits integer not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store reviews.
-- [req("review", "changes.track")]
create table Reviews (
    -- Name of the review
    name text not null,
    -- UTC date and time at which the review was held.
    utc_date text not null,
    -- Indicates the revision of a review to track retrospective changes.
    revision integer not null,
    -- The reviewers of the review.
    -- [req("review.reviewer")]
    reviewer text not null,
    -- Optional decription for the review.
    -- [req("review.description")]
    description text,
    primary key (name, utc_date, revision)
);

-- Table to map review to `mantra collect` runs.
-- [req("changes.track")]
create table ReviewCollections (
    -- Hash of the collected content.
    collect_hash text not null references Collections (hash) on delete cascade,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- The file the review content was collected from.
    source_filepath text not null,
    -- The hash of the file content the review was collected from.
    source_file_hash text not null,
    primary key (collect_hash, review_name, review_utc_date, review_revision),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade,
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, file_hash) on delete cascade
);

-- Table to store retrospective changes to a review.
-- [req("changes.track.reviews")]
create table ReviewChanges (
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Indicates the revision of a review to track retrospective changes.
    review_revision integer not null,
    -- The comment explaining the changes.
    comment text not null,
    -- The authors resonsiple for the changes.
    authors text not null,
    primary key (review_name, review_utc_date, review_revision),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade
);

-- Table to store requirement IDs that were manually verified in a review,
-- and the IDs could be mapped to requirements stored in the database.
-- [req("review.verify_req")]
create table ManuallyVerifiedRequirements (
    -- ID of the requirement that is manually verified.
    req_id text not null references Requirements (id) on delete cascade,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- Optional comment for the manual verification.
    comment text not null,
    primary key (
        req_id,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade
);

-- Table to store requirement IDs that were manually verified in a review,
-- but the IDs could not be mapped to requirements stored in the database.
-- [req("review.verify_req", "analyze.validate.store_invalid")]
create table UnrelatedManuallyVerifiedRequirements (
    -- ID of the requirement that is manually verified.
    req_id text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- Optional comment for the manual verification.
    comment text not null,
    primary key (
        req_id,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade
);

-- Table to store test case overrides from reviews.
-- [req("review.test_case_state")]
create table TestCaseOverrides (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- State that must be used instead of the one stored in the TestCase table.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed
    state integer not null,
    -- Optional comment explaining why the state must be overriden.
    comment text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade,
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    )
);

-- Table to store overrides from reviews for statement coverage entries of test runs.
-- [req("review.coverage", "testcov.cov.lines")]
create table TestRunStatementCoverageOverrides (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    hits integer not null,
    -- Optional comment explaining why this statement coverage must be overriden.
    comment text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        review_name,
        review_utc_date,
        review_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade,
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    ) references TestRunStatementCoverage (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    )
);

-- Table to store overrides from reviews for statement coverage entries of test cases.
-- [req("review.coverage", "testcov.cov.lines")]
create table TestCaseStatementCoverageOverrides (
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    hits integer not null,
    -- Optional comment explaining why this statement coverage must be overriden.
    comment text not null,
    primary key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        review_name,
        review_utc_date,
        review_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade,
    foreign key (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ) references TestCaseStatementCoverage (
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    )
);
