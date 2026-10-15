-- Contains covered lines mapped to traces that are covered by failed test runs
-- Note: may also be covered by passed test runs, but at least one failed test run
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestRuns (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_collect_nr integer not null,
    test_run_name text not null,
    test_run_date text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (agg_collect_nr, product_id, test_collect_nr, test_run_name, test_run_date, filepath, traced_line, cov_line),
    foreign key (test_collect_nr, product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(collect_nr, product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are covered by failed test cases
-- Note: may also be covered by passed test cases, but at least one failed test case
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestCases (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_collect_nr integer not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (agg_collect_nr, product_id, test_collect_nr, test_run_name, test_run_date, test_case_name, filepath, traced_line, cov_line),
    foreign key (test_collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(collect_nr, product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesCoveredByFailedTestRuns
-- and TraceMappedLinesCoveredByFailedTestCases
create table TraceMappedLinesCoveredByFailedTests (
    agg_collect_nr integer not null,
    product_id text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (agg_collect_nr, product_id, filepath, traced_line, cov_line),
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have at least one linked line that was covered by a failed test.
create table TracesCoveredByFailedTests (
    agg_collect_nr integer not null,
    product_id text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (agg_collect_nr, product_id, filepath, traced_line),
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create table TraceMappedLinesCoveredBySkippedTests (
    agg_collect_nr integer not null,
    product_id text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (agg_collect_nr, product_id, filepath, traced_line, cov_line),
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create table TracesCoveredBySkippedTests (
    agg_collect_nr integer not null,
    product_id text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (agg_collect_nr, product_id, filepath, traced_line),
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create view TraceMappedLinesCoveredByFailedOrSkippedTests as
select
    agg_collect_nr,
    product_id,
    file_collect_nr,
    filepath,
    file_hash,
    traced_line,
    cov_line
from TraceMappedLinesCoveredByFailedTests
union
select
    agg_collect_nr,
    product_id,
    file_collect_nr,
    filepath,
    file_hash,
    traced_line,
    cov_line
from TraceMappedLinesCoveredBySkippedTests;

-- Contains covered lines mapped to traces that are only covered by passed test runs
create table TraceMappedLinesOnlyCoveredByPassedTestRuns (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_collect_nr integer not null,
    test_run_name text not null,
    test_run_date text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (agg_collect_nr, product_id, test_collect_nr, test_run_name, test_run_date, filepath, traced_line, cov_line),
    foreign key (test_collect_nr, product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(collect_nr, product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are only covered by passed test cases
create table TraceMappedLinesOnlyCoveredByPassedTestCases (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    test_collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (agg_collect_nr, product_id, test_collect_nr, test_run_name, test_run_date, test_case_name, filepath, traced_line, cov_line),
    foreign key (test_collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(collect_nr, product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesOnlyCoveredByPassedTestRuns
-- and TraceMappedLinesOnlyCoveredByPassedTestCases
create table TraceMappedLinesOnlyCoveredByPassedTests (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    product_id text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (agg_collect_nr, product_id, filepath, traced_line, cov_line),
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have no mapped line that was covered by a failed test.
create table TracesOnlyCoveredByPassedTests (
    agg_collect_nr integer not null,
    product_id text not null,
    file_collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (agg_collect_nr, product_id, filepath, traced_line),
    foreign key (file_collect_nr, product_id, filepath, file_hash) references ProductRelatedHashedFiles (collect_nr, product_id, filepath, file_hash) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains direct verification states of requirements
--
-- **Note:** Direct means that the state is indipendent of the state of related requirements.
-- [req_clarify("req.state")]
create table DirectRequirementVerificationStates (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    -- 0=failed; 1=verified; 2=skipped; 3=unverified
    state integer not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table UsableLeafRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table UsableNonLeafRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementsWithOnlyOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Note: Leaf requirements are their own child
create table StatesOfRequirementsWithOnlyOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    child_collect_nr integer not null,
    child_product_id text not null,
    child_req_id text not null,
    state integer not null,
    primary key (
        agg_collect_nr,
        req_collect_nr,
        product_id,
        req_id,
        child_collect_nr,
        child_product_id,
        child_req_id
    ),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    foreign key (child_collect_nr, child_product_id, child_req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr and req_collect_nr <= agg_collect_nr)
);

create table FailedRequirements (
    agg_collect_nr integer not null,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table VerifiedRequirementsWithOnlyOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementsWithNonOptionalChildrenHavingSkippedState (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementsWithChildrenHavingPartiallyVerifiedState (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains verification states for requirements based on the requirement hierarchy.
-- This table only contains non-leaf requirements (requirements that have at least one child).
create table IndirectRequirementVerificationStates (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr,
    product_id text not null,
    req_id text not null,
    state integer not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementVerificationStates (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    state integer not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains requirements that are successfully verified.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is verified.
create table VerifiedRequirements (
    agg_collect_nr integer not null,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (agg_collect_nr, req_collect_nr, product_id, req_id)
        references RequirementVerificationStates(agg_collect_nr, req_collect_nr, product_id, req_id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains requirements that are skipped.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is skipped.
create table SkippedRequirements (
    agg_collect_nr integer not null,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (agg_collect_nr, req_collect_nr, product_id, req_id)
        references RequirementVerificationStates(agg_collect_nr, req_collect_nr, product_id, req_id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table SatisfiedRequirements (
    agg_collect_nr integer not null,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (agg_collect_nr, req_collect_nr, product_id, req_id)
        references RequirementVerificationStates(agg_collect_nr, req_collect_nr, product_id, req_id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains requirements that are neither verified nor satisfied.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is unknown.
create table UnknownRequirements (
    agg_collect_nr integer not null,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (agg_collect_nr, req_collect_nr, product_id, req_id)
        references RequirementVerificationStates(agg_collect_nr, req_collect_nr, product_id, req_id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);
