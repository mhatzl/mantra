
-- requirements that may be traced.
-- generation is used to show changes for "--dry-run" and to delete non-existing requirements.
-- annotation might be "manual" or "deprecated"
create table Requirements (
    id text not null primary key,
    generation integer not null,
    origin text not null,
    annotation text
);

-- hierarchy
create table RequirementHierarchies (
    child_id text not null references Requirements(id) on delete cascade,
    parent_id text not null references Requirements(id) on delete cascade,
    primary key (child_id, parent_id)
);

create view RequirementChildren as
with recursive TransitiveChildren(id, child_id) as
(
    select parent_id, child_id from RequirementHierarchies
    union all
    select tc.id, rh.child_id from RequirementHierarchies rh, TransitiveChildren tc
    where tc.child_id = rh.parent_id
)
select id, child_id from TransitiveChildren;

create view DeprecatedRequirements as
select id, origin, annotation from Requirements
where lower(annotation) = 'deprecated';

create view ManualRequirements as
select id, origin, annotation from Requirements
where lower(annotation) = 'manual';

create view DirectlyTracedRequirements as
select id, origin, annotation from Requirements
where id in (select req_id from Traces);

create view IndirectlyTracedRequirements as
select r.id, r.origin, r.annotation from Requirements r, RequirementChildren c
where r.id = c.id and c.child_id in (select id from DirectlyTracedRequirements)
and r.id not in (select id from DirectlyTracedRequirements);

create view TracedRequirements as
select id, origin, annotation from DirectlyTracedRequirements
union all
select id, origin, annotation from IndirectlyTracedRequirements;

create view UntracedRequirements as
select id, origin, annotation from Requirements
except
select id, origin, annotation from TracedRequirements;

create view DirectlyCoveredRequirements as
select id, origin, annotation from Requirements
where id in (select req_id from TestCoverage);

create view IndirectlyCoveredRequirements as
select r.id, r.origin, r.annotation from Requirements r, RequirementChildren c
where r.id = c.id and c.child_id in (select id from DirectlyCoveredRequirements)
and r.id not in (select id from DirectlyCoveredRequirements);

create view CoveredRequirements as
select id, origin, annotation from DirectlyCoveredRequirements
union all
select id, origin, annotation from IndirectlyCoveredRequirements;

create view UncoveredRequirements as
select id, origin, annotation from Requirements
except
select id, origin, annotation from CoveredRequirements;

create view PassedCoveredRequirements as
select id, origin, annotation from CoveredRequirements
where id not in (select req_id from FailedTestCoverage);

create view FailedCoveredRequirements as
select id, origin, annotation from CoveredRequirements
where id in (select req_id from FailedTestCoverage);

create view RequirementCoverageOverview as
with NrRequirements(cnt) as (select count(*) from Requirements),
NrTraced(cnt) as (select count(*) from TracedRequirements),
NrCovered(cnt) as (select count(*) from CoveredRequirements),
NrPassed(cnt) as (select count(*) from PassedCoveredRequirements)
select r.cnt as req_cnt, t.cnt as traced_cnt, case when r.cnt = 0 then null else (t.cnt * 1.0 / r.cnt) end as traced_ratio,
    c.cnt as covered_cnt, case when r.cnt = 0 then null else (c.cnt * 1.0 / r.cnt) end as covered_ration,
    p.cnt as passed_cnt, case when r.cnt = 0 then null else (p.cnt * 1.0 / r.cnt) end as passed_ratio
from NrRequirements r, NrTraced t, NrCovered c, NrPassed p;

-- traces to requirements
-- generation is used to show changes for "--dry-run" and to delete non-existing traces.
create table Traces (
    req_id text not null references Requirements(id) on delete cascade,
    generation integer not null,
    filepath text not null,
    line integer not null,
    primary key (req_id, filepath, line)
);

-- test runs that executed tests
--
-- NOTE: `nr_of_tests` is the number of expected tests in one run.
-- Meaning, if there are fewer associated tests in the Tests table, not all tests were executed.
create table TestRuns (
    name text not null,
    date text not null,
    nr_of_tests integer,
    logs text,
    primary key (name, date)
);

create view PassedTests as
select test_run_name, test_run_date, name, filepath, line
from Tests
where passed = 1;

create view FailedTestCoverage as
select tc.req_id, tc.test_run_name, tc.test_run_date, tc.test_name, tc.filepath, tc.line
from TestCoverage tc, Tests t
where tc.test_run_name = t.test_run_name and tc.test_run_date = t.test_run_date
    and tc.test_name = t.name and t.passed <> 1;

