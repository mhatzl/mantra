-- Base table used to track changes over multiple `mantra collect` runs.
-- [req("lifecycle.product", "changes.track")]
create table Collections (
    -- SHA256 hash over all data that was collected when running `mantra collect`.
    hash text not null primary key,
    -- Filepath to the `mantra.toml` file that was used to collect the data.
    -- [req("changes.track.origin")]
    collect_filepath text not null,
    -- UTC timestamp from the first execution of `mantra collect` whose collected data matched this hash.
    added_at_utc text not null,
    -- UTC timestamp from the last execution of `mantra collect` whose collected data matched this hash.
    updated_at_utc text not null,
    -- Optional names of authors involved in data collected.
    -- [req("changes.authors")]
    authors text,
    -- Optional comment explaining the data collected in the `mantra collect` run that resulted in this entry.
    -- [req("changes.comment")]
    comment text,
    -- SHA256 hash of the optional metadata that was collected.
    -- [req("changes.track.metadata")]
    metadata_hash text references GeneralJson (hash) on delete set null,
    constraint ch_times check (added_at_utc <= updated_at_utc)
);

-- Table to track collected information that was retrieved by a section in `mantra.toml`.
-- A section is either about requirements, traces, reviews, test runs, or product information.
create table CollectedSections (
    -- The hash of the collected information by this section.
    hash text not null primary key,
);

-- Table to map section contents to collections.
create table SectionCollections (
    -- The hash of a collection run.
    collect_hash test not null references Collections(hash) on delete cascade,
    -- The hash of a section that was collected in the related collection run.
    section_hash text not null references CollectedSections (hash) on delete cascade,
    -- The hash of the configuration in `mantra.toml` for this section that was used to collect related information.
    -- [req("cli.collect.config")]
    config_hash text references GeneralJson (hash) on delete set null,
    primary key (collect_hash, section_hash)
);

-- Table to store the plain content and the related SHA256 hash.
-- This reduces duplication of unchanged content.
--
-- [req("changes.show", "changes.compact_content")]
create table GeneralContents (
    -- Hash of the content
    hash text not null primary key,
    -- Content that is either plain text or of unknown format to mantra.
    content text not null
);

-- Table to store JSON data and the related SHA256 hash.
-- This reduces duplication of unchanged data.
--
-- TODO: map requirement
create table GeneralJson (
    -- Hash of the data
    hash text not null primary key,
    -- JSON data that may contain user defined information.
    data text not null
);

-- Table contains product IDs that were collected via `mantra collect`.
-- [req("lifecycle.product.id")]
create table Products (
    -- Product ID
    id text not null primary key,
);

-- Table contains product baselines that were collected via `mantra collect`.
-- [req("lifecycle.product.id")]
create table ProductBaselines (
    -- Product ID
    id text not null references Products (id) on delete cascade,
    -- Baseline of a product.
    -- e.g. git branch or commit hash
    base text not null,
    -- Optional version of a product.
    --
    -- **Note:** Version is optional, because it might not change between commits
    -- and is therefore not part of the primary key.
    version text,
    primary key (id, base)
);

-- Table contains product details that were collected via `mantra collect`.
-- [req("report.product_data")]
create table ProductDetails (
    -- Hash of the product details.
    hash text not null primary key,
    -- Name of a product.
    name text not null,
    -- Optional URL to the product's homepage.
    homepage text,
    -- Optional URL to the product's repository.
    repository text,
    -- Optional license of the product.
    license text,
    -- Optional metadata of the product.
    metadata_hash text references GeneralJson (hash) on delete set null
);

-- Table to link between products and collections.
-- [req("lifecycle.product.id")]
create table ProductCollections (
    -- Hash of the product data collected via `mantra collect`.
    section_hash text not null references CollectedSections (hash) on delete cascade,
    -- Product id that maps to the product of the collected data.
    product_id text not null,
    -- Product version of the collected data.
    product_base text not null,
    -- Product details that were collected.
    product_details_hash text references ProductDetails (hash) on delete set null,
    primary key (product_id, product_base, section_hash),
    foreign key (product_id, product_base) references ProductVersions (id, base) on delete cascade
);

-- Table containing all requirement IDs collected by mantra.
-- [req("req.id", "changes.track.reqs.id")]
create table Requirements (
    id text not null,
    product_id text not null references Products (id) on delete cascade,
    primary key (id, product_id)
);

