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
-- **Note:** Children of requirements that are explicitly marked are also affected.
create table DeprecatedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked to `ignore` them.
--
-- **Note:** Children of requirements that are explicitly marked are also affected.
create table IgnoredRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);

-- Contains requirements that are marked to require `manual verification`.
--
-- **Note:** Children of requirements that are explicitly marked are also affected.
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

-- Contains requirements that are satisfied either by a direct *satisfies* trace mentioning the ID,
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

create table TraceCoveragePerTestRuns (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    stmnt_line text not null,
    hits integer not null,
    primary key (product_id, test_run_name, test_run_date, filepath, file_hash, traced_line, stmnt_line),
    foreign key (product_id, test_run_name, test_run_date, filepath, stmnt_line)
        references TestRunStatementCoverage(product_id, test_run_name, test_run_date, stmnt_filepath, stmnt_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create view CoveredTracesPerTestRuns as
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    filepath,
    file_hash,
    traced_line,
    stmnt_line
from TraceCoveragePerTestRuns
where hits > 0;

create table TraceCoveragePerTestCases (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    stmnt_line text not null,
    hits integer not null,
    primary key (product_id, test_run_name, test_run_date, test_case_name, filepath, file_hash, traced_line, stmnt_line),
    foreign key (product_id, test_run_name, test_run_date, test_case_name, filepath, stmnt_line)
        references TestCaseStatementCoverage(product_id, test_run_name, test_run_date, test_case_name, stmnt_filepath, stmnt_line) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

create view CoveredTracesPerTestCases as
select
    last_collect_nr,
    product_id,
    test_run_name,
    test_run_date,
    test_case_name,
    filepath,
    file_hash,
    traced_line,
    stmnt_line
from TraceCoveragePerTestCases
where hits > 0;

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