create view TestRunOverview as
with NrTests(name, date, cnt) as
(
    select tr.name, tr.date, count(*)
    from TestRuns tr, Tests t
    where tr.name = t.test_run_name and tr.date = t.test_run_date
    group by tr.name, tr.date
),
NrPassed(name, date, cnt) as
(
    select tr.name, tr.date, count(*)
    from TestRuns tr, PassedTests t
    where tr.name = t.test_run_name and tr.date = t.test_run_date
    group by tr.name, tr.date
),
NrFailed(name, date, cnt) as
(
    select tr.name, tr.date, count(*)
    from TestRuns tr, Tests t
    where tr.name = t.test_run_name and tr.date = t.test_run_date
        and t.passed <> 1
    group by tr.name, tr.date
),
NrSkipped(name, date, cnt) as
(
    select tr.name, tr.date, count(*)
    from TestRuns tr, SkippedTests t
    where tr.name = t.test_run_name and tr.date = t.test_run_date
    group by tr.name, tr.date
),
TestRunCnts(name, date, tests, passed, failed, skipped) as
(
    select name, date, sum(tests), sum(passed), sum(failed), sum(skipped)
    from (
        select name, date, cnt as tests, 0 as passed, 0 as failed, 0 as skipped
        from NrTests
        union all
        select name, date, 0 as tests, cnt as passed, 0 as failed, 0 as skipped
        from NrPassed
        union all
        select name, date, 0 as tests, 0 as passed, cnt as failed, 0 as skipped
        from NrFailed
        union all
        select name, date, 0 as tests, 0 as passed, 0 as failed, cnt as skipped
        from NrSkipped
    )
    where name not null and date not null
    group by name, date
)
select name, date, tests,
    passed, case when tests = 0 then null else (passed * 1.0 / tests) end as passed_ratio,
    failed, case when tests = 0 then null else (failed * 1.0 / tests) end as failed_ratio,
    skipped, case when tests = 0 then null else (skipped * 1.0 / tests) end as skipped_ratio
from TestRunCnts;

create view OverallTestOverview as
select sum(tests) as tests,
    sum(passed) as passed, case when sum(tests) = 0 then null else (sum(passed) * 1.0 / sum(tests)) end as passed_ratio,
    sum(failed) as failed, case when sum(tests) = 0 then null else (sum(failed) * 1.0 / sum(tests)) end as failed_ratio,
    sum(skipped) as skipped, case when sum(tests) = 0 then null else (sum(skipped) * 1.0 / sum(tests)) end as skipped_ratio
from TestRunOverview;

-- tests per test run
--
-- NOTE: 'passed = null' means the test is still running, or was not finished properly.
create table Tests (
    test_run_name text not null,
    test_run_date text not null,
    name text not null,
    filepath text not null,
    line integer not null,
    passed integer,
    primary key (test_run_name, test_run_date, name),
    foreign key (test_run_name, test_run_date) references TestRuns(name, date) on delete cascade
);

-- skipped tests
create table SkippedTests (
    test_run_name text not null,
    test_run_date text not null,
    name text not null,
    filepath text not null,
    line integer not null,
    reason text,
    primary key (test_run_name, test_run_date, name),
    foreign key (test_run_name, test_run_date) references TestRuns(name, date) on delete cascade
);

-- coverage data per test
create table TestCoverage (
    req_id text not null references Requirements(id),
    test_run_name text not null,
    test_run_date text not null,
    test_name text not null,
    filepath text not null,
    line integer not null,
    primary key (req_id, test_run_name, test_run_date, test_name, filepath, line),
    foreign key (test_run_name, test_run_date, test_name) references Tests(test_run_name, test_run_date, name) on delete cascade,
    foreign key (req_id, filepath, line) references Traces(req_id, filepath, line) on delete cascade
);

-- review to add manually verified requirements
create table Reviews (
    name text not null,
    date text not null,
    reviewer text not null,
    comment text,
    primary key (name, date)
);

-- manually verified requirements
create table ManuallyVerified (
    req_id text not null references Requirements(id) on delete cascade,
    review_name text not null,    
    review_date text not null,
    comment text,
    primary key (req_id, review_name, review_date),
    foreign key (review_name, review_date) references Reviews(name, date) on delete cascade
);

create view ManuallyVerifiedRequirements as
select r.id, r.origin, r.annotation from Requirements r, ManuallyVerified m
where r.id = m.req_id;
