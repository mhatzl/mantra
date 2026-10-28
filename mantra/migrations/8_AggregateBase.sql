-- Table to map requirement traces to products.
--
-- **Note:** Requirement trace could either be in DetectedReqTraces or DetectedProductReqTraces,
-- because requirement traces without an explicit product ID are mapped to any product that
-- collects content the trace is located in.
-- [req("trace.id", "trace.mult_reqs")]
create table DirectProductReqTraces (
    product_id text not null,
    req_id text not null,
    filepath text not null,
    file_hash text not null,
    line integer not null,
    primary key (product_id, req_id, filepath, line),
    foreign key (file_hash, line) references Traces (file_hash, line) on delete cascade,
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade,
    foreign key (product_id, filepath)
        references ProductRelatedHashedFiles (product_id, filepath) on delete cascade
);

-- Table to represent the requirement hierarchy.
--
-- [req("req.hierarchy")]
create table RequirementHierarchies (
    -- Product ID the child requirement is defined in.
    child_product_id text not null,
    -- The ID of the child requirement, whose content referenced the parent ID.
    child_req_id text not null,
    -- The product ID the parent requirement is defined in.
    parent_product_id text not null,
    -- The ID of the parent requirement.
    parent_req_id text not null,
    -- 'true' makes the child requirement optional for the parent requirement.
    optional bool not null,
    primary key (child_product_id, child_req_id, parent_product_id, parent_req_id),
    foreign key (child_product_id, child_req_id) references Requirements (product_id, id) on delete cascade,
    foreign key (parent_product_id, parent_req_id) references Requirements (product_id, id) on delete cascade
);

-- Contains tables used as base for many follow up analysis steps.

