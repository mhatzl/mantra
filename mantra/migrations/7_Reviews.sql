
create table SchemaReviewProperties (
    schema_hash text not null references Schemas (content_hash) on delete cascade,
    property_key text not null,
    value_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (schema_hash, property_key)
);

create table ReviewCollectConfigs (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    cfg_entry integer not null,
    path text not null,
    source_hash text not null references GeneralJson (hash) on delete restrict,
    origin_hash text references GeneralJson (hash) on delete restrict,
    pattern text,
    primary key (product_id, cfg_entry),
    foreign key (product_id, cfg_entry) references CollectConfigs (product_id, entry_number) on delete restrict
);

create table ConfigReviewProperties (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    cfg_entry integer not null,
    property_key text not null,
    value_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (product_id, cfg_entry, property_key),
    foreign key (product_id, cfg_entry) references CollectConfigs (product_id, entry_number) on delete restrict
);

-- Table to store reviews.
-- [req("review", "changes.track")]
create table Reviews (
    latest_collection integer not null references Collections (id) on delete cascade,
   -- The product ID that maps to the product that got reviewed.
    product_id text not null,
    -- Name of the review
    name text not null,
    -- UTC date and time at which the review was held.
    utc_date text not null,
    -- The hash of the origin data of the review.
    -- [req("review.origin")]
    origin_hash text references GeneralJson (hash) on delete restrict,
    -- Hash of the optional decription for the review.
    -- [req("review.description")]
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- The hash of the data the review was collected from to detect changes.
    data_hash text not null,
    -- Schema the data was collected from
    schema_hash text not null references Schemas (content_hash) on delete restrict,
    -- Optional MIME/media type of review related general texts (e.g. description).
    media_type text,
    primary key (product_id, name),
    foreign key (product_id) references Products (id) on delete cascade
);

-- Table to store authors of a review.
-- [req("review.authors")]
create table ReviewAuthors (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    review_name text not null,
    author text not null,
    primary key (product_id, review_name, author),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade
);

-- Table to store optional metadata of a review.
create table ReviewProperties (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    review_name text not null,
    property_key text not null,
    value_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (product_id, review_name, property_key),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade
);

create table ReviewRevisions (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    review_name text not null,
    -- Indicates the revision
    revision integer not null,
    -- Comment for the revision.
    -- [req("changes.comment")]
    comment text not null,
    primary key (product_id, review_name, revision),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade
);

-- Names of authors of a review revision.
-- [req("changes.authors")]
create table ReviewRevisionAuthors (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    review_name text not null,
    -- Indicates the revision
    revision integer not null,
    -- Names of an author of the revision.
    -- [req("changes.authors")]
    author text not null,
    primary key (product_id, review_name, revision, author),
    foreign key (product_id, review_name, revision) references ReviewRevisions (product_id, review_name, revision) on delete cascade
);

-- Table to store requirement IDs that were manually verified in a review,
-- and the IDs could be mapped to requirements stored in the database.
-- [req("review.verify_req")]
create table ManuallyVerifiedRequirements (
    latest_collection integer not null references Collections (id) on delete cascade,
    -- Product ID that maps to the product that got reviewed.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
   -- ID of the requirement that is manually verified.
    req_id text not null,
    -- Hash of the comment for the manual verification.
    comment_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (
        product_id,
        review_name,
        req_id
    ),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade,
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade
);

-- Table to store test case overrides from reviews.
-- [req("review.test_case_state")]
create table TestCaseOverrides (
    latest_collection integer not null references Collections (id) on delete cascade,
   -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- State that must be used instead of the one stored in the TestCase table.
    -- 0=failed; 1=passed; 2=skipped; 3=unknown/running/not executed
    state integer not null,
    -- Hash of the comment explaining why the state must be overriden.
    comment_hash text not null references GeneralTexts(hash) on delete cascade,
    primary key (
        product_id,
        review_name,
        test_run_name,
        test_case_name
    ),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_case_name
    ) references TestCases (
        product_id,
        test_run_name,
        name
    )
);

