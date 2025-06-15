-- base table used to track changes over multiple `mantra collect` runs.
-- [req("lifecycle.versioning", "changes.track")]
create table Collections (
    -- sha256 hash over all data that was collected when running `mantra collect`
    hash text not null primary key,
    -- UTC timestamp from the first execution of `mantra collect` whose collected data matched this hash
    added_at_utc text not null,
    -- UTC timestamp from the last execution of `mantra collect` whose collected data matched this hash
    updated_at_utc text not null,
    constraint ch_times check (added_at_utc <= updated_at_utc)
);

-- additional metadata that may be set in `mantra.toml` when running `mantra collect`
create table CollectionMetadata (
    collect_hash text not null primary key references Collections (hash) on delete cascade,
    data text
);

-- table contains projects that were collected via `mantra collect`.
-- [req("lifecycle.versioning.id")]
create table Projects (
    -- name of a project
    name text not null,
    -- version of a project
    version text not null,
    primary key (name, version)
);

-- table to link between projects and collections.
-- [req("lifecycle.versioning.id")]
create table ProjectCollections (
    collect_hash text not null references Collections (hash) on delete cascade,
    project_name text not null,
    project_version text not null,
    foreign key (project_name, project_version) references Projects (name, version) on delete cascade,
    primary key (project_name, project_version, collect_hash)
);

-- table containing all requirement IDs collected by mantra
-- [req("req.id")]
create table Requirements (id text not null primary key);

-- table to link between collections and requirements.
-- [req("lifecycle.versioning", "changes.track")]
create table RequirementCollections (
    collect_hash text not null references Collections (hash) on delete cascade,
    req_id text not null references Requirements (id) on delete cascade,
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    source_filepath text not null,
    source_file_hash text not null,
    primary key (collect_hash, req_id),
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, hash) on delete cascade
);

create table RequirementContents (
    hash text not null primary key,
    title text not null,
    description text,
    properties text,
    manual_verification bool not null,
    deprecated bool not null
);

create table RequirementWikiOrigins (
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    filepath text not null,
    line int not null,
    repo_url text,
    rendered_url text,
    primary key (req_content_hash)
);

create table RequirementsWebOrigins (
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    url text not null,
    primary key (req_content_hash)
);

-- Requirement hierarchy per requirement content
create table RequirementHierarchies (
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    req_id text not null references Requirements (id) on delete cascade,
    parent_id text not null references Requirements (id) on delete cascade,
    primary key (req_content_hash, req_id, parent_id)
);

--
create table FileHashes (
    filepath text not null,
    hash text not null,
    primary key (filepath, hash)
);

create table FileContents (
    filepath text not null,
    file_hash text not null,
    content text not null,
    primary key (filepath, file_hash),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade
);

create table CollectedFileHashes (
    collect_hash text not null references Collections (hash) on delete cascade,
    filepath text not null,
    file_hash text not null,
    primary key (collect_hash, filepath, file_hash),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- base for all traces to link req traces to items
create table TracedLines (
    filepath text not null,
    file_hash text not null,
    line integer not null,
    primary key (filepath, file_hash, line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade
);

create table TraceProperties (
    filepath text not null,
    file_hash text not null,
    line integer not null,
    property text not null,
    primary key (filepath, file_hash, line, property),
    foreign key (filepath, file_hash, line) references TracedLines (filepath, file_hash, line) on delete cascade
);

-- traces to requirements
create table DirectReqTraces (
    req_id text not null references Requirements (id) on delete cascade,
    filepath text not null,
    file_hash text not null,
    line integer not null,
    primary key (req_id, filepath, file_hash, line),
    foreign key (filepath, file_hash, line) references TracedLines (filepath, file_hash, line) on delete cascade
);

-- Language elements such as function, test, struct, enum, class, ...
-- Note: Elements are uniquely identifiable by filepath and line number.
-- Due to feature flags or language semantics, idents may be declared multiple times, and are therefore not unique.
create table Elements (
    ident text,
    filepath text not null,
    file_hash text not null,
    start_line integer not null,
    end_line integer not null,
    kind integer not null,
    primary key (filepath, file_hash, start_line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade,
    constraint start_le_end check (start_line <= end_line)
);

-- Element that is directly traced
-- e.g.
-- #[req(my_req)] ... <- traced line
-- fn foo() {}    ... <- element start line
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
-- NOTE: `nr_of_tests` is the number of expected tests in one run.
-- Meaning, if there are fewer associated tests in the Tests table, not all tests were executed.
create table TestRuns (
    name text not null,
    date text not null,
    revision integer not null,
    nr_of_tests integer not null,
    primary key (name, date, revision),
    constraint ch_time check (date <= last_checked_at)
);

create table TestRunCollections (
    collect_hash text not null references Collections (hash) on delete cascade,
    name text not null,
    date text not null,
    revision integer not null,
    content_hash text not null,
    source_filepath text not null,
    source_file_hash text not null,
    primary key (collect_hash, name, date, revision),
    foreign key (name, date, revision) references TestRuns (name, date, revision) on delete cascade,
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, file_hash) on delete cascade
);

create table TestRunChanges (
    name text not null,
    date text not null,
    revision integer not null,
    revision_date text not null,
    comment text not null,
    authors text not null,
    primary key (name, date, revision),
    foreign key (name, date, revision) references TestRuns (name, date, revision) on delete cascade,
    constraint ch_revision_date check (date <= revision_date)
);

create table TestRunHierarchies (
    parent_name text not null,
    parent_date text not null,
    parent_revision integer not null,
    child_name text not null,
    child_date text not null,
    child_revision integer not null,
    primary key (
        parent_name,
        parent_date,
        parent_revision,
        child_name,
        child_date,
        child_revision
    ),
    foreign key (parent_name, parent_date, parent_revision) references TestRuns (name, date, revision) on delete cascade,
    foreign key (child_name, child_date, child_revision) references TestRuns (name, date, revision) on delete cascade
);

create table TestRunMetadata (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    data text not null,
    primary key (test_run_name, test_run_date, test_run_revision),
    foreign key (test_run_name, test_run_date, test_run_revision) references TestRuns (name, date, revision) on delete cascade
);

create table TestRunLogs (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    logs text not null,
    primary key (test_run_name, test_run_date, test_run_revision),
    foreign key (test_run_name, test_run_date, test_run_revision) references TestRuns (name, date, revision) on delete cascade
);

create table TestRunStatementCoverage (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (test_run_name, test_run_date, test_run_revision) references TestRuns (name, date, revision) on delete cascade
);

create table TestCases (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    name text not null,
    -- 0=failed; 1=passed; 2=skipped; null = running/not executed
    state integer,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        name
    ),
    foreign key (test_run_name, test_run_date, test_run_revision) references TestRuns (name, date, revision) on delete cascade
);

create table TestCaseMetadata (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    data text not null,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_date,
        test_run_revision,
        name
    ) on delete cascade
);

