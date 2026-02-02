-- Table to store plain text and the related hash.
-- This reduces duplication of unchanged content.
--
-- [req("changes.show", "changes.compact_content")]
create table GeneralTexts (
    -- Hash of the content
    hash text not null primary key,
    -- Content that is either plain text or of unknown format to mantra.
    content text not null
);

-- Table to store JSON content and the related hash.
-- This reduces duplication of unchanged content.
--
-- TODO: map requirement
create table GeneralJson (
    -- Hash of the content
    hash text not null primary key,
    -- JSON content that may contain user defined information.
    content text not null
);

-- Table to store hashes of file contents from which data was collected.
-- [req("changes.track.traces.files")]
create table FileHashes (
    -- Hash of the file content.
    hash text not null
);

-- Base table used to track changes over multiple `mantra collect` runs.
-- [req("lifecycle.product", "changes.track")]
create table Collections (
    nr integer primary key autoincrement,
    run_at_utc text not null,
    -- Filepath to the `mantra.toml` file that was used to collect the data.
    -- Path is relativ to the invocation of `mantra collect`.
    -- [req("changes.track.origin")]
    collect_filepath text not null,
    -- The hash of the configuration content in `mantra.toml` for this collection.
    -- [req("cli.collect.config")]
    config_hash text not null references GeneralJson (hash) on delete restrict,
    -- Optional hash of the arguments set when calling `mantra collect`.
    arguments_hash text references GeneralJson (hash) on delete restrict,
    -- Optional hash of the environmental variables set that are relevant for mantra
    -- when calling `mantra collect`.
    env_vars_hash text references GeneralJson (hash) on delete restrict
);

-- Table contains products that were collected via `mantra collect`.
-- [req("lifecycle.product.id", "report.product_data")]
create table Products (
    last_collect_nr integer not null references Collections (nr) on delete restrict,
    -- Product ID
    id text not null primary key,
    -- Name of a product.
    name text not null,
    -- Baseline of a product.
    -- e.g. git branch or commit hash
    base text not null,
    -- Optional version of a product.
    --
    -- **Note:** Version is optional, because it might not change between commits
    -- and is therefore not part of the primary key.
    version text,
    -- Optional URL to the product's homepage.
    homepage text,
    -- Optional URL to the product's repository.
    repository text,
    -- Optional license of the product.
    license text,
    -- Optional metadata of the product.
    metadata_hash text references GeneralJson (hash) on delete restrict
);

create table ProductsHistory (
    nr integer primary key,
    product_id text not null references Products (id) on delete cascade,
    collect_nr text not null references Collections (nr) on delete restrict,
    operation text not null check (operation in ('insert', 'update', 'delete')),
    name text,
    base text,
    version text,
    homepage text,
    repository text,
    license text,
    metadata_hash text references GeneralJson (hash) on delete restrict
);

create trigger ProductsUpdates
after update on Products
for each row
when (
    old.name is distinct from new.name or
    old.base is distinct from new.base or
    old.version is distinct from new.version or
    old.homepage is distinct from new.homepage or
    old.repository is distinct from new.repository or
    old.license is distinct from new.license or
    old.metadata_hash is distinct from new.metadata_hash
)
begin
    insert into ProductsHistory (
        product_id,
        collect_nr,
        operation,
        name,
        base,
        version,
        homepage,
        repository,
        license,
        metadata_hash
    )
    values (
        old.id,
        (select max(nr) from Collections),
        'update',
        case when old.name is distinct from new.name then old.name else null end,
        case when old.base is distinct from new.base then old.base else null end,
        case when old.version is distinct from new.version then old.version else null end,
        case when old.homepage is distinct from new.homepage then old.homepage else null end,
        case when old.repository is distinct from new.repository then old.repository else null end,
        case when old.license is distinct from new.license then old.license else null end,
        case when old.metadata_hash is distinct from new.metadata_hash then old.metadata_hash else null end
    );
end;

