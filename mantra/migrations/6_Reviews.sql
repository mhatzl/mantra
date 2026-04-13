
-- Table to store reviews.
-- [req("review", "changes.track")]
create table Reviews (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
   -- The product ID that maps to the product that got reviewed.
    product_id text not null references Products(id) on delete cascade,
    -- Name of the review
    name text not null,
    -- UTC date and time at which the review was held.
    utc_date text not null,
    -- Optional origin data of the review that was set for multiple reviews.
    -- [req("review.origin")]
    base_origin_hash text references GeneralJson (hash) on delete restrict,
    -- The hash of the origin data of the review.
    -- [req("review.origin")]
    origin_hash text references GeneralJson (hash) on delete restrict,
    -- Hash of the optional decription for the review.
    -- [req("review.description")]
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- The hash of the data the review was collected from to detect changes.
    data_hash text not null,
    -- Filepath the data was collected from
    data_filepath text not null,
    primary key (product_id, name, utc_date),
    foreign key (product_id, data_filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade
);

-- Table to store authors of a review.
-- [req("review.authors")]
create table ReviewAuthors (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    review_name text not null,
    review_date text not null,
    author text not null,
    primary key (product_id, review_name, review_date, author),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade
);

-- Table to store optional metadata of a review.
create table ReviewProperties (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    review_name text not null,
    review_date text not null,
    property_key text not null,
    value_hash text references GeneralJson (hash) on delete restrict,
    primary key (product_id, review_name, review_date, property_key),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade
);

create table ReviewRevisions (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    review_name text not null,
    review_date text not null,
    -- Indicates the revision
    revision integer not null,
    -- Comment for the revision.
    -- [req("changes.comment")]
    comment text not null,
    primary key (product_id, review_name, review_date, revision),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade
);

-- Names of authors of a review revision.
-- [req("changes.authors")]
create table ReviewRevisionAuthors (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    review_name text not null,
    review_date text not null,
    -- Indicates the revision
    revision integer not null,
    -- Names of an author of the revision.
    -- [req("changes.authors")]
    author text not null,
    primary key (product_id, review_name, review_date, revision, author),
    foreign key (product_id, review_name, review_date, revision) references ReviewRevisions (product_id, review_name, review_date, revision) on delete cascade
);

-- Table to store requirement IDs that were manually verified in a review,
-- and the IDs could be mapped to requirements stored in the database.
-- [req("review.verify_req")]
create table ManuallyVerifiedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
   -- ID of the requirement that is manually verified.
    req_id text not null,
    -- Product ID that maps to the product that got reviewed.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- Hash of the comment for the manual verification.
    comment_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (
        product_id,
        req_id,
        review_name,
        review_date
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade
);

-- Table to store test case overrides from reviews.
-- [req("review.test_case_state")]
create table TestCaseOverrides (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
   -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- State that must be used instead of the one stored in the TestCase table.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed
    state integer not null,
    -- Hash of the comment explaining why the state must be overriden.
    comment_hash text not null references GeneralTexts(hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        review_name,
        review_date
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        test_run_date,
        name
    ),
    -- ensure the test run happened before the review
    constraint future_date check (unixepoch(test_run_date) < unixepoch(review_date))
);

-- Table to store overrides from reviews for line coverage entries of test runs.
--
-- **Note:** No file hash needed, because the related coverage entry is either in the tracked or untracked table.
--
-- [req("review.coverage", "testcov.cov.lines")]
create table TestRunLineCoverageOverrides (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- File that was covered.
    cov_filepath text not null,
    -- Line that was covered.
    cov_line integer not null,
    -- Number of how often the line was covered/hit during test run execution.
    -- If null, the line is ignored from line coverage analysis for this test run.
    hits integer,
    -- Hash of the comment explaining why this line coverage must be overriden.
    comment_hash text not null references GeneralTexts (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        review_name,
        review_date,
        cov_filepath,
        cov_line
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        cov_filepath,
        cov_line
    ) references TestRunLineCoverage (
        product_id,
        test_run_name,
        test_run_date,
        cov_filepath,
        cov_line
    ),
    -- ensure the test run happened before the review
    constraint future_date check (unixepoch(test_run_date) < unixepoch(review_date))
);

-- Table to store overrides from reviews for line coverage entries of test cases.
--
-- **Note:** No file hash needed, because the related coverage entry is either in the tracked or untracked table.
--
-- [req("review.coverage", "testcov.cov.lines")]
create table TestCaseLineCoverageOverrides (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
   -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- UTC date and time of the test run.
    test_run_date text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- Name of the review.
    review_name text not null,
    -- UTC date and time at which the review was held.
    review_date text not null,
    -- File that was covered.
    cov_filepath text not null,
    -- Line that was covered.
    cov_line integer not null,
    -- Number of how often the line was covered/hit during test run execution.
    -- If null, the line is ignored from line coverage analysis for this test case.
    hits integer,
    -- Hash of the comment explaining why this line coverage must be overriden.
    comment_hash text not null references GeneralTexts (hash) on delete cascade,
    primary key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        review_name,
        review_date,
        cov_filepath,
        cov_line
    ),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        cov_filepath,
        cov_line
    ) references TestCaseLineCoverage (
        product_id,
        test_run_name,
        test_run_date,
        test_case_name,
        cov_filepath,
        cov_line
    ),
    -- ensure the test run happened before the review
    constraint future_date check (unixepoch(test_run_date) < unixepoch(review_date))
);

-- Contains collected entries that could not be mapped to existing data.
-- e.g. verified requirements, test case state or code coverage overrides
create table IgnoredReviewEntries (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    review_name text not null,
    review_date text not null,
    -- Hash of the content of the entry in the review that got ignored.
    entry_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (product_id, review_name, review_date, entry_hash),
    foreign key (product_id, review_name, review_date) references Reviews (product_id, name, utc_date) on delete cascade
);
