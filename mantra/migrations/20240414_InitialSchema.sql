
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

-----------------------------------------------------------------------------
-- Views
-----------------------------------------------------------------------------

create view RequirementChildren as
with recursive TransitiveChildren(id, child_id) as
(
    select parent_id, child_id from RequirementHierarchies
    union all
    select tc.id, rh.child_id from RequirementHierarchies rh, TransitiveChildren tc
    where tc.child_id = rh.parent_id
)
select id, child_id from TransitiveChildren;

-- Requirements without children
create view LeafRequirements as
select id, origin, annotation
from Requirements
where id not in (select parent_id from RequirementHierarchies);

create view NonLeafRequirements as
select id, origin, annotation
from Requirements
except
select id, origin, annotation
from LeafRequirements;

create view DeprecatedRequirements as
with MarkedDeprecated(id) as (
    select id from Requirements
    where lower(annotation) = 'deprecated'
),
ParentMarkedDeprecated(id) as (
    select rc.child_id
    from RequirementChildren rc, MarkedDeprecated md
    where rc.id = md.id
),
Deprecated(id) as (
    select id from MarkedDeprecated
    union
    select id from ParentMarkedDeprecated
)
select r.id, r.origin, r.annotation
from Requirements r, Deprecated d 
where r.id = d.id;

create view InvalidRequirements as
select d.id
from DeprecatedRequirements d, Traces t
where d.id = t.req_id;

create view ManualRequirements as
with MarkedManual(id) as (
    select id from Requirements
    where lower(annotation) = 'manual'
),
ParentMarkedManual(id) as (
    select rc.child_id
    from RequirementChildren rc, MarkedManual md
    where rc.id = md.id
),
Manual(id) as (
    select id from MarkedManual
    union
    select id from ParentMarkedManual
)
select r.id, r.origin, r.annotation
from Requirements r, Manual d 
where r.id = d.id;

create view DirectlyTracedRequirements as
select id, origin, annotation from Requirements
where id in (select req_id from Traces);

-- A requirement is indirectly traced
-- if **all** of its direct child requirements are either directly or indirectly traced.
create view IndirectlyTracedRequirements as
with recursive IsIndirectlyUntraced(id) as (
    -- Leaf requirements cannot be traced indirectly
    select id
    from LeafRequirements
    where id not in (select id from DirectlyTracedRequirements)
    union all
    -- Recursively get requirements that are not indirectly traced
    select r.id
    from NonLeafRequirements r, RequirementHierarchies rh, IsIndirectlyUntraced u
    where r.id = rh.parent_id
    and rh.child_id = u.id
),
-- Neither directly or indirectly traced requirements
IsUntraced(id) as (
    select id from IsIndirectlyUntraced
    except
    select id from DirectlyTracedRequirements
),
HasUntracedChild(id) as (
    select rh.parent_id
    from RequirementHierarchies rh, IsUntraced u
    where rh.child_id = u.id
)
-- Only non-leaf requirements can be indirectly traced
select distinct id, origin, annotation
from NonLeafRequirements
where id not in (select id from HasUntracedChild);

-- Traces to child requirements.
-- Also includes traces to non-leaf children.
create view IndirectRequirementTraces as
select ir.id, c.child_id as traced_id, t.filepath, t.line
from IndirectlyTracedRequirements ir, RequirementChildren c, Traces t
where ir.id = c.id and c.child_id = t.req_id;

create view TracedRequirements as
select id, origin, annotation from DirectlyTracedRequirements
union
select id, origin, annotation from IndirectlyTracedRequirements;

create view UntracedRequirements as
select id, origin, annotation from Requirements
except
select id, origin, annotation from TracedRequirements;

create view DirectlyCoveredRequirements as
select id, origin, annotation from Requirements
where id in (select req_id from TestCoverage);

