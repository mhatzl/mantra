
-- requirements
create table if not exists Requirements (
    id text primary key,
    origin text not null
);

-- projects, repositories, or branches for which trace data may be collected
create table if not exists Projects (
    name text primary key,
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
    project_name text not null references Projects(name),
    filepath text not null,
    line integer not null,
    primary key (req_id, project_name, filepath, line)
);

-- coverage data per test
create table if not exists Coverage (
    req_id text not null references Requirements(id),
    project_name text not null references Projects(name),
    test_name text not null,
    filepath text not null,
    line integer not null,
    primary key (req_id, project_name, test_name, filepath, line),
    foreign key (test_name, project_name) references Tests(name, project_name),
    foreign key (req_id, project_name, filepath, line) references Traces(req_id, project_name, filepath, line)
);

-- tests per project
--
-- NOTE: 'passed = null' means the test is still running, or was not finished properly.
create table if not exists Tests (
    name text not null,
    project_name text not null references Projects(name),
    filepath text not null,
    line integer not null,
    passed integer,
    primary key (name, project_name)
);

-- deprecated requirements
create table if not exists DeprecatedRequirements (
    req_id text not null references Requirements(id),
    project_name text not null references Projects(name),
    primary key (req_id, project_name)
);

-- untraceable requirements that require manual review
create table if not exists UntraceableRequirements (
    req_id text not null references Requirements(id),
    project_name text not null references Projects(name),
    primary key (req_id, project_name)
);

-- review to add manually verified requirements
create table if not exists Review (
    project_name text not null references Projects(name),
    name text not null,
    date text not null,
    reviewer text not null,
    comment text,
    primary key (name, project_name, date)
);

-- manually verified requirements
create table if not exists ManuallyVerified (
    req_id text not null references Requirements(id),
    project_name text not null,
    review_name text not null,    
    review_date text not null,
    comment text,
    foreign key (review_name, review_date, project_name) references Review(name, date, project_name)
);