-- Contains requirements that have no parents.
-- Root requirements also have no parents across products.
create table RootRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains descendants per requirements.
create table RequirementDescendants (
    product_id text not null,
    req_id text not null,
    descendant_product_id text not null,
    descendant_req_id text not null,
    optional bool not null,
    primary key (product_id, req_id, descendant_product_id, descendant_req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade,
    foreign key (descendant_product_id, descendant_req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that have no child requirements.
create table LeafRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked as `deprecated`.
--
-- **Note:** Children of explicitly marked requirements are also affected.
create table DeprecatedRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked to `exclude` them.
--
-- **Note:** Propagates to child requirements if all parents are marked `exclude`.
create table ExcludedRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked as `optional`.
--
-- **Note:** Propagates to child requirements if all parents are marked `optional`.
create table OptionalRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked to require `manual verification`.
--
-- **Note:** Propagates to child requirements if all parents are marked to require `manual verification`.
create table ManualRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are neither deprecated nor excluded.
create table UsableRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains *usable* requirements that are not part of the ManualRequirements table.
create table UsableNonManualRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains *usable* requirements that are part of the ManualRequirements table.
create table UsableManualRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are satisfied either by a *satisfies* trace mentioning the ID,
-- or it is verified by a review if the requirement is part of the ManualRequirements table.
create table DirectlySatisfiedRequirements (
    product_id text not null,
    req_id text not null,
    primary key (product_id, req_id),
    foreign key (product_id, req_id) references Requirements(product_id, id) on delete cascade
);

-- Contains the line span affected by a trace.
-- e.g. Span of an element a trace is mapped to
create table TraceSpans (
    file_hash text not null,
    traced_line integer not null,
    start_line integer not null,
    end_line integer not null,
    primary key (file_hash, traced_line, start_line),
    foreign key (file_hash, traced_line) references Traces (file_hash, line) on delete cascade,
    constraint start_le_end check (start_line <= traced_line and traced_line <= end_line)
);

-- Contains lines that must be excluded from coverage analysis.
-- Aggregate from block and line exclusion marker.
create table ExcludedCoverageLines (
    file_hash text not null references FileHashes (hash) on delete restrict,
    line integer not null,
    primary key (file_hash, line)
);

create table CollectedFileHashes (
    filepath text not null primary key references CollectedFiles(filepath) on delete cascade,
    file_hash text not null references FileHashes (hash) on delete restrict
);

-- Contains reviews that are obsolete and must **not** be used for further analysis.
create table ObsoleteReviews (
    product_id text not null,
    review_name text not null,
    primary key (product_id, review_name),
    foreign key (product_id, review_name) references Reviews(product_id, name) on delete cascade
);

-- Contains test runs that are obsolete and must **not** be used for further analysis.
create table ObsoleteTestRuns (
    product_id text not null,
    test_run_name text not null,
    primary key (product_id, test_run_name),
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

create table ValidReviews (
    product_id text not null,
    review_name text not null,
    primary key (product_id, review_name),
    foreign key (product_id, review_name) references Reviews(product_id, name) on delete cascade
);

create table ValidTestRuns (
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (agg_collect_nr, test_collect_nr, product_id, test_run_name, test_run_date),
    foreign key (test_collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

-- Contains the resolved state of test cases considering potential overrides from reviews.
create table ResolvedTestCaseStates (
    product_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    -- State of the test case.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed; 4=obsolete
    -- [req("testcov.test_case.state")]
    state integer not null,
    primary key (product_id, test_run_name, test_case_name),
    foreign key (product_id, test_run_name, test_case_name)
        references TestCases (product_id, test_run_name, name)
);

create view PassedTestCases as
select
    product_id,
    test_run_name,
    test_case_name
from ResolvedTestCaseStates
where state = 1;

create view SkippedTestCases as
select
    product_id,
    test_run_name,
    test_case_name
from ResolvedTestCaseStates
where state = 2;

create view FailedTestCases as
select
    product_id,
    test_run_name,
    test_case_name
from ResolvedTestCaseStates
-- Note: `unknown` is also considered as failure
where state != 1 and state != 2;

-- Contains test cases that passed and are **not** part of an obsolete test run.
create table UsableTestCases (
    product_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    primary key (product_id, test_run_name, test_case_name),
    foreign key (product_id, test_run_name, test_case_name)
        references TestCases (product_id, test_run_name, name)
);

create table TestRunDescendants (
    product_id text not null,
    test_run_name text not null,
    descendant_test_run_name text not null,
    primary key (product_id, test_run_name, descendant_test_run_name),
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade,
    foreign key (product_id, descendant_test_run_name) references TestRuns(product_id, name) on delete cascade
);

create table LeafTestRuns (
    product_id text not null,
    test_run_name text not null,
    primary key (product_id, test_run_name),
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

create table PassedTestRuns (
    product_id text not null,
    test_run_name text not null,
    primary key (product_id, test_run_name),
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

create table FailedTestRuns (
    product_id text not null,
    test_run_name text not null,
    primary key (product_id, test_run_name),
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

create table SkippedTestRuns (
    product_id text not null,
    test_run_name text not null,
    primary key (product_id, test_run_name),
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

-- Contains test tuns that are **not** obsolete and passed.
create table UsableTestRuns (
    product_id text not null,
    test_run_name text not null,
    primary key (product_id, test_run_name),
    foreign key (product_id, test_run_name) references TestRuns(product_id, name) on delete cascade
);

create view TestRunStates as
with BaseTestRunStates (
    product_id,
    test_run_name,
    state
) as (
    select
        product_id,
        test_run_name,
        0 as state
    from FailedTestRuns
    union all
    select
        product_id,
        test_run_name,
        1 as state
    from UsableTestRuns -- excludes obsolete test runs
    union all
    select
        product_id,
        test_run_name,
        2 as state
    from SkippedTestRuns
)
select
    product_id,
    test_run_name,
    state
from BaseTestRunStates
union all
-- to not overwrite failed or skipped obsolete test runs
select
    product_id,
    test_run_name,
    4 as state
from ObsoleteTestRuns ot
where not exists (
    select * from BaseTestRunStates bt
    where ot.product_id = bt.product_id
    and ot.test_run_name = bt.test_run_name
);

-- Contains line coverage from test runs with optional review overrides applied.
create table ResolvedTestRunLineCoverage (
    product_id text not null,
    test_run_name text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
    hits integer not null,
    primary key (
        product_id,
        test_run_name,
        cov_filepath,
        cov_line
    ),
    foreign key (
        product_id,
        test_run_name,
        cov_filepath,
        cov_line
    ) references TestRunLineCoverage (
        product_id,
        test_run_name,
        cov_filepath,
        cov_line
    ) on delete cascade
);

-- Contains line coverage from test cases with optional review overrides applied.
create table ResolvedTestCaseLineCoverage (
    product_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
    hits integer not null,
    primary key (
        product_id,
        test_run_name,
        test_case_name,
        cov_filepath,
        cov_line
    ),
    foreign key (
        product_id,
        test_run_name,
        test_case_name,
        cov_filepath,
        cov_line
    ) references TestCaseLineCoverage (
        product_id,
        test_run_name,
        test_case_name,
        cov_filepath,
        cov_line
    ) on delete cascade
);

create table ResolvedLineCoverageStates (
    product_id text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
    primary key (
        product_id,
        cov_filepath,
        cov_line
    )
);

create table ResolvedLineCoverageTestStates (
    product_id text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
    primary key (
        product_id,
        cov_filepath,
        cov_line
    )
);

create table TraceCoveragePerTestRuns (
    product_id text not null,
    test_run_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    hits integer not null,
    primary key (product_id, test_run_name, test_run_date, filepath, traced_line, cov_line),
    foreign key (product_id, test_run_name, filepath, cov_line)
        references TestRunLineCoverage(product_id, test_run_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create view TracesCoveredByTestRuns as
select distinct
    product_id,
    test_run_name,
    filepath,
    file_hash,
    traced_line
from TraceCoveragePerTestRuns;

create table TraceCoveragePerTestCases (
    product_id text not null,
    test_run_name text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    hits integer not null,
    primary key (product_id, test_run_name, test_case_name, filepath, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(product_id, test_run_name, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedHashedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create view TracesCoveredByTestCases as
select distinct
    product_id,
    test_run_name,
    test_case_name,
    filepath,
    file_hash,
    traced_line
from TraceCoveragePerTestCases;

-- Contains traces covered by tests.
create view TracesCoveredByTests as
select
    product_id,
    filepath,
    file_hash,
    traced_line
from TraceCoveragePerTestCases
union
select
    product_id,
    filepath,
    file_hash,
    traced_line
from TraceCoveragePerTestRuns;

create view CoverableLinesPerFilepath as
with CoveredLinesPerFilepath (product_id, filepath, line) as (
	select product_id, cov_filepath, cov_line
	from ResolvedTestRunLineCoverage

	union

	select product_id, cov_filepath, cov_line
	from ResolvedTestCaseLineCoverage
)
select product_id, filepath, count(line) as coverable_lines
from CoveredLinesPerFilepath
group by product_id, filepath;
