-- Contains covered lines mapped to traces that are only covered by passed test runs
create table TraceMappedLinesOnlyCoveredByPassedTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line text not null,
    primary key (product_id, test_run_name, test_run_date, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are only covered by passed test cases
create table TraceMappedLinesOnlyCoveredByPassedTestCases (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line text not null,
    primary key (product_id, test_run_name, test_run_date, test_case_name, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesOnlyCoveredByPassedTestRuns
-- and TraceMappedLinesOnlyCoveredByPassedTestCases
create table TraceMappedLinesOnlyCoveredByPassedTests (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line text not null,
    primary key (product_id, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have no mapped line that was covered by a failed test.
create table TracesOnlyCoveredByPassedTests (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, file_hash, traced_line),
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are covered by failed test runs
-- Note: may also be covered by passed test runs, but at least one failed test run
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line text not null,
    primary key (product_id, test_run_name, test_run_date, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are covered by failed test cases
-- Note: may also be covered by passed test cases, but at least one failed test case
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestCases (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line text not null,
    primary key (product_id, test_run_name, test_run_date, test_case_name, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesCoveredByFailedTestRuns
-- and TraceMappedLinesCoveredByFailedTestCases
create table TraceMappedLinesCoveredByFailedTests (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line text not null,
    primary key (product_id, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have at least one linked line that was covered by a failed test.
create table TracesCoveredByFailedTests (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, file_hash, traced_line),
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces covered by test runs.
create view TracesCoveredByTestRuns as
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    filepath,
    file_hash,
    traced_line
from TraceMappedLinesOnlyCoveredByPassedTestRuns
union all
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    filepath,
    file_hash,
    traced_line
from TraceMappedLinesCoveredByFailedTestRuns;

-- Contains traces covered by test cases.
create view TracesCoveredByTestCases as
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name,
    filepath,
    file_hash,
    traced_line
from TraceMappedLinesOnlyCoveredByPassedTestCases
union all
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name,
    filepath,
    file_hash,
    traced_line
from TraceMappedLinesCoveredByFailedTestCases;

-- Contains traces covered by tests.
--
-- Note: Since tests with coverage data can either pass or fail,
-- this view shows all covered traces by combining traces from
-- tables TracesOnlyCoveredByPassedTests and TracesCoveredByFailedTests
create view TracesCoveredByTests as
select
    last_collect_nr,
    product_id,
    filepath,
    file_hash,
    traced_line
from TracesOnlyCoveredByPassedTests
union all -- tables are disjoint so no need for deduplication
select
    last_collect_nr,
    product_id,
    filepath,
    file_hash,
    traced_line
from TracesCoveredByFailedTests;

-- Contains direct verification states of requirements based on the following conditions:
-- - verified: all of the following conditions must be met
--   - requirement has satisfies or verifies traces, or is explicitly verified by at least one test case
--     - if no *statisfies* trace exists for the requirement,
--       and a direct *verifies* trace mentions the ID and the trace is covered by at least one line
--       from coverage metrics of a test run or test case, and all test runs or test cases
--       that cover the line passed
--     - if a *satisfies* trace exists, in addition to the conditions above,
--       at least one *satisfies* trace must also be covered by the same test run or test case
--       that the *verifies* trace is covered by
--     - all test cases that verify the requirement must pass
--     - if no *verifies* trace for the requirement exists, but *satisfies* traces exist:
--       all *satisfies* traces must be covered, and all test runs or test cases that cover a *satisfies* trace must pass
--   - if it is not part of the ManualRequirements table:
--     - requirement must have at least either a satisfies or verifies trace
--       or be explicitly verified by a test case
--     - may be verified by review (but does not affect the state)
--   - if it is part of the ManualRequirements table
--     - must be verified by at least one review
--     - if the requirement has a satisfies or verifies trace
--       or is explicitly verified by a test case,
--       then those must also fulfill the conditions above
--
-- - failed:
--   if at least one of the test runs or test cases failed that would verifiy the requirement
--
-- - skipped:
--   a requirement verificiation is `skipped`, if there are no satisfies or verifies traces,
--   the requirement is not part of the ManualRequirements table,
--   it is explicitly verified by at least one test case, and all such test cases have state `skipped`.
--
-- - unverified: none of the conditions for verified or skipped applied
--   e.g. no satisfies or verifies traces exist, no review for ManualRequirements,
--   and no explicit verification by a test case
--   also possible: verifies trace exists, but tests do not also cover existing satisfies
--
-- **Note:** Direct means that the state is indipendent of the state of related requirements.
create table DirectRequirementVerificationStates (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    -- 0=failed; 1=verified; 2=skipped; 3=unverified
    state integer not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains verification states for requirements based on the requirement hierarchy.
-- This table only contains non-leaf requirements (requirements that have at least one child).
-- States:
-- - verified: all non-optional descendants are verified
-- - failed: at least one descendant failed (or is of unknown state)
--      also including optional descendants
-- - skipped: at least one non-optional descendant was skipped, and none failed or are unverified
-- - unverified: at least one non-optional descendant was unverified, but none failed or were skipped
create table IndirectRequirementVerificationStates (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    state integer not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

create table RequirementVerificationStates (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    state integer not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are successfully verified.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is verified.
-- For non-leaf requirements:
-- - The IndirectRequirementVerificationStates must **not** be failed
-- - If an entry in DirectRequirementVerificationStates exists it must be verified
-- - If no entry in DirectRequirementVerificationStates exists the indirect state must be verified
create table VerifiedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are skipped.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is skipped.
-- For non-leaf requirements, either
-- - IndirectRequirementVerificationStates = skipped
--   DirectRequirementVerificationStates = verified or skipped or unverified
-- - IndirectRequirementVerificationStates = verified or skipped
--   DirectRequirementVerificationStates = skipped
create table SkippedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are skipped.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is failed.
-- For non-leaf requirements, either IndirectRequirementVerificationStates = failed
-- or DirectRequirementVerificationStates = failed
create table FailedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are unverified.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is unverified.
-- For non-leaf requirements, either
-- - IndirectRequirementVerificationStates = unverified
--   DirectRequirementVerificationStates = unverified
-- - IndirectRequirementVerificationStates = skipped, or unverified
--   DirectRequirementVerificationStates = unverified
create table UnverifiedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);