-- Indirectly covered requirements have the same constraint
-- as indirectly traced requirements.
--
-- See description for indirectly traced requirements for more information.
create view IndirectlyCoveredRequirements as
with recursive IsIndirectlyUncovered(id) as (
    -- Leaf requirements cannot be covered indirectly
    select id
    from LeafRequirements
    where id not in (select id from DirectlyCoveredRequirements)
    union all
    -- Recursively get requirements that are not indirectly covered
    select r.id
    from NonLeafRequirements r, RequirementHierarchies rh, IsIndirectlyUncovered u
    where r.id = rh.parent_id
    and rh.child_id = u.id
),
-- Neither directly or indirectly covered requirements
IsUncovered(id) as (
    select id from IsIndirectlyUncovered
    except
    select id from DirectlyCoveredRequirements
),
HasUncoveredChild(id) as (
    select rh.parent_id
    from RequirementHierarchies rh, IsUncovered u
    where rh.child_id = u.id
)
-- Only non-leaf requirements can be indirectly uncovered
select distinct id, origin, annotation
from NonLeafRequirements
where id not in (select id from HasUncoveredChild);

-- Test coverage of child requirements.
-- Also includes test coverage of non-leaf children.
create view IndirectRequirementTestCoverage as
select r.id, c.child_id as covered_id, v.test_run_name, v.test_run_date, v.test_name, v.filepath, v.line
from IndirectlyCoveredRequirements r, RequirementChildren c, TestCoverage v
where r.id = c.id and c.child_id = v.req_id;

create view CoveredRequirements as
select id, origin, annotation from DirectlyCoveredRequirements
union
select id, origin, annotation from IndirectlyCoveredRequirements;

create view UncoveredRequirements as
select id, origin, annotation from Requirements
except
select id, origin, annotation from CoveredRequirements;

create view PassedCoveredRequirements as
select r.id, r.origin, r.annotation
from CoveredRequirements r
except
select f.id, f.origin, f.annotation
from FailedCoveredRequirements f;

-- Coverage of a requirement failed if either one of the following holds:
--
-- - one of the tests failed that directly covered the requirement
-- - one of the child requirements has failed coverage
create view FailedCoveredRequirements as
with HasFailedChild(id, covered_id) as (
    select r.id, rc.child_id from Requirements r, RequirementChildren rc, FailedTestCoverage f
    where r.id = rc.id and rc.child_id = f.req_id
)
select c.id, c.origin, c.annotation, hf.covered_id
from CoveredRequirements c, HasFailedChild hf
where c.id = hf.id
union all
select c.id, c.origin, c.annotation, null as covered_id
from CoveredRequirements c, FailedTestCoverage f
where c.id = f.req_id;

create view FailedRequirementCoverage as
select fr.id, null as covered_id, fc.test_run_name, fc.test_run_date, fc.test_name, fc.filepath, fc.line
from FailedCoveredRequirements fr, FailedTestCoverage fc
where fr.id = fc.req_id
union all
select fr.id, fr.covered_id as covered_id, fc.test_run_name, fc.test_run_date, fc.test_name, fc.filepath, fc.line
from FailedCoveredRequirements fr, FailedTestCoverage fc
where fr.covered_id = fc.req_id;

create view RequirementCoverageOverview as
with NrRequirements(cnt) as (select count(*) from Requirements),
NrTraced(cnt) as (select count(*) from TracedRequirements),
NrCovered(cnt) as (select count(*) from CoveredRequirements),
NrPassed(cnt) as (select count(*) from PassedCoveredRequirements)
select r.cnt as req_cnt, t.cnt as traced_cnt, case when r.cnt = 0 then 0.0 else (t.cnt * 1.0 / r.cnt) end as traced_ratio,
    c.cnt as covered_cnt, case when r.cnt = 0 then 0.0 else (c.cnt * 1.0 / r.cnt) end as covered_ratio,
    p.cnt as passed_cnt, case when r.cnt = 0 then 0.0 else (p.cnt * 1.0 / r.cnt) end as passed_ratio
from NrRequirements r, NrTraced t, NrCovered c, NrPassed p;

