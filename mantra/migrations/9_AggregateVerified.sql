-- Contains covered lines mapped to traces that are covered by failed test runs
-- Note: may also be covered by passed test runs, but at least one failed test run
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestRuns (
    product_id text not null,
    test_run_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (product_id, test_run_name, filepath, traced_line, cov_line),
    foreign key (product_id, test_run_name, filepath, cov_line)
        references TestRunLineCoverage(product_id, test_run_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are covered by failed test cases
-- Note: may also be covered by passed test cases, but at least one failed test case
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestCases (
    product_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (product_id, test_run_name, test_case_name, filepath, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(product_id, test_run_name, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesCoveredByFailedTestRuns
-- and TraceMappedLinesCoveredByFailedTestCases
create table TraceMappedLinesCoveredByFailedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (product_id, filepath, traced_line, cov_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have at least one linked line that was covered by a failed test.
create table TracesCoveredByFailedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, traced_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create table TraceMappedLinesCoveredBySkippedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (product_id, filepath, traced_line, cov_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create table TracesCoveredBySkippedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, traced_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create view TraceMappedLinesCoveredByFailedOrSkippedTests as
select
    product_id,
    filepath,
    file_hash,
    traced_line,
    cov_line
from TraceMappedLinesCoveredByFailedTests
union
select
    product_id,
    filepath,
    file_hash,
    traced_line,
    cov_line
from TraceMappedLinesCoveredBySkippedTests;

-- Contains covered lines mapped to traces that are only covered by passed test runs
create table TraceMappedLinesOnlyCoveredByPassedTestRuns (
    product_id text not null,
    test_run_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (product_id, test_run_name, filepath, traced_line, cov_line),
    foreign key (product_id, test_run_name, filepath, cov_line)
        references TestRunLineCoverage(product_id, test_run_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are only covered by passed test cases
create table TraceMappedLinesOnlyCoveredByPassedTestCases (
    product_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (product_id, test_run_name, test_case_name, filepath, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(product_id, test_run_name, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesOnlyCoveredByPassedTestRuns
-- and TraceMappedLinesOnlyCoveredByPassedTestCases
create table TraceMappedLinesOnlyCoveredByPassedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (product_id, filepath, traced_line, cov_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have no mapped line that was covered by a failed test.
create table TracesOnlyCoveredByPassedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, traced_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains direct verification states of requirements
--
-- **Note:** Direct means that the state is indipendent of the state of related requirements.
-- [req_clarify("req.state")]
create table DirectRequirementVerificationStates (
    product_id text not null,
    req_id text not null,
    -- 0=failed; 1=verified; 2=skipped; 3=unverified
    state integer not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

create table UsableLeafRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

create table UsableNonLeafRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

create table RequirementsWithOnlyOptionalChildren (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Note: Leaf requirements are their own child
create table StatesOfRequirementsWithOnlyOptionalChildren (
    product_id text not null,
    req_id text not null,
    child_product_id text not null,
    child_req_id text not null,
    state integer not null,
    primary key (
        product_id,
        req_id,
        child_product_id,
        child_req_id
    ),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (child_product_id, child_req_id) references Requirements(product_id, id) on delete cascade
);

create table FailedRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

create table VerifiedRequirementsWithOnlyOptionalChildren (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

create table RequirementsWithNonOptionalChildrenHavingSkippedState (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

create table RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains verification states for requirements based on the requirement hierarchy.
-- This table only contains non-leaf requirements (requirements that have at least one child).
create table IndirectRequirementVerificationStates (
    product_id text not null,
    req_id text not null,
    state integer not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

create table RequirementVerificationStates (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    req_id text not null,
    state integer not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are successfully verified.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is verified.
create table VerifiedRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id)
        references RequirementVerificationStates(product_id, req_id) on delete cascade
);

-- Contains requirements that are skipped.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is skipped.
create table SkippedRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id)
        references RequirementVerificationStates(product_id, req_id) on delete cascade
);

create table SatisfiedRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id)
        references RequirementVerificationStates(product_id, req_id) on delete cascade
);

-- Contains requirements that are neither verified nor satisfied.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is unknown.
create table UnknownRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id)
        references RequirementVerificationStates(product_id, req_id) on delete cascade
);

create table ChangedRequirementVerificationStates (
    product_id text not null,
    req_id text not null,
    old_state integer not null,
    primary key (product_id, req_id),
    foreign key (
        product_id,
        req_id
    ) references RequirementVerificationStates (product_id, req_id) on delete cascade
);

create trigger RequirementVerificationStateChanges
after update on RequirementVerificationStates
for each row
when (
    old.data_hash is distinct from new.data_hash
)
begin
    insert into ChangedRequirementVerificationStates (
        product_id,
        req_id,
        old_state
    )
    values (
        old.product_id,
        old.req_id,
        old.state
    );
end;
