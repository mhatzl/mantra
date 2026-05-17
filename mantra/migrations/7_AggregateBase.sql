-- Contains tables used as base for many follow up analysis steps.

-- Contains descendants per requirements.
create table RequirementDescendants (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    descendant_product_id text not null,
    descendant_id text not null,
    primary key (product_id, id, descendant_product_id, descendant_id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade,
    foreign key (descendant_product_id, descendant_id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that have no child requirements.
create table LeafRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked as `deprecated`.
--
-- **Note:** Children of explicitly marked requirements are also affected.
create table DeprecatedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked to `ignore` them.
--
-- **Note:** Children of explicitly marked requirements are also affected.
create table IgnoredRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked as `optional`.
--
-- **Note:** Children of explicitly marked requirements are also affected.
create table OptionalRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked to require `manual verification`.
--
-- **Note:** Children of explicitly marked requirements are also affected.
create table ManualRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are neither deprecated nor marked as `ignore = true`.
create table UsableRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains *usable* requirements that are not part of the ManualRequirements table.
create table UsableNonManualRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains *usable* requirements that are part of the ManualRequirements table.
create table UsableManualRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are satisfied either by a *satisfies* trace mentioning the ID,
-- or it is verified by a review if the requirement is part of the ManualRequirements table.
create table DirectlySatisfiedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
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
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    review_name text not null,
    review_date text not null,
    primary key (product_id, review_name, review_date),
    foreign key (product_id, review_name, review_date) references Reviews(product_id, name, utc_date) on delete cascade
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
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

-- Contains test runs that are likely obsolete, but are still used for further analysis.
-- This uses available historic data to flag test runs as likely obsolete.
-- It is then up to the user to decide what to do.
--
-- Likely reasons:
-- - verified requirement changed since test run date
-- - file hash for the filepath of the test case location or coverage data changed since test run date
create table LikelyObsoleteTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

-- Contains the resolved state of test cases considering potential overrides from reviews.
create table ResolvedTestCaseStates (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
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
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name
from ResolvedTestCaseStates
where state = 1;

create view SkippedTestCases as
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name
from ResolvedTestCaseStates
where state = 2;

create view FailedTestCases as
select
    last_collect_nr,
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
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    descendant_test_run_name text not null,
    descendant_test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date, descendant_test_run_name, descendant_test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade,
    foreign key (product_id, descendant_test_run_name, descendant_test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

create table LeafTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

create table PassedTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

create table FailedTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

create table SkippedTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

-- Contains test tuns that are **not** obsolete and passed.
create table UsableTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    primary key (product_id, test_run_name, test_run_date),
    foreign key (product_id, test_run_name, test_run_date) references TestRuns(product_id, name, utc_date) on delete cascade
);

create view TestRunStates as
with BaseTestRunStates (
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    state
) as (
    select
        last_collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        0 as state
    from FailedTestRuns
    union all
    select
        last_collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        1 as state
    from UsableTestRuns
    union all
    select
        last_collect_nr,
        product_id,
        test_run_name,
        test_run_date,
        2 as state
    from SkippedTestRuns
)
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    state
from BaseTestRunStates
union all
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    4 as state
from ObsoleteTestRuns ot
where not exists (
    select * from BaseTestRunStates bt
    where ot.last_collect_nr = bt.last_collect_nr
    and ot.product_id = bt.product_id
    and ot.test_run_name = bt.test_run_name
    and ot.test_run_date = bt.test_run_date
);

-- Contains line coverage from test runs with optional review overrides applied.
create table ResolvedTestRunLineCoverage (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    cov_filepath text not null,
    cov_file_hash text,
    cov_line integer not null,
    state integer not null,
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
        test_run_date,
        cov_filepath,
        cov_line
    ) references TestRunLineCoverage (
        product_id,
        test_run_name,
        test_run_date,
        cov_filepath,
        cov_line
    ) on delete cascade
);

-- Contains line coverage from test cases with optional review overrides applied.
create table ResolvedTestCaseLineCoverage (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
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
        test_case_name,
        cov_filepath,
        cov_line
    ) references TestCaseLineCoverage (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        cov_filepath,
        cov_line
    ) on delete cascade
);

create table ResolvedLineCoverageStates (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
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
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    hits integer not null,
    primary key (product_id, test_run_name, test_run_date, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces covered by test runs.
create view TracesCoveredByTestRuns as
select distinct
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    filepath,
    file_hash,
    traced_line
from TraceCoveragePerTestRuns;

create table TraceCoveragePerTestCases (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    hits integer not null,
    primary key (product_id, test_run_name, test_run_date, test_case_name, filepath, file_hash, traced_line, cov_line),
    foreign key (product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces covered by test cases.
create view TracesCoveredByTestCases as
select distinct
    last_collect_nr,
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
    last_collect_nr,
    product_id,
    filepath,
    file_hash,
    traced_line
from TracesCoveredByTestCases
union
select
    last_collect_nr,
    product_id,
    filepath,
    file_hash,
    traced_line
from TracesCoveredByTestRuns;

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
