
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
    foreign key (test_name, project_name) references Tests(name, project_name)
);

-- tests per project
create table if not exists Tests (
    name text not null,
    project_name text not null references Projects(name),
    filepath text not null,
    line integer not null,
    primary key (name, project_name)
);

-- deprecated requirements
create table if not exists DeprecatedRequirements (
    req_id text not null references Requirements(id),
    project_name text not null references Projects(name),
    primary key (req_id, project_name)
);

-- manually traced requirements
create table if not exists UntraceableRequirements (
    req_id text not null references Requirements(id),
    project_name text not null references Projects(name),
    primary key (req_id, project_name)
);
