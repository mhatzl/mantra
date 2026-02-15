-- Contains requirements that are verified by fulfilling at least one of the following conditions:
-- - it is not part of the ManualRequirements table and either:
--   - if no *statisfies* trace exists for the requirement,
--     and a direct *verifies* trace mentions the ID and the trace is covered by at least one statement
--     from coverage metrics of a test run or test case, and all test runs or test cases
--     that cover the statement passed
--   - if a *satisfies* trace exists, in addition to the conditions above,
--     at least one *satisfies* trace must also be covered by the same test run or test case
--     the *verifies* trace is covered
--   - a test case verifies the requirement and passed
--   - if no *verifies* trace for the requirement exists, but *satisfies* traces exist:
--     all *satisfies* traces must be covered, and all test runs or test cases that cover a *satisfies* trace must pass
-- - it is verified by a review if the requirement is part of the ManualRequirements table.
--   if *verifies* or *satisfies* traces exist, the verified-conditions above for traces must also pass
create table DirectlyVerifiedRequirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    id text not null,
    primary key (product_id, id),
    foreign key (product_id, id) references Requirements(product_id, id) on delete cascade
);