create trigger ProductsInsertions
after insert on Products
for each row
begin
    insert into ProductsHistory (
        product_id,
        collect_nr,
        operation,
        name,
        base,
        version,
        homepage,
        repository,
        license,
        metadata_hash
    )
    values (
        new.id,
        (select max(nr) from Collections),
        'insert',
        new.name,
        new.base,
        new.version,
        new.homepage,
        new.repository,
        new.license,
        new.metadata_hash
    );
end;

create trigger ProductsDeletions
after delete on Products
for each row
begin
    insert into ProductsHistory (
        product_id,
        collect_nr,
        operation,
        name,
        base,
        version,
        homepage,
        repository,
        license,
        metadata_hash
    )
    values (
        old.id,
        (select max(nr) from Collections),
        'delete',
        old.name,
        old.base,
        old.version,
        old.homepage,
        old.repository,
        old.license,
        old.metadata_hash
    );
end;

create table ProductRelatedFiles (
    -- Product ID
    product_id text not null,
    filepath text not null,
    file_hash text not null references FileHashes (hash) on delete restrict,
    constraint ProductRelatedFiles primary key (product_id, filepath)
);

-- Table containing all requirement IDs collected by mantra.
-- [req("req.id", "changes.track.reqs.id")]
create table Requirements (
    id text not null,
    product_id text not null references Products (id) on delete cascade,
    -- Flag indicating whether the requirement requires manual verification.
    -- `true`: The requirement requires manual verification.
    -- [req("req.manual")]
    manual_verification bool not null,
    -- Flag indicating whether the requirement is deprecated.
    -- `true`: The requirement is deprecated.
    -- [req("req.deprecated")]
    deprecated bool not null
    -- The title of the requirement.
    -- [req("req.title")]
    title text not null,
    -- The origin data of the requirement.
    -- [req("req.origin")]
    origin_hash text not null references GeneralJson (hash) on delete restrict,
    -- Optional description content of the requirement.
    -- [req("req.description")]
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- Hash of the source the requirement was defined in.
    -- e.g. Markdown or JSON file
    src_hash text not null references FileHashes (hash) on delete restrict,
    constraint RequirementsPk primary key (id, product_id)
);

-- Table to map to properties of requirements.
-- [req("req.properties")]
create table RequirementProperties (
    req_id text not null,
    product_id text not null,
    -- Key of the property
    property_key text not null,
    -- Hash of a custom property of the requirement.
    value_hash text not null references GeneralJson (hash) on delete restrict,
    constraint RequirementPropertiesPk primary key (req_id, product_id, property_key),
    foreign key (req_id, product_id) references Requirements (id, product_id) on delete cascade
);

-- Table to represent the requirement hierarchy per requirement content.
--
-- **Note:** Per requirement content, because the parent IDs are part of the content.
-- [req("req.hierarchy")]
create table RequirementHierarchies (
    -- Product ID the child requirements id defined in.
    child_product_id text not null,
    -- The ID of the child requirement, whose content referenced the parent ID.
    child_req_id text not null,
    -- The product ID the parent requirement is defined in.
    parent_product_id text not null,
    -- The ID of the parent requirement.
    parent_req_id text not null,
    constraint RequirementHierarchiesPk primary key (child_product_id, child_req_id, parent_product_id, parent_req_id),
    foreign key (child_product_id, child_req_id) references Requirements (product_id, id) on delete cascade,
    foreign key (parent_product_id, parent_req_id) references Requirements (product_id, id) on delete cascade
);

-- Table to store all traces.
-- [req("trace.origin", "changes.track")]
create table Traces (
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at in the file.
    line integer not null,
    -- Trace kind (0 = clarifies, 1 = satisfies, 2 = verifies, 3 = links).
    -- [req("trace.kind")]
    kind integer not null,
    primary key (file_hash, line),
    foreign key (file_hash) references FileHashes (hash) on delete restrict
);

-- Table to store custom properties of traces.
-- [req("trace.properties")]
create table CustomTraceProperties (
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    -- Custom property of the trace. e.g. "critical"
    property text not null,
    primary key (file_hash, line, property),
    foreign key (file_hash, line) references Traces (file_hash, line) on delete cascade
);

