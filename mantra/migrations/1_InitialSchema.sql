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
-- [req("changes.track.reqs", "req.title", "req.description", "req.properties", "req.manual", "req.deprecated")
create table RequirementContents (
    -- The SAH256 hash of the requirement content.
    hash text not null primary key,
    -- The title of the requirement.
    title text not null,
    -- Optional description of the requirement.
    description text,
    -- Optional properties of the requirement.
    properties text,
    -- Flag indicating whether the requirement requires manual verification.
    -- `true`: The requirement requires manual verification.
    manual_verification bool,
    -- Flag indicating whether the requirement is deprecated.
    -- `true`: The requirement is deprecated.
    deprecated bool
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
-- [req("changes.track")
create table FileHashes (
    -- Filepath of a file that containes content that is stored in the database.
    filepath text not null,
    -- SHA256 hash of the file to be able to map to a *snapshot* of the file content.
    hash text not null,
    primary key (filepath, hash)
);

-- Table to map file hashes to `mantra collect` runs.
-- [req("changes.track")
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
-- [req("trace.origin", "changes.track")
create table TracedLines (
    -- Filepath of the file the trace was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at in the file.
    line integer not null,
    primary key (filepath, file_hash, line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- Table to store properties of traces.
-- [req("trace-properties")]
create table TraceProperties (
    -- File the trace was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    -- Property of the trace. e.g. "satisfies", "verifies"
    property text not null,
    primary key (filepath, file_hash, line, property),
    foreign key (filepath, file_hash, line) references TracedLines (filepath, file_hash, line) on delete cascade
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
    -- Libe the trace was detected at.
    line integer not null,
    primary key (req_id, filepath, file_hash, line),
    foreign key (filepath, file_hash, line) references TracedLines (filepath, file_hash, line) on delete cascade
);

-- Table to store language elements such as function, test, struct, enum, class, ...
--
-- Note: Elements are uniquely identifiable by filepath and line number.
-- Due to feature flags or language semantics, idents may be declared multiple times, and are therefore not unique.
-- [req("trace.element")]
create table Elements (
    -- Optional ident for the element.
    ident text,
    -- File the element is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the element definition starts.
    -- [req("trace.element.span")]
    start_line integer not null,
    -- Line the element definition ends.
    -- [req("trace.element.span")]
    end_line integer not null,
    -- Type of the element.
    -- [req("trace.element.kind")]
    kind integer not null,
    -- Hash of the content of the element.
    content_hash text not null references ElementContents (content_hash),
    primary key (filepath, file_hash, start_line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade,
    constraint start_le_end check (start_line <= end_line)
);

-- Table to store the content of an element.
-- [req("report.coverage.content")]
create table ElementContents (
    -- The hash of the content.
    content_hash text not null primary key,
    -- The element content.
    content text not null
);

-- Table to store direct links between elements and traces.
--
-- ```rust
-- #[derive(Debug)]... <- element start line
-- #[req("trace.element")]  ... <- traced line
-- fn foo() {}
-- ```
--
-- [req("trace.element")]
create table DirectTracedElements (
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    element_start_line integer not null,
    primary key (
        filepath,
        file_hash,
        traced_line,
        element_start_line
    ),
    foreign key (filepath, file_hash, element_start_line) references Elements (filepath, file_hash, start_line) on delete cascade,
    foreign key (filepath, file_hash, traced_line) references TracedLines (filepath, file_hash, line) on delete cascade
);

create table DirectElementReferences (
    origin_filepath text not null,
    origin_file_hash text not null,
    origin_start_line integer not null,
    ref_filepath text not null,
    ref_file_hash text not null,
    ref_line integer not null,
    primary key (
        origin_filepath,
        origin_file_hash,
        origin_start_line,
        ref_filepath,
        ref_file_hash,
        ref_line
    ),
    foreign key (
        origin_filepath,
        origin_file_hash,
        origin_start_line
    ) references Elements (filepath, file_hash, start_line) on delete cascade,
    foreign key (ref_filepath, ref_file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- traces to requirements that were not part of the database when the trace was added.
create table UnrelatedDirectReqTraces (
    req_id text not null,
    filepath text not null,
    file_hash text not null,
    line integer not null,
    primary key (req_id, filepath, file_hash, line),
    foreign key (filepath, file_hash, line) references TracedLines (filepath, file_hash, line) on delete cascade
);

-- test runs that executed tests
--
-- NOTE: `nr_of_test_cases` is the number of expected test cases in one run.
-- Meaning, if there are fewer associated test cases in the `TestCases` table,
-- not all test cases were executed.
create table TestRuns (
    name text not null,
    utc_date text not null,
    revision integer not null,
    nr_of_test_cases integer not null,
    primary key (name, utc_date, revision)
);

create table TestRunCollections (
    collect_hash text not null references Collections (hash) on delete cascade,
    name text not null,
    utc_date text not null,
    revision integer not null,
    content_hash text not null,
    source_filepath text not null,
    source_file_hash text not null,
    primary key (collect_hash, name, utc_date, revision),
    foreign key (name, utc_date, revision) references TestRuns (name, utc_date, revision) on delete cascade,
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, file_hash) on delete cascade
);

create table TestRunChanges (
    name text not null,
    utc_date text not null,
    revision integer not null,
    comment text not null,
    authors text not null,
    primary key (name, utc_date, revision),
    foreign key (name, utc_date, revision) references TestRuns (name, utc_date, revision) on delete cascade
);

create table TestRunHierarchies (
    parent_name text not null,
    parent_utc_date text not null,
    parent_revision integer not null,
    child_name text not null,
    child_utc_date text not null,
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

create table TestRunMetadata (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
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

create table TestRunLogs (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
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

create table TestRunStatementCoverage (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
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

create table TestCases (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    name text not null,
    -- 0=failed; 1=passed; 2=skipped; null = running/not executed
    state integer,
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

create table TestCaseMetadata (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
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

create table TestCaseLogs (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
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

create table TestCaseLocations (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    filepath text not null,
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

create table TestCaseStateReason (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
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

create table TestCaseStatementCoverage (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
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

-- review to add manually verified requirements
create table Reviews (
    name text not null,
    utc_date text not null,
    revision integer not null,
    reviewer text not null,
    description text,
    primary key (name, utc_date, revision)
);

create table ReviewCollections (
    collect_hash text not null references Collections (hash) on delete cascade,
    name text not null,
    utc_date text not null,
    revision integer not null,
    source_filepath text not null,
    source_file_hash text not null,
    primary key (collect_hash, name, utc_date, revision),
    foreign key (name, utc_date, revision) references Reviews (name, utc_date, revision) on delete cascade,
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, file_hash) on delete cascade
);

-- manually verified requirements
create table ManuallyVerified (
    req_id text not null references Requirements (id) on delete cascade,
    review_name text not null,
    review_utc_date text not null,
    review_revision integer not null,
    comment text,
    primary key (
        req_id,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade
);

-- manually verified requirements
create table UnrelatedManuallyVerified (
    req_id text not null,
    review_name text not null,
    review_utc_date text not null,
    review_revision integer not null,
    comment text,
    primary key (
        req_id,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (review_name, review_utc_date, review_revision) references Reviews (name, utc_date, revision) on delete cascade
);

create table TestCaseOverrides (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    review_name text not null,
    review_utc_date text not null,
    review_revision integer not null,
    -- 0=failed; 1=passed; 2=skipped
    state integer not null,
    comment text,
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

create table TestRunStatementCoverageOverrides (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    review_name text not null,
    review_utc_date text not null,
    review_revision integer not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    comment text,
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

create table TestCaseStatementCoverageOverrides (
    test_run_name text not null,
    test_run_utc_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    review_name text not null,
    review_utc_date text not null,
    review_revision integer not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    comment text,
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
