
-- Table containing all requirement IDs collected by mantra.
-- [req("req.id", "changes.track.reqs.id")]
create table Requirements (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    id text not null,
    product_id text not null references Products (id) on delete cascade,
    -- Flag indicating whether the requirement requires manual verification.
    -- `true`: The requirement requires manual verification.
    -- [req("req.manual")]
    manual_verification bool not null,
    -- Flag indicating whether the requirement is deprecated.
    -- `true`: The requirement is deprecated.
    -- [req("req.deprecated")]
    deprecated bool not null,
    -- Flag indicating whether the requirement should be ignored for this product.
    -- `true`: The requirement must be ignored.
    -- [req("req.ignored")]
    ignore bool not null,
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
    -- Hash of the content the requirement was collected from.
    src_hash text not null,
    constraint RequirementsPk primary key (id, product_id)
);

-- Table to map to properties of requirements.
-- [req("req.properties")]
create table RequirementProperties (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    req_id text not null,
    product_id text not null,
    -- Key of the property
    property_key text not null,
    -- Hash of a custom property of the requirement.
    value_hash text not null references GeneralJson (hash) on delete restrict,
    constraint RequirementPropertiesPk primary key (req_id, product_id, property_key),
    foreign key (req_id, product_id) references Requirements (id, product_id) on delete cascade
);

-- Table to represent the requirement hierarchy per requirement content.
--
-- **Note:** Per requirement content, because the parent IDs are part of the content.
-- [req("req.hierarchy")]
create table RequirementHierarchies (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    -- Product ID the child requirements id defined in.
    child_product_id text not null,
    -- The ID of the child requirement, whose content referenced the parent ID.
    child_req_id text not null,
    -- The product ID the parent requirement is defined in.
    parent_product_id text not null,
    -- The ID of the parent requirement.
    parent_req_id text not null,
    constraint RequirementHierarchiesPk primary key (child_product_id, child_req_id, parent_product_id, parent_req_id),
    foreign key (child_product_id, child_req_id) references Requirements (product_id, id) on delete cascade deferrable initially deferred,
    foreign key (parent_product_id, parent_req_id) references Requirements (product_id, id) on delete cascade deferrable initially deferred
);
