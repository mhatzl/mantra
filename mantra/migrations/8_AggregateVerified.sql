-- Contains covered lines mapped to traces that are only covered by passed test runs
create table TraceMappedLinesOnlyCoveredByPassedTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(collect_nr, product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are only covered by passed test cases
create table TraceMappedLinesOnlyCoveredByPassedTestCases (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(collect_nr, product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesOnlyCoveredByPassedTestRuns
-- and TraceMappedLinesOnlyCoveredByPassedTestCases
create table TraceMappedLinesOnlyCoveredByPassedTests (
    collect_nr integer not null,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (collect_nr, product_id, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, filepath) references ProductRelatedFiles (collect_nr, product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have no mapped line that was covered by a failed test.
create table TracesOnlyCoveredByPassedTests (
    collect_nr integer not null,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (collect_nr, product_id, filepath, traced_line),
    foreign key (collect_nr, product_id, filepath) references ProductRelatedFiles (collect_nr, product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are covered by failed test runs
-- Note: may also be covered by passed test runs, but at least one failed test run
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestRuns (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, test_run_name, test_run_date, filepath, cov_line)
        references TestRunLineCoverage(collect_nr, product_id, test_run_name, test_run_date, cov_filepath, cov_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains covered lines mapped to traces that are covered by failed test cases
-- Note: may also be covered by passed test cases, but at least one failed test case
-- also covered the line.
create table TraceMappedLinesCoveredByFailedTestCases (
    collect_nr integer not null,
    product_id text not null,
    test_run_name text not null,
    test_run_date text not null,
    test_case_name text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, test_run_name, test_run_date, test_case_name, filepath, cov_line)
        references TestCaseLineCoverage(collect_nr, product_id, test_run_name, test_run_date, test_case_name, cov_filepath, cov_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Table combining TraceMappedLinesCoveredByFailedTestRuns
-- and TraceMappedLinesCoveredByFailedTestCases
create table TraceMappedLinesCoveredByFailedTests (
    collect_nr integer not null,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    cov_line integer not null,
    primary key (collect_nr, product_id, filepath, traced_line, cov_line),
    foreign key (collect_nr, product_id, filepath) references ProductRelatedFiles (collect_nr, product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains traces that have at least one linked line that was covered by a failed test.
create table TracesCoveredByFailedTests (
    collect_nr integer not null,
    product_id text not null,
    filepath text not null,
    file_hash text not null,
    traced_line integer not null,
    primary key (collect_nr, product_id, filepath, traced_line),
    foreign key (collect_nr, product_id, filepath) references ProductRelatedFiles (collect_nr, product_id, filepath) on delete cascade,
    foreign key (file_hash, traced_line) references Traces(file_hash, line) on delete cascade
);

-- Contains direct verification states of requirements
--
-- **Note:** Direct means that the state is indipendent of the state of related requirements.
-- [req_clarify("req.state")]
create table DirectRequirementVerificationStates (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    -- 0=failed; 1=verified; 2=skipped; 3=unverified
    state integer not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, req_id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table UsableLeafRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table UsableNonLeafRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementsWithOnlyOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementsWithUnverifiedNonOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementsWithSkippedNonOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Note: May contain multiple states per requirement
create table StatesOfRequirementsWithOnlyOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    state integer not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id, state),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table VerifiedRequirementsWithOnlyOptionalChildren (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains verification states for requirements based on the requirement hierarchy.
-- This table only contains non-leaf requirements (requirements that have at least one child).
create table IndirectRequirementVerificationStates (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr,
    product_id text not null,
    req_id text not null,
    state integer not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

create table RequirementVerificationStates (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    state integer not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains requirements that are successfully verified.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is verified.
create table VerifiedRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains requirements that are skipped.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is skipped.
create table SkippedRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains requirements that are skipped.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is failed.
create table FailedRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);

-- Contains requirements that are unverified.
-- For leaf requirements, this means the state in DirectRequirementVerificationStates is unverified.
create table UnverifiedRequirements (
    agg_collect_nr integer not null references Collections (nr) on delete restrict,
    req_collect_nr integer not null,
    product_id text not null,
    req_id text not null,
    primary key (agg_collect_nr, req_collect_nr, product_id, req_id),
    foreign key (req_collect_nr, product_id, req_id) references Requirements(collect_nr, product_id, id) on delete cascade,
    constraint agg_after_collect check (req_collect_nr <= agg_collect_nr)
);