-- Table to store relations between traces and requirements.
-- [req("trace.id", "trace.mult_reqs")]
create table DirectReqTraces (
    -- Product ID that maps the trace and requirement.
    product_id text not null,
    -- Requirement ID that is directly set on the trace.
    req_id text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    primary key (product_id, req_id, file_hash, line),
    foreign key (file_hash, line) references Traces (file_hash, line) on delete cascade,
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade
);

-- Table to store language elements such as functions, tests, structs, enums, classes, ...
--
-- Note: Elements are uniquely identifiable by filepath and line number.
-- Due to feature flags or language semantics, idents may be declared multiple times, and are therefore not unique.
-- [req("trace.element")]
create table Elements (
    -- Name of the element.
    --
    -- **Note:** The fully qualified identifier is stored in ElementIdents.
    name text not null,
    -- Hash of the file content.
    file_hash text not null references FileHashes (hash) on delete restrict,
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
    -- Optional hash of the content of the element.
    content_hash text references GeneralTexts (hash) on delete restrict,
    primary key (file_hash, definition_line),
    constraint start_le_end check (start_line <= end_line),
    constraint def_in_span check (start_line <= definition_line <= end_line)
);

create table ElementIdents (
    product_id text not null,
    -- File the element is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the element is defined at.
    definition_line integer not null,
    ident text not null,
    primary key (product_id, filepath, file_hash, definition_line),
    foreign key (product_id, filepath, file_hash) references ProductRelatedFiles (product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, definition_line) references Elements (file_hash, definition_line) on delete cascade
);

