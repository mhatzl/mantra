-- Table to represent the requirement hierarchy.
--
-- [req("req.hierarchy")]
create table RequirementHierarchies (
    child_collect_nr integer not null,
    -- Product ID the child requirement is defined in.
    child_product_id text not null,
    -- The ID of the child requirement, whose content referenced the parent ID.
    child_req_id text not null,
    parent_collect_nr integer not null,
    -- The product ID the parent requirement is defined in.
    parent_product_id text not null,
    -- The ID of the parent requirement.
    parent_req_id text not null,
    -- 'true' makes the child requirement optional for the parent requirement.
    optional bool not null,
    primary key (child_collect_nr, child_product_id, child_req_id, parent_collect_nr, parent_product_id, parent_req_id),
    foreign key (child_collect_nr, child_product_id, child_req_id) references Requirements (collect_nr, product_id, id) on delete cascade deferrable initially deferred,
    foreign key (parent_collect_nr, parent_product_id, parent_req_id) references Requirements (collect_nr, product_id, id) on delete cascade deferrable initially deferred
);

-- Contains tables used as base for many follow up analysis steps.

-- Contains requirements that have no parents.
-- Root requirements also have no parents across products.
create table RootRequirements (
    -- the collection in which the requirement is a root requirement
    -- **Note:** Needed in case later collections affect the requirements hierarchy without collecting the requirement itself
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains descendants per requirements.
create table RequirementDescendants (
    -- the collection in which the entry was added
    -- may either match with collect_nr or descendant_collect_nr
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    descendant_collect_nr integer not null,
    descendant_product_id text not null,
    descendant_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id, descendant_collect_nr, descendant_product_id, descendant_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    foreign key (descendant_collect_nr, descendant_product_id, descendant_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint related_collection (agg_collect_nr = req_collect_nr or agg_collect_nr = descendant_collect_nr)
);

-- Contains requirements that have no child requirements.
create table LeafRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains requirements that are marked as `deprecated`.
--
-- **Note:** Children of explicitly marked requirements are also affected.
create table DeprecatedRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains requirements that are marked to `exclude` them.
--
-- **Note:** Propagates to child requirements if all parents are marked `exclude`.
create table ExcludedRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains requirements that are marked as `optional`.
--
-- **Note:** Propagates to child requirements if all parents are marked `optional`.
create table OptionalRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains requirements that are marked to require `manual verification`.
--
-- **Note:** Propagates to child requirements if all parents are marked to require `manual verification`.
create table ManualRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains requirements that are neither deprecated nor excluded.
create table UsableRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains *usable* requirements that are not part of the ManualRequirements table.
create table UsableNonManualRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains *usable* requirements that are part of the ManualRequirements table.
create table UsableManualRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
);

-- Contains requirements that are satisfied either by a *satisfies* trace mentioning the ID,
-- or it is verified by a review if the requirement is part of the ManualRequirements table.
create table DirectlySatisfiedRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade
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

-- Contains reviews that are likely obsolete, but are still used for further analysis.
-- This uses available historic data to flag reviews as likely obsolete.
-- It is then up to the user to decide what to do.
--
-- Likely reasons:
-- - manually verified requirement changed since review date
-- - test run of mapped overrides is marked as (likely) obsolete
create table LikelyObsoleteReviews (
    collect_nr integer not null,
    product_id text not null,
    review_name text not null,
    review_date text not null,
    primary key (collect_nr, product_id, review_name, review_date),
    foreign key (collect_nr, product_id, review_name, review_date) references Reviews(collect_nr, product_id, name, utc_date) on delete cascade
);

-- Contains test runs that are obsolete and must **not** be used for further analysis.
-- Reasons why a test run may be obsolete:
-- - test case location contains file hash for filepath that differs to the hash collected in the latest run
-- - coverage data contains file hash for filepath that differs to the hash collected in the latest run
--
-- **Note:** Cannot use historic data for this prediction,
-- because initial data may have been collected long after the date of a test run,
-- but data could still have been changed between.
create table ObsoleteTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

-- Contains test runs that are likely obsolete, but are still used for further analysis.
-- This uses available historic data to flag test runs as likely obsolete.
-- It is then up to the user to decide what to do.
--
-- Likely reasons:
-- - verified requirement changed since test run date
-- - file hash for the filepath of the test case location or coverage data changed since test run date
create table LikelyObsoleteTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

-- Contains the resolved state of test cases considering potential overrides from reviews.
-- TODO: check for primary and foreign key
create table ResolvedTestCaseStates (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    -- State of the test case.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed; 4=obsolete
    -- [req("testcov.test_case.state")]
    state integer not null
);

create view PassedTestCases as
select
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name
from ResolvedTestCaseStates
where state = 1;

create view SkippedTestCases as
select
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name
from ResolvedTestCaseStates
where state = 2;

create view FailedTestCases as
select
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name
from ResolvedTestCaseStates
-- Note: `unknown` is also considered as failure
where state != 1 and state != 2;

-- Contains test cases that passed and are **not** part of an obsolete test run.
create table UsableTestCases (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null
);

create table TestRunDescendants (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    descendant_test_run_name text not null,
    descendant_test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date, descendant_test_run_name, descendant_test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade,
    foreign key (collect_nr, product_id, descendant_test_run_name, descendant_test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

create table LeafTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

create table PassedTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

create table FailedTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

create table SkippedTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

-- Contains test tuns that are **not** obsolete and passed.
create table UsableTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date),
    foreign key (collect_nr, product_id, test_run_name, test_run_date) references TestRuns(collect_nr, product_id, name, utc_date) on delete cascade
);

create view TestRunStates as
with BaseTestRunStates (
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    state
) as (
    select
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        0 as state
    from FailedTestRuns
    union all
    select
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        1 as state
    from UsableTestRuns -- excludes obsolete test runs
    union all
    select
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        2 as state
    from SkippedTestRuns
)
select
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    state
from BaseTestRunStates
union all
-- to not overwrite failed or skipped obsolete test runs
select
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    4 as state
from ObsoleteTestRuns ot
where not exists (
    select * from BaseTestRunStates bt
    where ot.collect_nr = bt.collect_nr
    and ot.product_id = bt.product_id
    and ot.test_run_name = bt.test_run_name
    and ot.test_run_date = bt.test_run_date
);

-- Contains line coverage from test runs with optional review overrides applied.
create table ResolvedTestRunLineCoverage (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
    hits integer,
    primary key (
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        cov_filepath,
        cov_line
    ),
    foreign key (
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        cov_filepath,
        cov_line
    ) references TestRunLineCoverage (
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        cov_filepath,
        cov_line
    ) on delete cascade
);

-- Contains line coverage from test cases with optional review overrides applied.
create table ResolvedTestCaseLineCoverage (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
    hits integer,
    primary key (
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        cov_filepath,
        cov_line
    ),
    foreign key (
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        cov_filepath,
        cov_line
    ) references TestCaseLineCoverage (
        collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        cov_filepath,
        cov_line
    ) on delete cascade
);

create table ResolvedLineCoverageStates (
    collect_nr integer not null,
    product_id text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
    primary key (
        collect_nr,
        product_id,
        cov_filepath,
        cov_line
    ),
    foreign key (collect_nr, product_id, cov_filepath) references ProductRelatedFiles (collect_nr, product_id, filepath) on delete cascade
);

create table TraceCoveragePerTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    hits integer not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(collect_nr, product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces covered by test runs.
create view TracesCoveredByTestRuns as
select distinct
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    filepath,
    file_hash,
    traced_line
from TraceCoveragePerTestRuns;

create table TraceCoveragePerTestCases (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    hits integer not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(collect_nr, product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces covered by test cases.
create view TracesCoveredByTestCases as
select distinct
    collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name,
    filepath,
    file_hash,
    traced_line
from TraceCoveragePerTestCases;

-- Contains traces covered by tests.
create view TracesCoveredByTests as
select
    collect_nr,
    product_id,
    filepath,
    file_hash,
    traced_line
from TracesCoveredByTestCases
union
select
    collect_nr,
    product_id,
    filepath,
    file_hash,
    traced_line
from TracesCoveredByTestRuns;

create view CoverableLinesPerFilepath as
with CoveredLinesPerFilepath (collect_nr, product_id, filepath, line) as (
	select collect_nr, product_id, cov_filepath, cov_line
	from ResolvedTestRunLineCoverage

	union

	select collect_nr, product_id, cov_filepath, cov_line
	from ResolvedTestCaseLineCoverage
)
select collect_nr, product_id, filepath, count(line) as coverable_lines
from CoveredLinesPerFilepath
group by collect_nr, product_id, filepath;
