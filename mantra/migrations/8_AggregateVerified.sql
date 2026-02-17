-- Contains statement lines mapped to traces that are only covered by passed test runs
create table TracesOnlyCoveredByPassedTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    stmnt_line text not null,
    primary key (product_id, test_run_name, test_run_date, filepath, file_hash, traced_line, stmnt_line),
    foreign key (product_id, test_run_name, test_run_date, filepath, stmnt_line)
        references TestRunStatementCoverage(product_id, test_run_name, test_run_date, stmnt_filepath, stmnt_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains statement lines mapped to traces that are only covered by passed test cases
create table TracesOnlyCoveredByPassedTestCases (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    stmnt_line text not null,
    primary key (product_id, test_run_name, test_run_date, test_case_name, filepath, file_hash, traced_line, stmnt_line),
    foreign key (product_id, test_run_name, test_run_date, test_case_name, filepath, stmnt_line)
        references TestCaseStatementCoverage(product_id, test_run_name, test_run_date, test_case_name, stmnt_filepath, stmnt_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains direct verification states of requirements based on the following conditions:
-- - passed: all of the following conditions must be met
--   - requirement has satisfies or verifies traces, or is explicitly verified by at least one test case
--     - if no *statisfies* trace exists for the requirement,
--       and a direct *verifies* trace mentions the ID and the trace is covered by at least one statement
--       from coverage metrics of a test run or test case, and all test runs or test cases
--       that cover the statement passed
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
-- - unverified: none of the conditions for passed or skipped applied
--   e.g. no satisfies or verifies traces exist, no review for ManualRequirements,
--   and no explicit verification by a test case
--
-- **Note:** Direct means that the state is indipendent of the state of related requirements.
create table DirectRequirementVerificationStates (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,

    state integer not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains verification states for requirements based on the requirement hierarchy.
-- This table only contains non-leaf requirements (requirements that have at least one child).
-- States:
-- - passed: all descendants passed
-- - failed: at least one descendant failed (or is of unknown state)
-- - skipped: at least one descendant was skipped, but none failed
-- - unverified: at least one descendant was unverified, but none failed or were skipped
create table IndirectRequirementVerificationStates (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    state integer not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are successfully verified (passed).
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is passed.
-- For non-leaf requirements, the IndirectRequirementVerificationStates must be passed,
-- and if an entry in DirectRequirementVerificationStates is available it must also be passed.
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
--   DirectRequirementVerificationStates = passed or skipped
-- - IndirectRequirementVerificationStates = passed or skipped
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

-- Contains requirements that are skipped.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is unverified.
-- For non-leaf requirements, either
-- - IndirectRequirementVerificationStates = unverified
--   DirectRequirementVerificationStates = passed, skipped, or unverified
-- - IndirectRequirementVerificationStates = passed, skipped, or unverified
--   DirectRequirementVerificationStates = unverified
create table UnverifiedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);