-- Table to store language code blocks that are linked to traces.
-- [req("trace.code_block")]
create table TracedCodeBlocks (
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
    -- Optional hash of the code block.
    content_hash text references GeneralTexts (hash) on delete restrict,
    primary key (file_hash, traced_line),
    foreign key (file_hash, traced_line) references Traces (file_hash, line) on delete cascade,
    constraint start_le_trace_le_end check (start_line <= traced_line <= end_line)
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
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace related to the element was detected at.
    traced_line integer not null,
    -- Line the element is defined at.
    element_definition_line integer not null,
    primary key (
        file_hash,
        traced_line,
        element_definition_line
    ),
    foreign key (file_hash, element_definition_line) references Elements (file_hash, definition_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces (file_hash, line) on delete cascade
);


-- Table to store traces to requirements that were not part of the database
-- when the trace was added via `mantra collect`.
--
-- Note: Reference to the selection hash and product ID is needed to get the collection time relation.
--
-- [req("analyze.validate.store_invalid")]
create table UnrelatedDirectReqTraces (
    -- The product ID that maps to the product that misses the requirement ID.
    product_id text not null references Products(id) on delete cascade,
    -- The requirement ID that was not part of the requirements table at collection time for the product.
    req_id text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    primary key (product_id, req_id, file_hash, line),
    foreign key (file_hash, line) references Traces (file_hash, line) on delete cascade
);

-- Table to store line spans that must be excluded from code coverage analysis.
--
-- TODO: add req trace
create table CoverageBlockExcludes (
    -- Hash of the file content.
    file_hash text not null references FileHashes (hash) on delete restrict,
    -- First line that must be excluded from code coverage analysis until the `end_line`.
    start_line integer not null,
    -- Last line that must be excluded (inclusive) from code coverage analysis.
    end_line integer not null,
    -- Hash of the comment explaining why the spanned lines must be excluded from code coverage calculations.
    comment_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (file_hash, start_line),
    constraint start_le_end check (start_line <= end_line)
);

-- Table to store lines that must be excluded from code coverage analysis.
--
-- TODO: add req trace
create table CoverageLineExcludes (
    -- Hash of the file content.
    file_hash text not null references FileHashes (hash) on delete restrict,
    -- Line that must be excluded from code coverage analysis.
    line integer not null,
    -- Hash of the comment explaining why the line must be excluded from code coverage analysis.
    comment text not null references GeneralTexts (hash) on delete cascade,
    primary key (file_hash, line),
);

-- Base table for test runs.
-- [req("testcov.test_run")]
create table TestRuns (
    product_id text not null references Products (id) on delete cascade,
    -- The name of the test run.
    name text not null,
    -- The UTC date and time at which the test run was executed.
    utc_date text not null,
    -- Optional duration about how long the test run took.
    duration integer,
    -- The number of expected test cases mapped to the test run.
    -- Meaning, if there are fewer associated test cases in the `TestCases` table,
    -- not all test cases were executed.
    nr_of_test_cases integer not null,
    -- The hash of the origin data of the test run.
    -- [req("testcov.test_run.origin")]
    origin_hash text not null references GeneralJson (hash) on delete restrict,
    -- Hash of the source the test run data was collected from.
    -- e.g. JSON file
    src_hash text not null references FileHashes (hash) on delete restrict,
    primary key (product_id, name, utc_date)
);

-- Table to store optional metadata of a test run.
-- [req("testcov.test_run.metadata")]
create table TestRunProperties (
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    property_key text not null,
    property_value text references GeneralJson (hash) on delete restrict,
    primary key (product_id, test_run_name, test_run_date, property_key),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns (product_id, name, utc_date) on delete cascade
);

create table TestRunRevisions (
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    -- Indicates the revision
    revision integer not null,
    -- Optional names of authors of the revision.
    -- Mandatory for later revisions.
    -- [req("changes.authors")]
    authors text,
    -- Optional comment for the revision.
    -- Mandatory for later revisions.
    -- [req("changes.comment")]
    comment text,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns (product_id, name, utc_date) on delete cascade,
    constraint revision_check
        check (
          (revision = 0) or
          (authors is not null and comment is not null)
        )
);

-- Table to store test run hierarchies.
-- This allows to have nested test runs,
-- while each test run may additionally have regular test cases.
-- [req("testcov.test_run.nested")]
create table TestRunHierarchies (
    -- The product ID of the parent test run.
    parent_product_id text not null,
    -- The name of the parent test run.
    parent_name text not null,
    -- The UTC date and time of the parent test run.
    parent_date text not null,
    -- The product ID of the child test run.
    child_product_id text not null,
    -- The name of the child test run.
    child_name text not null,
    -- The UTC date and time of the child test run.
    child_date text not null,
    primary key (
        parent_product_id,
        parent_name,
        parent_date,
        child_product_id,
        child_name,
        child_date
    ),
    foreign key (parent_product_id, parent_name, parent_date) references TestRuns (product_id, name, utc_date) on delete cascade,
    foreign key (child_product_id, child_name, child_date) references TestRuns (product_id, name, utc_date) on delete cascade
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
    -- Date and time of the test run.
    test_run_date text not null,
    -- stdout = 0, stderr = 1
    log_src integer not null,
    -- Hash of the logs captured during the test run execution.
    logs_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        log_src
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date
    ) references TestRuns (product_id, name, utc_date) on delete cascade
);

-- Table to store statement coverage per test run.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.use_hash"])
create table TestRunStatementCoverage (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Optional hash of the file content when the coverage was captured.
    stmnt_file_hash text,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    hits integer not null,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date
    ) references TestRuns (product_id, name, utc_date) on delete cascade
);

-- Table to store test case results.
-- [req("testcov.test_case")]
create table TestCases (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    name text not null,
    -- State of the test case.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed
    -- [req("testcov.test_case.state")]
    state integer not null,
    -- Optional utc date and time for the test case.
    utc_date text,
    -- Optional duration of the test case.
    duration text,
    primary key (
        product_id,
        test_run_name,
        test_run_date
        name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date
    ) references TestRuns (product_id, name, utc_date) on delete cascade
);