-- Table to store overrides from reviews for line coverage entries of test runs.
--
-- **Note:** No file hash needed, because the related coverage entry is either in the tracked or untracked table.
--
-- [req("review.coverage", "testcov.cov.lines")]
create table TestRunLineCoverageOverrides (
    latest_collection integer not null references Collections (id) on delete cascade,
    -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- File that was covered.
    cov_filepath text not null,
    -- Line that was covered.
    cov_line integer not null,
    -- Indicates if a line is covered `true`, uncovered `false`,
    -- or unknown `null` indicating that the line has no executable code at this line.
    --
    -- If `null`, the line is ignored from line coverage analysis for this test run.
    covered bool,
    -- Hash of the comment explaining why this line coverage must be overriden.
    comment_hash text not null references GeneralTexts (hash) on delete cascade,
    primary key (
        product_id,
        review_name,
        test_run_name,
        cov_filepath,
        cov_line
    ),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        cov_filepath,
        cov_line
    ) references TestRunLineCoverage (
        product_id,
        test_run_name,
        cov_filepath,
        cov_line
    )
);

-- Table to store overrides from reviews for line coverage entries of test cases.
--
-- **Note:** No file hash needed, because the related coverage entry is either in the tracked or untracked table.
--
-- [req("review.coverage", "testcov.cov.lines")]
create table TestCaseLineCoverageOverrides (
    latest_collection integer not null references Collections (id) on delete cascade,
   -- The product ID that maps to the product that got reviewed and tested.
    product_id text not null,
    -- Name of the review.
    review_name text not null,
    -- Name of the test run.
    test_run_name text not null,
    -- Name of the test case.
    test_case_name text not null,
    -- File that was covered.
    cov_filepath text not null,
    -- Line that was covered.
    cov_line integer not null,
    -- Indicates if a line is covered `true`, uncovered `false`,
    -- or unknown `null` indicating that the line has no executable code at this line.
    --
    -- If `null`, the line is ignored from line coverage analysis for this test run.
    covered bool,
    -- Hash of the comment explaining why this line coverage must be overriden.
    comment_hash text not null references GeneralTexts (hash) on delete cascade,
    primary key (
        product_id,
        review_name,
        test_run_name,
        test_case_name,
        cov_filepath,
        cov_line
    ),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade,
    foreign key (
        product_id,
        test_run_name,
        test_case_name,
        cov_filepath,
        cov_line
    ) references TestCaseLineCoverage (
        product_id,
        test_run_name,
        test_case_name,
        cov_filepath,
        cov_line
    )
);

-- Contains collected entries that could not be mapped to existing data.
-- e.g. verified requirements, test case state or code coverage overrides
create table IgnoredReviewEntries (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    review_name text not null,
    -- Hash of the content of the entry in the review that got ignored.
    entry_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (product_id, review_name, entry_hash),
    foreign key (product_id, review_name) references Reviews (product_id, name) on delete cascade
);

create table AddedReviews (
    product_id text not null,
    review_name text not null,
    primary key (product_id, review_name),
    foreign key (
        product_id,
        review_name
    ) references Reviews (product_id, name) on delete cascade
);

create trigger ReviewInsertions
after insert on Reviews
for each row
begin
    insert into AddedReviews (
        product_id,
        review_name
    )
    values (
        new.product_id,
        new.name
    );
end;

create table RemovedReviews (
    product_id text not null,
    review_name text not null,
    primary key (product_id, review_name)
);

create trigger ReviewDeletions
after delete on Reviews
for each row
begin
    insert into RemovedReviews (
        product_id,
        review_name
    )
    values (
        old.product_id,
        old.name
    );
end;

create table ChangedReviews (
    product_id text not null,
    review_name text not null,
    primary key (product_id, review_name),
    foreign key (
        product_id,
        review_name
    ) references Reviews (product_id, name) on delete cascade
);

create trigger ReviewChanges
after update on Reviews
for each row
when (
    old.data_hash is distinct from new.data_hash
)
begin
    insert into ChangedReviews (
        product_id,
        review_name
    )
    values (
        old.product_id,
        old.name
    );
end;
