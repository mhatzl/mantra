
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

create table TracesCoveredBySkippedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, traced_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that are covered by at least one test with "unknown" state
create table TracesCoveredByUnknownTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, traced_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that are covered by at least one passed test
create table TracesCoveredByPassedTests (
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (product_id, filepath, traced_line),
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create table RequirementsWithSatisfyTraces (
    product_id text not null,
    req_id text not null,
    file_hash text not null,
    trace_line text not null,
    primary key (product_id, req_id, file_hash, trace_line),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (file_hash, trace_line) references Traces(file_hash, line) on delete cascade
);

create table RequirementsWithVerifyTraces (
    product_id text not null,
    req_id text not null,
    file_hash text not null,
    trace_line text not null,
    primary key (product_id, req_id, file_hash, trace_line),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (file_hash, trace_line) references Traces(file_hash, line) on delete cascade
);

create view RequirementsWithSatisfyAndVerifyTraces as
select product_id, req_id
from RequirementsWithSatisfyTraces
union
select product_id, req_id
from RequirementsWithVerifyTraces;

create view RequirementsWithSatisfyButNoVerifyTraces as
select product_id, req_id
from RequirementsWithSatisfyTraces
except
select product_id, req_id
from RequirementsWithVerifyTraces;

create view RequirementsWithVerifyButNoSatisfyTraces as
select product_id, req_id
from RequirementsWithVerifyTraces
except
select product_id, req_id
from RequirementsWithSatisfyTraces;

create view TracesCoveredByPassedAndNoFailedOrUnknownTests as
select product_id, filepath, file_hash, traced_line
from TracesCoveredByPassedTests
except
select product_id, filepath, file_hash, traced_line
from (
    select product_id, filepath, file_hash, traced_line
    from TracesCoveredByFailedTests

    union all

    select product_id, filepath, file_hash, traced_line
    from TracesCoveredByUnknownTests
);

create table RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces (
    product_id text not null,
    req_id text not null,
    test_run_name text not null,
    primary key (product_id, req_id, test_run_name),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

create table RequirementsWithTestCasesCoveringSatisfyAndVerifyTraces (
    product_id text not null,
    req_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    primary key (product_id, req_id, test_run_name, test_case_name),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (product_id, test_run_name, test_case_name) references TestCases(product_id, test_run_name, name) on delete cascade
);

create view RequirementsWithTestsCoveringSatisfyAndVerifyTraces as
select product_id, req_id
from RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces
union
select product_id, req_id
from RequirementsWithTestCasesCoveringSatisfyAndVerifyTraces;

create table RequirementsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces (
    product_id text not null,
    req_id text not null,
    test_run_name text not null,
    primary key (product_id, req_id, test_run_name),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

create table RequirementsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces (
    product_id text not null,
    req_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    primary key (product_id, req_id, test_run_name, test_case_name),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (product_id, test_run_name, test_case_name) references TestCases(product_id, test_run_name, name) on delete cascade
);

create view RequirementsWithTestsCoveringVerifyTracesButNoSatisfyTraces as
select product_id, req_id
from RequirementsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces
union
select product_id, req_id
from RequirementsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces;

-- Contains direct verification states of requirements
--
-- **Note:** Direct means that the state is indipendent of the state of related requirements.
-- [req_clarify("req.state")]
create table DirectRequirementVerificationStates (
    product_id text not null,
    req_id text not null,
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
    old.state is distinct from new.state
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