-- Table to link between collections and requirements.
-- [req("lifecycle.product", "changes.track")]
create table RequirementCollections (
    -- Hash of the data collected via `mantra collect`.
    section_hash text not null references CollectedSections (hash) on delete cascade,
    -- The product ID that maps to the content hash in the particular collection.
    product_id text not null,
    -- The requirement ID of a product that maps to the content hash in the particular collection.
    req_id text not null,
    -- The requirement content hash that maps to general information about a requirement.
    req_content_hash text not null references RequirementContents (hash) on delete cascade,
    primary key (section_hash, req_id, product_id)
);

-- Stores the content of requirements.
--
-- **Note:** Decoupled from RequirementCollections, because a contnet change in one requirement
-- should not require to duplicate the content mapping for all other requirements.
-- [req("changes.track.reqs")]
create table RequirementContents (
    hash text not null primary key,
    -- The requirement content hash that maps to general information about a requirement.
    req_details_hash text not null references RequirementDetails (hash) on delete cascade,
    req_properties_hash text not null references RequirementPropertiesHashes (hash) on delete cascade,
    req_hierarchies_hash text not null references RequirementHierarchiesHashes (hash) on delete cascade,
    -- Flag indicating whether the requirement requires manual verification.
    -- `true`: The requirement requires manual verification.
    -- [req("req.manual")]
    manual_verification bool not null,
    -- Flag indicating whether the requirement is deprecated.
    -- `true`: The requirement is deprecated.
    -- [req("req.deprecated")]
    deprecated bool not null,
);

-- Stores general requirements details such as title and description.
--
-- **Note:** Multiple IDs may have the same content.
-- However, this likely indicates a rename of the requirement ID.
-- [req("changes.track.reqs")]
create table RequirementDetails (
    -- The SAH256 hash of the requirement details.
    hash text not null primary key,
    -- The hash of the title of the requirement.
    -- [req("req.title")]
    title_hash text not null references GeneralContents (hash) on delete cascade,
    -- The hash of the origin data of the requirement.
    -- [req("req.origin")]
    origin_hash text not null references GeneralJson (hash) on delete cascade,
    -- Optional hash of the description content of the requirement.
    -- [req("req.description")]
    description_hash text references GeneralContents (hash) on delete set null,
);

-- Table to map to properties of requirements.
-- [req("req.properties")]
create table RequirementPropertiesHashes (
    -- The hash over all properties of a requirement.
    hash text not null primary key,
);

-- Table to map to properties of requirements.
-- [req("req.properties")]
create table RequirementProperties (
    -- The hash of the requirement content.
    req_properties_hash text not null references RequirementPropertiesHashes (hash) on delete cascade,
    -- Hash of a custom property of the requirement.
    property_hash text not null references GeneralJson (hash) on delete cascade,
    primary key (req_properties_hash, property_hash)
);

-- Table to map the requirement hierarchy to a requirement.
-- [req("req.hierarchy", "changes.track.reqs")]
create table RequirementHierarchiesHashes (
    -- The hash of requirement parents block that defines the requirement hierarchy.
    hash text not null primary key,
);

