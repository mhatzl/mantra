
-- Table containing all requirement IDs collected by mantra.
-- [req("req.id", "changes.track.reqs.id")]
create table Requirements (
    collect_nr integer not null,
    id text not null,
    product_id text not null,
    -- Flag indicating whether the requirement requires manual verification.
    -- `true`: The requirement requires manual verification.
    -- [req("req.manual")]
    manual_verification bool not null,
    -- Flag indicating whether the requirement is deprecated.
    -- `true`: The requirement is deprecated.
    -- [req("req.deprecated")]
    deprecated bool not null,
    -- Flag indicating whether the requirement should be excluded for this product.
    -- `true`: The requirement must be excluded.
    -- [req("req.exclude")]
    exclude bool not null,
    -- Flag indicating whether the requirement is optional to be verified.
    -- Verification state of optional requirements does **not** affect the state of parents.
    -- All children of an optional requirement are also optional.
    -- `true`: The requirement is optional.
    -- [req("req.optional")]
    optional bool not null,
    -- The title of the requirement.
    -- [req("req.title")]
    title text not null,
    -- Optional origin data of the requirement that was set for multiple requirements.
    -- [req("req.origin")]
    base_origin_hash text references GeneralJson (hash) on delete restrict,
    -- The origin data of the requirement.
    -- [req("req.origin")]
    origin_hash text not null references GeneralJson (hash) on delete restrict,
    -- Optional description content of the requirement.
    -- [req("req.description")]
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- Hash of the data the requirement was collected from.
    --
    -- **Note:** This may differ from the related file content hash if the file defined more than one requirement.
    data_hash text not null,
    -- Filepath the data was collected from
    data_filepath text not null,
    -- Optional MIME/media type of requirement related general texts (e.g. title and description).
    media_type text,
    primary key (collect_nr, id, product_id),
    foreign key (collect_nr, product_id) references Products (collect_nr, id) on delete cascade,
    foreign key (product_id, data_filepath) references ProductRelatedFiles (collect_nr, product_id, filepath) on delete cascade
);

-- Table to map to properties of requirements.
-- [req("req.properties")]
create table RequirementProperties (
    collect_nr integer not null,
    req_id text not null,
    product_id text not null,
    -- Key of the property
    property_key text not null,
    -- Hash of a custom property of the requirement.
    value_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (req_id, product_id, property_key),
    foreign key (collect_nr, req_id, product_id) references Requirements (collect_nr, id, product_id) on delete cascade
);

-- Table to store the direct parents of a requirement.
--
-- **Note:** Parent requirement may not be collected in same collection,
-- so mapping is done later during aggregation in table RequirementHierarchies.
-- [req("req.hierarchy")]
create table RequirementParents (
    collect_nr integer not null,
    -- Product ID the requirement is defined in.
    product_id text not null,
    -- The ID of the requirement, whose content referenced the parent ID.
    req_id text not null,
    -- The product ID the parent requirement is defined in.
    parent_product_id text not null,
    -- The ID of the parent requirement.
    parent_req_id text not null,
    -- 'true' makes the requirement an optional child
    optional bool not null,
    primary key (collect_nr, product_id, req_id, parent_product_id, parent_req_id),
    foreign key (collect_nr, product_id, req_id) references Requirements (collect_nr, product_id, id) on delete cascade
);

-- Table to store the direct children of a requirement.
--
-- **Note:** Child requirement may not be collected in same collection,
-- so mapping is done later during aggregation in table RequirementHierarchies.
-- [req("req.hierarchy")]
create table RequirementChildren (
    collect_nr integer not null,
    -- Product ID the requirements id defined in.
    product_id text not null,
    -- The ID of the requirement, whose content referenced the parent ID.
    req_id text not null,
    -- The product ID the child requirement is defined in.
    child_product_id text not null,
    -- The ID of the child requirement.
    child_req_id text not null,
    -- 'true' makes the child requirement optional for the (parent) requirement
    optional bool not null,
    primary key (collect_nr, product_id, req_id, child_product_id, child_req_id),
    foreign key (collect_nr, product_id, req_id) references Requirements (collect_nr, product_id, id) on delete cascade
);

-- Table to map if a requirement replaces others.
--
-- **Note:** Replacements are only possible inside the same product.
-- [req("req.replacing")]
create table RequirementReplacements (
    collect_nr integer not null,
    req_id text not null,
    product_id text not null,
    -- Requirement id that is replaced by the requirement set in "req_id"
    replaced_req_id text not null,
    primary key (collect_nr, req_id, product_id, replaced_req_id),
    foreign key (collect_nr, req_id, product_id) references Requirements (collect_nr, id, product_id) on delete cascade,
    -- **Note:** Deferred, because replaced ID may not have been collected yet
    foreign key (collect_nr, replaced_req_id, product_id) references Requirements (collect_nr, id, product_id) on delete cascade deferrable initially deferred
);
