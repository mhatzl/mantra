
create view DirectCoverageTree as
with CompactTraceEntry(id, test_run_name, test_run_date, test_name, test_passed, trace) as (
	select id, test_run_name, test_run_date, test_name, test_passed,
	json_object('filepath', trace_filepath, 'line', trace_line)
	from DirectRequirementCoverage
), GroupedTraceEntry(id, test_run_name, test_run_date, test_name, test_passed, trace_list) as (
	select id, test_run_name, test_run_date, test_name, test_passed, '[' || group_concat(trace) || ']'
	from CompactTraceEntry
	group by id, test_run_name, test_run_date, test_name
), CompactTestEntry(id, test_run_name, test_run_date, test) as (
	select id, test_run_name, test_run_date,
	json_object('name', test_name, 'passed', case when test_passed = 1 then json('true') else json('false') end, 'traces', json(trace_list))
	from GroupedTraceEntry
), GroupedTestEntry(id, test_run_name, test_run_date, test_list) as (
	select id, test_run_name, test_run_date, '[' || group_concat(test) || ']'
	from CompactTestEntry
	group by id, test_run_name, test_run_date
)
select id, test_run_name, test_run_date, json(test_list) as tests from GroupedTestEntry;

-- Groups all coverage information by id and covered_id
-- creating a JSON tree of covered_id->test_runs->tests->traces 
create view IndirectTestCoverageTree as
with CompactTraceEntry(id, covered_id, test_run_name, test_run_date, test_name, test_passed, trace) as (
	select id, covered_id, test_run_name, test_run_date, test_name, test_passed,
	json_object('filepath', trace_filepath, 'line', trace_line)
	from IndirectRequirementTestCoverage
), GroupedTraceEntry(id, covered_id, test_run_name, test_run_date, test_name, test_passed, trace_list) as (
	select id, covered_id, test_run_name, test_run_date, test_name, test_passed, '[' || group_concat(trace) || ']'
	from CompactTraceEntry
	group by id, covered_id, test_run_name, test_run_date, test_name
), CompactTestEntry(id, covered_id, test_run_name, test_run_date, test) as (
	select id, covered_id, test_run_name, test_run_date,
	json_object('name', test_name, 'passed', case when test_passed = 1 then json('true') else json('false') end, 'traces', json(trace_list))
	from GroupedTraceEntry
), GroupedTestEntry(id, covered_id, test_run_name, test_run_date, test_list) as (
	select id, covered_id, test_run_name, test_run_date, '[' || group_concat(test) || ']'
	from CompactTestEntry
	group by id, covered_id, test_run_name, test_run_date
), CompactTestRunEntry(id, covered_id, test_run) as (
	select id, covered_id,
	json_object('name', test_run_name, 'date', test_run_date, 'tests', json(test_list))
	from GroupedTestEntry
), GroupedTestRunEntry(id, covered_id, test_run_list) as (
	select id, covered_id, '[' || group_concat(test_run) || ']'
	from CompactTestRunEntry
	group by id, covered_id
)
select id, covered_id, json(test_run_list) as test_runs from GroupedTestRunEntry;

create view RequirementCoverageOverview as
with NrRequirements(cnt) as (select count(*) from Requirements),
NrTraced(cnt) as (select count(*) from TracedRequirements),
NrCovered(cnt) as (select count(*) from CoveredRequirements),
NrPassed(cnt) as (select count(*) from PassedCoveredRequirements),
VerifiedOverview(cnt, ratio) as (
    -- Only consider manual requirements for verified cnt and ratio
    select 
        case when m.nr_manuals = 0 then null else c.cnt end as cnt,
        case when m.nr_manuals = 0 then 0.0 else (c.cnt * 1.0 / m.nr_manuals) end as ratio
    from (
        select count(*) as cnt
        from ManuallyVerifiedRequirements m, ManualRequirements r
        where m.req_id = r.id
    ) as c, (
        select count(*) as nr_manuals
        from ManualRequirements
    ) as m
)
select r.cnt as req_cnt, t.cnt as traced_cnt, case when r.cnt = 0 then 0.0 else (t.cnt * 1.0 / r.cnt) end as traced_ratio,
    c.cnt as covered_cnt, case when r.cnt = 0 then 0.0 else (c.cnt * 1.0 / r.cnt) end as covered_ratio,
    p.cnt as passed_cnt, case when r.cnt = 0 then 0.0 else (p.cnt * 1.0 / r.cnt) end as passed_ratio,
    v.cnt as verified_cnt, v.ratio as verified_ratio
from NrRequirements r, NrTraced t, NrCovered c, NrPassed p, VerifiedOverview v;

create view LeafChildOverview as
with NrLeafs(id, cnt) as (
    select rc.id, count(*)
    from RequirementDescendants rc, LeafRequirements lr
    where rc.descendant_id = lr.id
    group by rc.id
), NrTracedLeafs(id, cnt) as (
    select rc.id, count(*)
    from RequirementDescendants rc, LeafRequirements lr, DirectlyTracedRequirements dt
    where rc.descendant_id = lr.id and lr.id = dt.id
    group by rc.id
), NrCoveredLeafs(id, cnt) as (
    select rc.id, count(*)
    from RequirementDescendants rc, LeafRequirements lr, DirectlyCoveredRequirements dc
    where rc.descendant_id = lr.id and lr.id = dc.id
    group by rc.id
), NrPassedCoveredLeafs(id, cnt) as (
    select rc.id, count(*)
    from RequirementDescendants rc, LeafRequirements lr, PassedCoveredRequirements pc
    where rc.descendant_id = lr.id and lr.id = pc.id
    group by rc.id
)
select id, sum(leaf_cnt) as leaf_cnt,
sum(traced_leaf_cnt) as traced_leaf_cnt, case when sum(leaf_cnt) = 0 then 0.0 else (sum(traced_leaf_cnt)  * 1.0 / sum(leaf_cnt)) end as traced_leaf_ratio,
sum(covered_leaf_cnt) as covered_leaf_cnt, case when sum(leaf_cnt) = 0 then 0.0 else (sum(covered_leaf_cnt) * 1.0 / sum(leaf_cnt)) end as covered_leaf_ratio,
sum(passed_covered_leaf_cnt) as passed_covered_leaf_cnt, case when sum(leaf_cnt) = 0 then 0.0 else (sum(passed_covered_leaf_cnt) * 1.0 / sum(leaf_cnt)) end as passed_covered_leaf_ratio
from (
    select id, cnt as leaf_cnt, 0 as traced_leaf_cnt, 0 as covered_leaf_cnt, 0 as passed_covered_leaf_cnt
    from NrLeafs
    union all
    select id, 0 as leaf_cnt, cnt as traced_leaf_cnt, 0 as covered_leaf_cnt, 0 as passed_covered_leaf_cnt
    from NrTracedLeafs
    union all
    select id, 0 as leaf_cnt, 0 as traced_leaf_cnt, cnt as covered_leaf_cnt, 0 as passed_covered_leaf_cnt
    from NrCoveredLeafs
    union all
    select id, 0 as leaf_cnt, 0 as traced_leaf_cnt, 0 as covered_leaf_cnt, cnt as passed_covered_leaf_cnt
    from NrPassedCoveredLeafs
)
group by id;

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