create table TestCaseLogs (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    logs text not null,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_date,
        test_run_revision,
        name
    ) on delete cascade
);

create table TestCaseLocations (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    filepath text not null,
    line integer not null,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_date,
        test_run_revision,
        name
    ) on delete cascade
);

create table TestCaseStateReason (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    reason text not null,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_date,
        test_run_revision,
        name
    ) on delete cascade
);

create table TestCaseStatementCoverage (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- review to add manually verified requirements
create table Reviews (
    name text not null,
    date text not null,
    revision integer not null,
    reviewer text not null,
    description text,
    primary key (name, date, revision)
);

create table ReviewCollections (
    collect_hash text not null references Collections (hash) on delete cascade,
    name text not null,
    date text not null,
    revision integer not null,
    source_filepath text not null,
    source_file_hash text not null,
    primary key (collect_hash, name, date, revision),
    foreign key (name, date, revision) references Reviews (name, date, revision) on delete cascade,
    foreign key (source_filepath, source_file_hash) references FileHashes (filepath, file_hash) on delete cascade
);

-- manually verified requirements
create table ManuallyVerified (
    req_id text not null references Requirements (id) on delete cascade,
    review_name text not null,
    review_date text not null,
    review_revision integer not null,
    comment text,
    primary key (req_id, review_name, review_date, review_revision),
    foreign key (review_name, review_date, review_revision) references Reviews (name, date, revision) on delete cascade
);

-- manually verified requirements
create table UnrelatedManuallyVerified (
    req_id text not null,
    review_name text not null,
    review_date text not null,
    review_revision integer not null,
    comment text,
    primary key (req_id, review_name, review_date, review_revision),
    foreign key (review_name, review_date, review_revision) references Reviews (name, date, revision) on delete cascade
);

create table TestCaseOverrides (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    review_name text not null,
    review_date text not null,
    review_revision integer not null,
    -- 0=failed; 1=passed; 2=skipped
    state integer not null,
    comment text,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name,
        review_name,
        review_date,
        review_revision
    ),
    foreign key (review_name, review_date, review_revision) references Reviews (name, date, revision) on delete cascade,
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        test_run_name,
        test_run_date,
        test_run_revision,
        name
    )
);

create table TestRunStatementCoverageOverrides (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    review_name text not null,
    review_date text not null,
    review_revision integer not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    comment text,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        review_name,
        review_date,
        review_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (review_name, review_date, review_revision) references Reviews (name, date, revision) on delete cascade,
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    ) references TestRunStatementCoverage (
        test_run_name,
        test_run_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    )
);

create table TestCaseStatementCoverageOverrides (
    test_run_name text not null,
    test_run_date text not null,
    test_run_revision integer not null,
    test_case_name text not null,
    review_name text not null,
    review_date text not null,
    review_revision integer not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    comment text,
    primary key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name,
        review_name,
        review_date,
        review_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (review_name, review_date, review_revision) references Reviews (name, date, revision) on delete cascade,
    foreign key (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ) references TestCaseStatementCoverage (
        test_run_name,
        test_run_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    )
);
