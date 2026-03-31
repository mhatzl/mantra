

-- Base table for test runs.
-- [req("testcov.test_run")]
create table TestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null references Products (id) on delete cascade,
    -- The name of the test run.
    name text not null,
    -- The UTC date and time at which the test run was executed.
    utc_date text not null,
    -- Optional description hash
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- Optional duration about how long the test run took.
    duration Text,
    -- The number of expected test cases mapped to the test run.
    -- Meaning, if there are fewer associated test cases in the `TestCases` table,
    -- not all test cases were executed.
    nr_of_test_cases integer not null,
    -- Optional origin data of the test run that was set for multiple test runs.
    -- [req("testcov.test_run.origin")]
    base_origin_hash text references GeneralJson (hash) on delete restrict,
    -- Optional hash of the origin data of the test run.
    -- [req("testcov.test_run.origin")]
    origin_hash text references GeneralJson (hash) on delete restrict,
    -- Hash of the data the test run was collected from.
    data_hash text not null,
    primary key (product_id, name, utc_date)
);

-- Table to store filepaths from which test run data was collected.
-- Due to test runs potentially  being created from multiple well-known formats
-- such as JUnit and Cobertura, multiple filepaths may be set per test run.
--
-- Note: Test runs created internally to map covered files to test runs do not have source filepaths.
create table TestRunDataFilepaths (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    -- Filepath the data was collected from
    filepath text not null,
    primary key (product_id, test_run_name, test_run_date, filepath),
    -- Note: may be inserted while collecting well-known data before test run is inserted => defer foreign key check
    foreign key (product_id, test_run_name, test_run_date) references TestRuns (product_id, name, utc_date) on delete cascade deferrable initially deferred,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade
);

-- Table to store optional metadata of a test run.
-- [req("testcov.test_run.metadata")]
create table TestRunProperties (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    property_key text not null,
    value_hash text references GeneralJson (hash) on delete restrict,
    primary key (product_id, test_run_name, test_run_date, property_key),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns (product_id, name, utc_date) on delete cascade
);

create table TestRunRevisions (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    -- Indicates the revision
    revision integer not null,
    -- Comment for the revision.
    -- [req("changes.comment")]
    comment text not null,
    primary key (product_id, test_run_name, test_run_date, revision),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns (product_id, name, utc_date) on delete cascade
);

-- Names of authors of a test run revision.
-- [req("changes.authors")]
create table TestRunRevisionAuthors (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    -- Indicates the revision
    revision integer not null,
    -- Names of an author of the revision.
    -- [req("changes.authors")]
    author text not null,
    primary key (product_id, test_run_name, test_run_date, revision, author),
    foreign key (product_id, test_run_name, test_run_date, revision) references TestRunRevisions (product_id, test_run_name, test_run_date, revision) on delete cascade
);

-- Table to store test run hierarchies.
-- This allows to have nested test runs,
-- while each test run may additionally have regular test cases.
--
-- **Note:** Both test runs must refer to the same product ID,
-- because test results should not span across products.
-- [req("testcov.test_run.nested")]
create table TestRunHierarchies (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    -- The product ID of both test runs.
    product_id text not null,
    -- The name of the parent test run.
    parent_name text not null,
    -- The UTC date and time of the parent test run.
    parent_date text not null,
    -- The name of the child test run.
    child_name text not null,
    -- The UTC date and time of the child test run.
    child_date text not null,
    primary key (
        product_id,
        parent_name,
        parent_date,
        child_name,
        child_date
    ),
    foreign key (product_id, parent_name, parent_date) references TestRuns (product_id, name, utc_date) on delete cascade deferrable initially deferred,
    foreign key (product_id, child_name, child_date) references TestRuns (product_id, name, utc_date) on delete cascade deferrable initially deferred
);

-- Table to store logs that were captured during test run execution.
--
-- **Note:** Separate table to `TestRuns`, because logs at test run level should be rare,
-- which would lead to a field that is mostly `null`.
--
-- [req("testcov.test_case.metadata")]
create table TestRunLogs (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- Date and time of the test run.
    test_run_date text not null,
    -- stdout = 0, stderr = 1
    log_src integer not null,
    -- Hash of the log content captured during the test run execution.
    log_hash text not null references GeneralTexts (hash) on delete restrict,
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

-- Table to store line coverage per test run.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.use_hash"])
create table TestRunLineCoverage (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- File that was covered.
    cov_filepath text not null,
    -- Optional hash of the file content when the coverage was captured.
    cov_file_hash text,
    -- Line that was covered.
    cov_line text not null,
    -- Number of how often the line was covered/hit during test run execution.
    -- If null, the line is ignored from line coverage analysis.
    -- Unless it is not null for test cases or child test runs of this test run.
    hits integer,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        cov_filepath,
        cov_line
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
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
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
    -- Optional description hash
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- Optional utc date and time for the test case.
    utc_date text,
    -- Optional duration of the test case.
    duration text,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        name
    ),
    foreign key (
        product_id,
        test_run_name,
        test_run_date
    ) references TestRuns (product_id, name, utc_date) on delete cascade
);

-- Stores requirements that are explicitely verified by the test case.
create table TestCaseVerifiedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    req_id text not null,
    primary key (product_id, test_run_name, test_run_date, test_case_name, req_id),
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade,
    foreign key (product_id, test_run_name, test_run_date, test_case_name) references TestCases (product_id, test_run_name, test_run_date, name) on delete cascade
);

-- Table to store optional metadata of a test case.
-- [req("testcov.test_case.metadata")]
create table TestCaseProperties (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    property_key text not null,
    value_hash text references GeneralJson (hash) on delete restrict,
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
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
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
    -- Hash of the log content that was captured during the execution of the test case.
    log_hash text not null references GeneralTexts (hash) on delete cascade,
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
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
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
        test_case_name,
        filepath
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

-- Table to store additional properties for the state of a test case.
-- This is mostly needed for *skipped* or *failed* test cases.
-- [req("testcov.test_case.state")]
create table TestCaseStateProperties (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
   -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- The key of the additional property for the state of a test case.
    property_key text not null,
    value_hash text references GeneralJson (hash) on delete restrict,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        property_key
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

-- Table to store line coverage per test case.
-- [req("testcov.cov.lines", "testcov.cov.trace_mapping.use_hash"])
create table TestCaseLineCoverage (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
   -- The product ID that maps to the product that got tested with this test run.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- File that was covered.
    cov_filepath text not null,
    -- Hash of the file that was covered.
    cov_file_hash text,
    -- Line that was covered.
    cov_line text not null,
    -- Number of how often the line was covered/hit during the test case execution.
    -- If null, the line is ignored from line coverage analysis for this test case.
    hits integer,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        cov_filepath,
        cov_line
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