-- Table to represent the requirement hierarchy per requirement content.
--
-- **Note:** Per requirement content, because the parent IDs are part of the content.
-- [req("req.hierarchy", "changes.track.reqs")]
create table RequirementHierarchies (
    -- The hash of the requirement content.
    req_hierarchies_hash text not null references RequirementHierarchiesHashes (hash) on delete cascade,
    -- Product ID the child requirements id defined in.
    child_product_id text not null,
    -- The ID of the child requirement, whose content referenced the parent ID.
    child_req_id text not null,
    -- The product ID the parent requirement is defined in.
    parent_product_id text not null,
    -- The ID of the parent requirement.
    parent_req_id text not null,
    primary key (req_content_hash, child_product_id, child_req_id, parent_product_id, parent_req_id),
    foreign key (child_product_id, child_req_id) references Requirements (product_id, id) on delete cascade,
    foreign key (parent_product_id, parent_id) references Requirements (product_id, id) on delete cascade
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
create table FileHashCollections (
    -- Hash of the collected content.
    section_hash text not null references CollectedSections (hash) on delete cascade,
    -- Filepath of a file content was collected from.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    primary key (section_hash, filepath, file_hash),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade
);

-- Table to store line spans that must be excluded from code coverage analysis.
--
-- TODO: add req trace
create table CoverageBlockExcludes (
    -- Filepath of the file the trace was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- First line that must be excluded from code coverage analysis until the `end_line`.
    start_line integer not null,
    -- Last line that must be excluded (inclusive) from code coverage analysis.
    end_line integer not null,
    -- Hash of the comment explaining why the spanned lines must be excluded from code coverage calculations.
    comment_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (filepath, file_hash, start_line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade,
    constraint start_le_end check (start_line <= end_line)
);

-- Table to store lines that must be excluded from code coverage analysis.
--
-- TODO: add req trace
create table CoverageLineExcludes (
    -- Filepath of the file the trace was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line that must be excluded from code coverage analysis.
    line integer not null,
    -- Hash of the comment explaining why the line must be excluded from code coverage analysis.
    comment text not null references GeneralContents (hash) on delete cascade,
    primary key (filepath, file_hash, line),
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
    -- Product ID that maps the trace and requirement.
    product_id text not null,
    -- Requirement ID that is directly set on the trace.
    req_id text not null,
    -- File the trace to the requirement was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    primary key (req_id, filepath, file_hash, line),
    foreign key (filepath, file_hash, line) references Traces (filepath, file_hash, line) on delete cascade,
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade
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
    primary key (filepath, file_hash, definition_line),
    foreign key (filepath, file_hash) references FileHashes (filepath, hash) on delete cascade,
    constraint start_le_end check (start_line <= end_line),
    constraint def_in_span check (start_line <= definition_line <= end_line)
);

-- Table to link to the content of an element.
-- Note: This table may be left empty if content can be retrieved locally when generating reports.
-- [req("report.coverage.content", "trace.element")]
create table ElementContents (
    -- File the element is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the element is defined at.
    definition_line integer not null,
    -- Hash of the content of the element.
    content_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (filepath, file_hash, definition_line),
    foreign key (filepath, file_hash, definition_line) references Elements (filepath, file_hash, definition_line) on delete cascade
);

-- Table to store language code blocks that are linked to traces.
-- [req("trace.code_block")]
create table TracedCodeBlocks (
    -- File the code block is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace related to the code block is set.
    traced_line integer not null,
    -- Line the code block span starts.
    -- [req("trace.code_block.span")]
    start_line integer not null,
    -- Line the code block span ends.
    -- [req("trace.code_block.span")]
    end_line integer not null,
    -- The code block kind. other=0, if=1, else-if=2, else=3, loop=4, while=5, for=6, match/case=7,
    kind integer not null,
    primary key (filepath, file_hash, traced_line),
    foreign key (filepath, file_hash, traced_line) references Traces (filepath, file_hash, line) on delete cascade,
    constraint start_le_trace_le_end check (start_line <= traced_line <= end_line)
);

-- Table to link to the content of a code block.
-- Note: This table may be left empty if content can be retrieved locally when generating reports.
-- [req("report.coverage.content", "trace.code_block")]
create table CodeBlockContents (
    -- File the code block is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace related to the code block is set.
    traced_line integer not null,
    -- The hash of the content.
    content_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (filepath, file_hash, traced_line),
    foreign key (filepath, file_hash, traced_line) references TracedCodeBlocks (filepath, file_hash, traced_line) on delete cascade
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
--
-- Note: Reference to the selection hash and product ID is needed to get the collection time relation.
--
-- [req("analyze.validate.store_invalid")]
create table UnrelatedDirectReqTraces (
    -- Hash of the collected content.
    section_hash text not null references CollectedSections (hash) on delete cascade,
    -- The product ID that maps to the product that misses the requirement ID.
    product_id text not null references Products(id) on delete cascade,
    -- The requirement ID that was not part of the requirements table at collection time for the product.
    req_id text not null,
    -- File the trace to the requirement was detected in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    primary key (section_hash, product_id, req_id, filepath, file_hash, line),
    foreign key (filepath, file_hash, line) references Traces (filepath, file_hash, line) on delete cascade
);

-- Table to store test runs that executed one or more test cases.
-- [req("testcov.test_run", "changes.track.test_runs")]
create table TestRuns (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null references Products(id) on delete cascade,
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
    -- Hash of the optional metadata of a test run.
    -- [req("testcov.test_run.metadata")]
    metadata_hash text references GeneralContents (hash) on delete set null,
    -- The hash of the origin data of the test run.
    -- [req("testcov.test_run.origin")]
    origin_hash text not null references GeneralContents (hash) on delete cascade,
    -- Hash of the test run content.
    --
    -- **Note:** Needed to detect changes and require a new revision.
    content_hash text not null,
    primary key (product_id, name, utc_date, revision)
);

-- Table to store the mapping between a `mantra collect` invocation
-- and the collected test run.
-- [req("changes.track")]
create table TestRunCollections (
    -- Hash of the collected content.
    section_hash text not null references CollectedSections (hash) on delete cascade,
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- The name of the test run.
    test_run_name text not null,
    -- The UTC date and time at which the test run was executed.
    test_run_utc_date text not null,
    -- Indicates the revision of a test run to track retrospective changes.
    test_run_revision integer not null,
    primary key (section_hash, test_run_name, test_run_utc_date, test_run_revision),
    foreign key (product_id, test_run_name, test_run_utc_date, test_run_revision) references TestRuns (product_id, name, utc_date, revision) on delete cascade
);

-- Table to store test run hierarchies.
-- This allows to have nested test runs,
-- while each test run may additionally have regular test cases.
-- [req("testcov.test_run.nested")]
create table TestRunHierarchies (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
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
        product_id,
        parent_name,
        parent_utc_date,
        parent_revision,
        child_name,
        child_utc_date,
        child_revision
    ),
    foreign key (product_id, parent_name, parent_utc_date, parent_revision) references TestRuns (product_id, name, utc_date, revision) on delete cascade,
    foreign key (product_id, child_name, child_utc_date, child_revision) references TestRuns (product_id, name, utc_date, revision) on delete cascade
);

-- Table to store logs that were captured during test run execution.
--
-- **Note:** Separate table to `TestRuns`, because logs at test run level should be rare,
-- which would lead to a field that is mostly `null`.
--
-- [req("testcov.test_case.metadata")]
create table TestRunLogs (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Hash of the logs captured during the test run execution.
    logs_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (product_id, name, utc_date, revision) on delete cascade
);

-- Table to store statement coverage per test run for files that are mapped to tracked files.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.use_hash"])
create table TestRunTrackedStatementCoverage (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Hash of the file content when the coverage was captured.
    stmnt_file_hash text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    hits integer not null,
    primary key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (product_id, name, utc_date, revision) on delete cascade,
    foreign key (stmnt_filepath, stmnt_file_hash) references FileHashes (file, hash) on delete cascade
);

-- Table to store statement coverage per test run for files that cannot be mapped to tracked files.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.no_hash"])
create table TestRunUntrackedStatementCoverage (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
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
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (product_id, name, utc_date, revision) on delete cascade
);

-- Table to store test case results.
-- [req("testcov.test_case")]
create table TestCases (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
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
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ),
    foreign key (
        produc_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision
    ) references TestRuns (product_id, name, utc_date, revision) on delete cascade
);

-- Table to link to metadata of a test case.
--
-- Note: Metadata in own table, because in contrast to test runs,
-- it is expected that test cases will seldom have metadata.
--
-- [req("testcov.test_case.metadata")]
create table TestCaseMetadata (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Hash of the metadata of the test case.
    data_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        produc_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to link log output to a test case.
--
-- **Note:** Logs separate to metadata table, because test cases likely have no metadata esides logs,
-- so the `data` field would be mostly `null` if logs and metadata are stored in one table.
--
-- [req("testcov.test_case.metadata")]
create table TestCaseLogs (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_utc_date text not null,
    -- Revision of the test run.
    test_run_revision integer not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Logs that were captured during the execution of the test case.
    logs_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store the link between an element and a test case where the test case location
-- can be mapped to an element in a tracked file.
-- [req("testcov.test_case.origin", "changes.track")]
create table TestCaseTrackedLocations (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
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
    -- This links to the definition line of the element.
    line integer not null,
    primary key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade,
    foreign key (
        filepath,
        file_hash,
        line
    ) references Elements (
        filepath,
        file_hash,
        definition_line
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
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
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
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
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
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
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
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store statement coverage per test case.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.use_hash"])
create table TestCaseTrackedStatementCoverage (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
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
    -- Hash of the file that was covered.
    stmnt_file_hash text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during the test case execution.
    hits integer not null,
    primary key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade,
    foreign key (stmnt_filepath, stmnt_file_hash) references FileHashes (
        filepath,
        file_hash
    ) on delete cascade
);

-- Table to store statement coverage per test case for files that cannot be mapped to collected files.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.no_hash])
create table TestCaseUntrackedStatementCoverage (
    -- Product ID that maps to the product that is tested.
    product_id text not null,
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
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    ) on delete cascade
);

-- Table to store reviews.
-- [req("review", "changes.track")]
create table Reviews (
    -- The product ID that maps to the product that got reviewed.
    product_id text not null references Products(id) on delete cascade,
    -- Name of the review
    name text not null,
    -- UTC date and time at which the review was held.
    utc_date text not null,
    -- Indicates the revision of a review to track retrospective changes.
    revision integer not null,
    -- The hash of the review content to detect changes.
    content_hash text not null,
    -- The reviewers of the review.
    -- [req("review.reviewer")]
    reviewer text not null,
    -- The hash of the origin data of the review.
    -- [req("review.origin")]
    origin_hash text not null references GeneralContents (hash) on delete cascade,
    -- Hash of the optional decription for the review.
    -- [req("review.description")]
    description_hash text references GeneralContents (hash) on delete set null,
    primary key (product_id, name, utc_date, revision)
);

-- Table to map review to `mantra collect` runs.
-- [req("changes.track")]
create table ReviewCollections (
    -- Hash of the collected content.
    section_hash text not null references CollectedSections (hash) on delete cascade,
    -- The product ID that maps to the product that got reviewed.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    primary key (section_hash, review_name, review_utc_date, review_revision),
    foreign key (product_id, review_name, review_utc_date, review_revision) references Reviews (product_id, name, utc_date, revision) on delete cascade
);

-- Table to store requirement IDs that were manually verified in a review,
-- and the IDs could be mapped to requirements stored in the database.
-- [req("review.verify_req")]
create table ManuallyVerifiedRequirements (
    -- ID of the requirement that is manually verified.
    req_id text not null references Requirements (id) on delete cascade,
    -- Product ID that maps to the product that got reviewed.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- Hash of the comment for the manual verification.
    comment_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        product_id,
        req_id,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (product_id, review_name, review_utc_date, review_revision) references Reviews (product_id, name, utc_date, revision) on delete cascade
);

-- Table to store requirement IDs that were manually verified in a review,
-- but the IDs could not be mapped to requirements stored in the database.
-- [req("review.verify_req", "analyze.validate.store_invalid")]
create table UnrelatedManuallyVerifiedRequirements (
    -- ID of the requirement that is manually verified.
    req_id text not null,
    -- The product ID that maps to the product that got reviewed.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_utc_date text not null,
    -- Revision of the review.
    review_revision integer not null,
    -- Hash of the comment for the manual verification.
    comment_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        product_id,
        req_id,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (product_id, review_name, review_utc_date, review_revision) references Reviews (product_id, name, utc_date, revision) on delete cascade
);

-- Table to store test case overrides from reviews.
-- [req("review.test_case_state")]
create table TestCaseOverrides (
    -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
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
    -- Hash of the comment explaining why the state must be overriden.
    comment_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (product_id, review_name, review_utc_date, review_revision) references Reviews (product_id, name, utc_date, revision) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        name
    )
);

-- Table to store overrides from reviews for statement coverage entries of test runs.
--
-- **Note:** No file hash needed, because the related coverage entry is either in the tracked or untracked table.
--
-- [req("review.coverage", "testcov.cov.lines")]
create table TestRunStatementCoverageOverrides (
    -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
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
    -- Hash of the comment explaining why this statement coverage must be overriden.
    comment_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        review_name,
        review_utc_date,
        review_revision,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (product_id, review_name, review_utc_date, review_revision) references Reviews (product_id, name, utc_date, revision) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    ) references TestRunStatementCoverage (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        stmnt_filepath,
        stmnt_line
    )
);

-- Table to store overrides from reviews for statement coverage entries of test cases.
--
-- **Note:** No file hash needed, because the related coverage entry is either in the tracked or untracked table.
--
-- [req("review.coverage", "testcov.cov.lines")]
create table TestCaseStatementCoverageOverrides (
    -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
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
    -- Hash of the comment explaining why this statement coverage must be overriden.
    comment_hash text not null references GeneralContents (hash) on delete cascade,
    primary key (
        product_id,
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
    foreign key (product_id, review_name, review_utc_date, review_revision) references Reviews (product_id, name, utc_date, revision) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ) references TestCaseStatementCoverage (
        product_id,
        test_run_name,
        test_run_utc_date,
        test_run_revision,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    )
);
