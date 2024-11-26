
create view RequirementDescendants as
with recursive TransitiveChildren(id, descendant_id) as
(
    select parent_id, child_id from RequirementHierarchies
    union all
    select tc.id, rh.child_id from RequirementHierarchies rh, TransitiveChildren tc
    where tc.descendant_id = rh.parent_id
)
select distinct id, descendant_id from TransitiveChildren;

-- Requirements without children
create view LeafRequirements as
select distinct id
from Requirements
where id not in (select parent_id from RequirementHierarchies);

create view NonLeafRequirements as
select id
from Requirements
except
select id
from LeafRequirements;

create view DeprecatedRequirements as
with MarkedDeprecated(id) as (
    select id from Requirements
    where deprecated = true
),
ParentMarkedDeprecated(id) as (
    select rc.descendant_id
    from RequirementDescendants rc, MarkedDeprecated md
    where rc.id = md.id
),
Deprecated(id) as (
    select id from MarkedDeprecated
    union
    select id from ParentMarkedDeprecated
)
select distinct id from Deprecated;

create view ManualRequirements as
with MarkedManual(id) as (
    select id from Requirements
    where manual = true
),
ParentMarkedManual(id) as (
    select rc.descendant_id
    from RequirementDescendants rc, MarkedManual md
    where rc.id = md.id
),
Manual(id) as (
    select id from MarkedManual
    union
    select id from ParentMarkedManual
)
select distinct id from Manual;

create view DirectlyTracedRequirements as
select distinct r.id from Requirements r, ReqTraces tr
where r.id = tr.req_id;

create view UntracedRequirements as
with recursive IsUntraced(id) as (
    -- Leaf requirements cannot be traced indirectly
    select id
    from LeafRequirements
    where id not in (select id from DirectlyTracedRequirements)
    union all
    -- Recursively get requirements that are not directly traced,
    -- or have at least one untraced child
    select r.id
    from (
		select id from NonLeafRequirements except select id from DirectlyTracedRequirements
	) r, RequirementHierarchies rh, IsUntraced u
    where r.id = rh.parent_id
    and rh.child_id = u.id
)
select distinct id from IsUntraced;

-- A requirement is indirectly traced
-- if **all** of its direct child requirements are either directly or indirectly traced.
create view IndirectlyTracedRequirements as
with HasUntracedChild(id) as (
    select rh.parent_id
    from RequirementHierarchies rh, UntracedRequirements u
    where rh.child_id = u.id
)
-- Only non-leaf requirements can be indirectly traced
select distinct id
from NonLeafRequirements
where id not in (select id from HasUntracedChild);

-- Traces to child requirements.
create view IndirectRequirementTraces as
select ir.id, c.descendant_id as traced_id, t.filepath, t.line
from IndirectlyTracedRequirements ir, RequirementDescendants c, ReqTraces t
where ir.id = c.id and c.descendant_id = t.req_id;

create view IndirectTraceTree as
with CompactTraceEntry(id, traced_id, trace) as (
	select id, traced_id, json_object('filepath', filepath, 'line', line)
	from IndirectRequirementTraces
), GroupedTraceEntry(id, traced_id, trace_list) as (
	select id, traced_id, '[' || group_concat(json(trace)) || ']'
	from CompactTraceEntry
	group by id, traced_id
)
select id, traced_id, json(trace_list) as traces
from GroupedTraceEntry;

create view TracedRequirements as
select id from DirectlyTracedRequirements
union
select id from IndirectlyTracedRequirements;

-- A requirement is fully traced if all its leaf requirements are traced.
-- Consequently, leaf requirements are fully traced if they are traced.
create view FullyTracedRequirements as
with HasUntracedLeaf(id) as (
    select rc.id
    from RequirementDescendants rc, LeafRequirements lr, UntracedRequirements ur
    where rc.descendant_id = lr.id and lr.id = ur.id
)
select lr.id
from LeafRequirements lr, DirectlyTracedRequirements dr
where lr.id = dr.id
union all
select id
from NonLeafRequirements
where id not in (select id from HasUntracedLeaf);

create view InvalidRequirements as
select d.id
from DeprecatedRequirements d, TracedRequirements t
where d.id = t.id;

create view ItemReferences as
with recursive TransitiveReferences(
    item_filepath, item_start_line,
    ref_item_filepath, ref_item_start_line, ref_item_end_line
) as (
    select
        dr.origin_filepath, dr.origin_start_line,
        dr.ref_filepath, dr.ref_start_line, i.end_line
    from DirectItemReferences dr, Items i
    where i.filepath = dr.ref_filepath and i.start_line = dr.ref_start_line
    union all
    select
        dr.origin_filepath, dr.origin_start_line,
        tr.ref_item_filepath, tr.ref_item_start_line, i.end_line
    from DirectItemReferences dr, TransitiveReferences tr, Items i
    where dr.ref_filepath = tr.item_filepath
        and dr.origin_start_line = tr.item_start_line
        and tr.ref_item_filepath = i.filepath
        and tr.ref_item_start_line = i.start_line
)
select distinct
    tr.item_filepath, tr.item_start_line, i.end_line,
    tr.ref_item_filepath, tr.ref_item_start_line, tr.ref_item_end_line