-- Table to store optional metadata of a test case.
-- [req("testcov.test_case.metadata")]
create table TestCaseProperties (
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    property_key text not null,
    property_value text references GeneralJson (hash) on delete restrict,
    primary key (product_id, test_run_name, test_run_date, test_case_name, property_key),
    foreign key (product_id, test_run_name, test_run_date, test_case_name) references TestCases (product_id, test_run_name, test_run_date, name) on delete cascade
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
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- stdout = 0, stderr = 1
    log_src integer not null,
    -- Logs that were captured during the execution of the test case.
    logs_hash text not null references GeneralTexts (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        log_src
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_date,
        name
    ) on delete cascade
);

-- Table to store the optional location of test cases.
-- [req("testcov.test_case.origin")]
create table TestCaseLocations (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- File the test case is defined in.
    filepath text not null,
    -- Optional hash of the file content.
    file_hash text,
    -- Line the test case is defined at.
    line integer not null,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_date,
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
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- The reason for the state of a test case.
    reason text not null,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_date,
        name
    ) on delete cascade
);

-- Table to store statement coverage per test case.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.use_hash"])
create table TestCaseStatementCoverage (
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Hash of the file that was covered.
    stmnt_file_hash text,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during the test case execution.
    hits integer not null,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_date,
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
    -- The reviewers of the review.
    -- [req("review.reviewer")]
    reviewer text not null,
    -- The hash of the origin data of the review.
    -- [req("review.origin")]
    origin_hash text not null references GeneralTexts (hash) on delete restrict,
    -- Hash of the optional decription for the review.
    -- [req("review.description")]
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- The hash of the file content the review was collected from to detect changes.
    src_hash text not null references FileHashes (hash) on delete restrict,
    primary key (product_id, name, utc_date, revision),
    constraint revision_check
        check (
          (revision = 0) or
          (revision_author is not null and revision_comment is not null)
        )
);

create table ReviewRevisions (
    product_id text not null,
    review_name text not null,
    review_date text not null,
    -- Indicates the revision
    revision integer not null,
    -- Optional names of authors of the revision.
    -- Mandatory for later revisions.
    -- [req("changes.authors")]
    authors text,
    -- Optional comment for the revision.
    -- Mandatory for later revisions.
    -- [req("changes.comment")]
    comment text,
    primary key (product_id, review_name, review_date),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    constraint revision_check
        check (
          (revision = 0) or
          (authors is not null and comment is not null)
        )
);


-- Table to store requirement IDs that were manually verified in a review,
-- and the IDs could be mapped to requirements stored in the database.
-- [req("review.verify_req")]
create table ManuallyVerifiedRequirements (
    -- ID of the requirement that is manually verified.
    req_id text not null,
    -- Product ID that maps to the product that got reviewed.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- Hash of the comment for the manual verification.
    comment_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (
        product_id,
        req_id,
        review_name,
        review_date
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade
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
    review_date text not null,
    -- Hash of the comment for the manual verification.
    comment_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (
        product_id,
        req_id,
        review_name,
        review_utc_date,
        review_revision
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade
);

-- Table to store test case overrides from reviews.
-- [req("review.test_case_state")]
create table TestCaseOverrides (
    -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- State that must be used instead of the one stored in the TestCase table.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed
    state integer not null,
    -- Hash of the comment explaining why the state must be overriden.
    comment_hash text not null references GeneralTexts(hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        review_name,
        review_date
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_date,
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
    test_run_date text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    hits integer not null,
    -- Hash of the comment explaining why this statement coverage must be overriden.
    comment_hash text not null references GeneralTexts (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        review_name,
        review_date,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        stmnt_filepath,
        stmnt_line
    ) references TestRunStatementCoverage (
        product_id,
        test_run_name,
        test_run_date,
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
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- File that was covered.
    stmnt_filepath text not null,
    -- Line that was covered.
    stmnt_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    hits integer not null,
    -- Hash of the comment explaining why this statement coverage must be overriden.
    comment_hash text not null references GeneralTexts (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        review_name,
        review_date,
        stmnt_filepath,
        stmnt_line
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    ) references TestCaseStatementCoverage (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        stmnt_filepath,
        stmnt_line
    )
);
