
-- requirements
create table if not exists Requirements (
    id text primary key,
    origin text not null
);

-- hierarchy
create table if not exists RequirementHierarchies (
    child_id text not null references Requirements(id),
    parent_id text not null references Requirements(id),
    primary key (child_id, parent_id)
);

-- traces
create table if not exists Traces (
    req_id text not null references Requirements(id),
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
    foreign key (test_run_name, test_run_date) references TestRuns(name, date)
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
    foreign key (test_run_name, test_run_date, test_name) references Tests(test_run_name, test_run_date, name),
    foreign key (req_id, filepath, line) references Traces(req_id, filepath, line)
);

-- deprecated requirements
create table if not exists DeprecatedRequirements (
    req_id text not null references Requirements(id),
    primary key (req_id)
);

-- requirements that require manual review
create table if not exists ManualRequirements (
    req_id text not null references Requirements(id),
    primary key (req_id)
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
    req_id text not null references ManualRequirements(req_id),
    review_name text not null,    
    review_date text not null,
    comment text,
    primary key (req_id, review_name, review_date),
    foreign key (review_name, review_date) references Reviews(name, date)
);