from TransitiveReferences tr, Items i
where tr.item_filepath = i.filepath and tr.item_start_line = i.start_line;

-- TODO
create view DirectStaticRequirementTestCoverage as
select distinct
    req_id, test_name, test_filepath, test_line
from ReqTraces rt, Items i, TestItems ti, TracedItems tr
where 

-- TODO
create view IndirectStaticRequirementTestCoverage as
select distinct
    req_id, test_name, test_filepath, test_line,
    covered_item_filepath, covered_item_line, covered_item_name
from 

create view DirectlyCoveredRequirements as
select id from Requirements
where id in (select req_id from TestCoverage);

create view DirectRequirementCoverage as
select v.req_id as id, v.test_run_name, v.test_run_date, v.test_name,
v.trace_filepath, v.trace_line, coalesce(t.passed, 0) as test_passed
from TestCoverage v, Tests t
where v.test_run_name = t.test_run_name and v.test_run_date = t.test_run_date
and v.test_name = t.name;


create view UncoveredRequirements as
with recursive IsUncovered(id) as (
    -- Leaf requirements cannot be covered indirectly
    select id
    from LeafRequirements
    where id not in (select id from DirectlyCoveredRequirements)
    union all
    -- Recursively get requirements that are not directly covered,
    -- or have at least one uncovered child
    select r.id
    from (
        select id from NonLeafRequirements except select id from DirectlyCoveredRequirements
    ) r, RequirementHierarchies rh, IsIndirectlyUncovered u
    where r.id = rh.parent_id
    and rh.child_id = u.id
)
select distinct id from IsUncovered;

-- Indirectly covered requirements have the same constraint
-- as indirectly traced requirements.
--
-- See description for indirectly traced requirements for more information.
create view IndirectlyCoveredRequirements as
with HasUncoveredChild(id) as (
    select rh.parent_id
    from RequirementHierarchies rh, UncoveredRequirements u
    where rh.child_id = u.id
)
-- Only non-leaf requirements can be indirectly uncovered
select distinct id
from NonLeafRequirements
where id not in (select id from HasUncoveredChild);

-- Test coverage of child requirements.
create view IndirectRequirementTestCoverage as
select r.id, c.descendant_id as covered_id,
v.test_run_name, v.test_run_date, v.test_name,
v.trace_filepath, v.trace_line,
coalesce(t.passed, 0) as test_passed
from IndirectlyCoveredRequirements r, RequirementDescendants c, TestCoverage v, Tests t
where r.id = c.id and c.descendant_id = v.req_id
and v.test_run_name = t.test_run_name and v.test_run_date = t.test_run_date
and v.test_name = t.name;

create view CoveredRequirements as
select id from DirectlyCoveredRequirements
union
select id from IndirectlyCoveredRequirements;

-- Coverage of a requirement failed if either one of the following holds:
--
-- - one of the tests failed that directly covered the requirement
-- - one of the child requirements has failed coverage
create view FailedCoveredRequirements as
with HasFailedChild(id, covered_id) as (
    select r.id, rc.descendant_id from Requirements r, RequirementDescendants rc, FailedTestCoverage f
    where r.id = rc.id and rc.descendant_id = f.req_id
)
select c.id, hf.covered_id
from CoveredRequirements c, HasFailedChild hf
where c.id = hf.id
union all
select c.id, null as covered_id
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

create view PassedCoveredRequirements as
select id from CoveredRequirements
except
select id from FailedCoveredRequirements;

-- A requirement is fully covered if all its leaf requirements are passed covered.
-- Consequently, leaf requirements are fully covered if they are passed covered.
create view FullyCoveredRequirements as
with HasUncoveredOrFailedLeaf(id) as (
    select rc.id
    from RequirementDescendants rc, LeafRequirements lr, UncoveredRequirements ur
    where rc.descendant_id = lr.id and lr.id = ur.id
    union all
    select rc.id
    from RequirementDescendants rc, LeafRequirements lr, FailedCoveredRequirements fr
    where rc.descendant_id = lr.id and lr.id = fr.id
)
select lr.id
from LeafRequirements lr, PassedCoveredRequirements pr
where lr.id = pr.id
union all
select id
from NonLeafRequirements
where id not in (select id from HasUncoveredOrFailedLeaf);

create view PassedTests as
select test_run_name, test_run_date, name, filepath, line
from Tests
where passed = 1;

create view FailedTestCoverage as
select tc.req_id, tc.test_run_name, tc.test_run_date, tc.test_name, tc.trace_filepath, tc.trace_line
from TestCoverage tc, Tests t
where tc.test_run_name = t.test_run_name and tc.test_run_date = t.test_run_date
    and tc.test_name = t.name and (t.passed <> 1 or t.passed is null);

create view ManuallyVerifiedRequirements as
select req_id from ManuallyVerified;
