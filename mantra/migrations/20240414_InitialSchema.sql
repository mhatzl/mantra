
-- requirements that may be traced.
-- generation is used to show changes for "--dry-run" and to delete non-existing requirements.
-- annotation might be "manual" or "deprecated"
create table if not exists Requirements (
    id text not null primary key,
    generation integer not null,
    origin text not null,
    annotation text
);

-- hierarchy
create table if not exists RequirementHierarchies (
    child_id text not null references Requirements(id) on delete cascade,
    parent_id text not null references Requirements(id) on delete cascade,
    primary key (child_id, parent_id)
);

-- traces to requirements
-- generation is used to show changes for "--dry-run" and to delete non-existing traces.
create table if not exists Traces (
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
create table if not exists TestRuns (
    name text not null,
    date text not null,
    nr_of_tests integer,
    logs text,
    primary key (name, date)
);

-- tests per test run
--
-- NOTE: 'passed = null' means the test is still running, or was not finished properly.
create table if not exists Tests (
    test_run_name text not null,
    test_run_date text not null,
    name text not null,
    filepath text not null,
    line integer not null,
    passed integer,
    primary key (test_run_name, test_run_date, name),
    foreign key (test_run_name, test_run_date) references TestRuns(name, date) on delete cascade
);

-- coverage data per test
create table if not exists TestCoverage (
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
create table if not exists Reviews (
    name text not null,
    date text not null,
    reviewer text not null,
    comment text,
    primary key (name, date)
);

-- manually verified requirements
create table if not exists ManuallyVerified (
    req_id text not null references Requirements(id) on delete cascade,
    review_name text not null,    
    review_date text not null,
    comment text,
    primary key (req_id, review_name, review_date),
    foreign key (review_name, review_date) references Reviews(name, date) on delete cascade
);