create view PassedTests as
select test_run_name, test_run_date, name, filepath, line
from Tests
where passed = 1;

create view FailedTestCoverage as
select tc.req_id, tc.test_run_name, tc.test_run_date, tc.test_name, tc.filepath, tc.line
from TestCoverage tc, Tests t
where tc.test_run_name = t.test_run_name and tc.test_run_date = t.test_run_date
    and tc.test_name = t.name and (t.passed <> 1 or t.passed is null);

create view TestRunOverview as
with NrTests(name, date, cnt) as
(
    select tr.name, tr.date, tr.nr_of_tests
    from TestRuns tr
),
NrRanTests(name, date, cnt) as
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
        and (t.passed <> 1 or t.passed is null)
    group by tr.name, tr.date
),
NrSkipped(name, date, cnt) as
(
    select tr.name, tr.date, count(*)
    from TestRuns tr, SkippedTests t
    where tr.name = t.test_run_name and tr.date = t.test_run_date
    group by tr.name, tr.date
),
TestRunCnts(name, date, test_cnt, ran_cnt, passed_cnt, failed_cnt, skipped_cnt) as
(
    select name, date, sum(test_cnt), sum(ran_cnt), sum(passed_cnt), sum(failed_cnt), sum(skipped_cnt)
    from (
        select name, date, cnt as test_cnt, 0 as ran_cnt, 0 as passed_cnt, 0 as failed_cnt, 0 as skipped_cnt
        from NrTests
        union all
        select name, date, 0 as test_cnt, cnt as ran_cnt, 0 as passed_cnt, 0 as failed_cnt, 0 as skipped_cnt
        from NrRanTests
        union all
        select name, date, 0 as test_cnt, 0 as ran_cnt, cnt as passed_cnt, 0 as failed_cnt, 0 as skipped_cnt
        from NrPassed
        union all
        select name, date, 0 as test_cnt, 0 as ran_cnt, 0 as passed_cnt, cnt as failed_cnt, 0 as skipped_cnt
        from NrFailed
        union all
        select name, date, 0 as test_cnt, 0 as ran_cnt, 0 as passed_cnt, 0 as failed_cnt, cnt as skipped_cnt
        from NrSkipped
    )
    where name not null and date not null
    group by name, date
)
select name, date, test_cnt,
    ran_cnt, case when test_cnt = 0 then 0.0 else (ran_cnt * 1.0 / test_cnt) end as ran_ratio,
    passed_cnt, case when test_cnt = 0 then 0.0 else (passed_cnt * 1.0 / test_cnt) end as passed_ratio,
    failed_cnt, case when test_cnt = 0 then 0.0 else (failed_cnt * 1.0 / test_cnt) end as failed_ratio,
    skipped_cnt, case when test_cnt = 0 then 0.0 else (skipped_cnt * 1.0 / test_cnt) end as skipped_ratio
from TestRunCnts;

create view OverallTestOverview as
select sum(test_cnt) as test_cnt,
    sum(ran_cnt) as ran_cnt, case when sum(test_cnt) = 0 then 0.0 else (sum(ran_cnt) * 1.0 / sum(test_cnt)) end as ran_ratio,
    sum(passed_cnt) as passed_cnt, case when sum(test_cnt) = 0 then 0.0 else (sum(passed_cnt) * 1.0 / sum(test_cnt)) end as passed_ratio,
    sum(failed_cnt) as failed_cnt, case when sum(test_cnt) = 0 then 0.0 else (sum(failed_cnt) * 1.0 / sum(test_cnt)) end as failed_ratio,
    sum(skipped_cnt) as skipped_cnt, case when sum(test_cnt) = 0 then 0.0 else (sum(skipped_cnt) * 1.0 / sum(test_cnt)) end as skipped_ratio
from TestRunOverview;

create view ManuallyVerifiedRequirements as
select r.id, r.origin, r.annotation from Requirements r, ManuallyVerified m
where r.id = m.req_id;
