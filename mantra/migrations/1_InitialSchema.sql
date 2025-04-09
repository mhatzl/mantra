-- requirements that may be traced.
-- the data field may contain custom JSON data
create table Requirements (
    id text not null primary key,
    content_hash blob not null,
    last_modified_at text not null,
    last_checked_at text not null,
    title text not null,
    origin text not null,
    data text,
    manual bool not null,
    deprecated bool not null,
    constraint ch_mdf_times check(last_modified_at <= last_checked_at)
);

-- hierarchy
create table RequirementHierarchies (
    child_id text not null references Requirements(id) on delete cascade,
    parent_id text not null references Requirements(id) on delete cascade,
    primary key (child_id, parent_id)
);

-- information to detect changes to any file that may contain items and/or traces
create table TraceableFiles (
    filepath text not null primary key,
    content_hash blob not null,
    last_modified_at text not null,
    last_checked_at text not null,
    constraint ch_mdf_times check(last_modified_at <= last_checked_at)
);

-- base for all traces to link req traces to items
create table TracedLines (
    filepath text not null references TraceableFiles(filepath) on delete cascade,
    line integer not null,
    primary key (filepath, line)
);

-- traces to requirements
create table DirectReqTraces (
    req_id text not null references Requirements(id) on delete cascade,
    filepath text not null,
    line integer not null,
    primary key (req_id, filepath, line),
    foreign key (filepath, line) references TracedLines(filepath, line) on delete cascade
);

-- Language item such as function, test, struct, enum, class, ...
-- Note: Items are uniquely identifiable by filepath and line number.
-- Due to feature flags or language semantics, idents may be declared multiple times, and are therefore not unique.
create table Items (
    ident text not null,
    filepath text not null references TraceableFiles(filepath) on delete cascade,
    start_line integer not null,
    end_line integer not null,
    primary key (filepath, start_line),
    constraint start_le_end check(start_line <= end_line)
);

create table TestItems (
    filepath text not null,
    start_line integer not null,
    primary key (filepath, start_line),
    foreign key (filepath, start_line) references Items(filepath, start_line) on delete cascade
);

-- Item that is directly traced
-- e.g.
-- #[req(my_req)] ... <- traced line 
-- fn foo() {}    ... <- item start line
create table DirectTracedItems (
    filepath text not null,
    traced_line integer not null,
    item_start_line integer not null,
    primary key (filepath, traced_line, item_start_line),
    foreign key (filepath, item_start_line) references Items(filepath, start_line) on delete cascade,
    foreign key (filepath, traced_line) references TracedLines(filepath, line) on delete cascade
);

create table DirectItemReferences (
    origin_filepath text not null,
    origin_start_line integer not null,
    ref_filepath text not null,
    ref_start_line integer not null,
    primary key (origin_filepath, origin_start_line, ref_filepath, ref_start_line),
    foreign key (origin_filepath, origin_start_line) references Items(filepath, start_line) on delete cascade,
    foreign key (ref_filepath, ref_start_line) references Items(filepath, start_line) on delete cascade
);

-- traces to requirements that were not part of the database when the trace was added.
create table UnrelatedDirectReqTraces (
    req_id text not null,
    filepath text not null,
    line integer not null,
    primary key (req_id, filepath, line),
    foreign key (filepath, line) references TracedLines(filepath, line) on delete cascade
);

-- test runs that executed tests
--
-- NOTE: `nr_of_tests` is the number of expected tests in one run.
-- Meaning, if there are fewer associated tests in the Tests table, not all tests were executed.
create table TestRuns (
    name text not null,
    date text not null,
    content_hash blob not null,
    last_checked_at text not null,
    nr_of_tests integer not null,
    data text,
    logs text,
    primary key (name, date),
    constraint ch_time check(date <= last_checked_at)
);

create table Tests (
    test_run_name text not null,
    test_run_date text not null,
    name text not null,
    primary key (test_run_name, test_run_date, name),
    foreign key (test_run_name, test_run_date) references TestRuns(name, date) on delete cascade
);

create table TestLocations (
    test_run_name text not null,
    test_run_date text not null,
    name text not null,
    filepath text not null,
    line integer not null,
    primary key (test_run_name, test_run_date, name),
    foreign key (test_run_name, test_run_date, name) references Tests(test_run_name, test_run_date, name) on delete cascade,
    foreign key (filepath, line) references TestItems(filepath, start_line)
);

-- tests per test run
create table RunTests (
    test_run_name text not null,
    test_run_date text not null,
    name text not null,
    passed integer not null,
    primary key (test_run_name, test_run_date, name),
    foreign key (test_run_name, test_run_date, name) references Tests(test_run_name, test_run_date, name) on delete cascade
);

-- skipped tests
create table SkippedTests (
    test_run_name text not null,
    test_run_date text not null,
    name text not null,
    reason text,
    primary key (test_run_name, test_run_date, name),
    foreign key (test_run_name, test_run_date, name) references Tests(test_run_name, test_run_date, name) on delete cascade
);

create table TestRunStatementCoverage (
    test_run_name text not null,
    test_run_date text not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    primary key (test_run_name, test_run_date, stmnt_filepath, stmnt_line),
    foreign key (test_run_name, test_run_date) references TestRuns(test_run_name, test_run_date) on delete cascade,
    foreign key (stmnt_filepath) references TraceableFiles(filepath)
);

create table TestStatementCoverage (
    test_run_name text not null,
    test_run_date text not null,
    test_name text not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    primary key (test_run_name, test_run_date, test_name, stmnt_filepath, stmnt_line),
    foreign key (test_run_name, test_run_date, test_name) references RunTests(test_run_name, test_run_date, name) on delete cascade,
    foreign key (stmnt_filepath) references TraceableFiles(filepath)
);

-- review to add manually verified requirements
create table Reviews (
    name text not null,
    date text not null,
    content_hash blob not null,
    last_checked_at text not null,
    reviewer text not null,
    comment text,
    primary key (name, date),
    constraint ch_time check(date <= last_checked_at)
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

-- manually verified requirements
create table UnrelatedManuallyVerified (
    req_id text not null,
    review_name text not null,    
    review_date text not null,
    comment text,
    primary key (req_id, review_name, review_date),
    foreign key (review_name, review_date) references Reviews(name, date) on delete cascade
);

create table TestOverrides (
    test_run_name text not null,
    test_run_date text not null,
    test_name text not null,
    review_name text not null,    
    review_date text not null,
    -- 0=failed; 1=passed; 2=skipped 
    state integer not null,
    comment text,
    primary key (test_run_name, test_run_date, test_name, review_name, review_date),
    foreign key (review_name, review_date) references Reviews(name, date) on delete cascade,
    foreign key (test_run_name, test_run_date, test_name) references Tests(test_run_name, test_run_date, name)
);

create table StatementCoverageOverrides (
    test_run_name text not null,
    test_run_date text not null,
    review_name text not null,    
    review_date text not null,
    stmnt_filepath text not null,
    stmnt_line text not null,
    hits integer not null,
    comment text,
    primary key (test_run_name, test_run_date, test_name, review_name, review_date),
    foreign key (review_name, review_date) references Reviews(name, date) on delete cascade,
    foreign key (test_run_name, test_run_date) references TestRuns(test_run_name, test_run_date),
    foreign key (test_run_name, test_run_date, stmnt_filepath, stmnt_line) references TestRunStatementCoverage(test_run_name, test_run_date, stmnt_filepath, stmnt_line)
);
