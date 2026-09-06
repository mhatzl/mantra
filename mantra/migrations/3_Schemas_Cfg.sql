create table Schemas (
    content_hash text not null primary key,
    origin_hash text references GeneralJson (hash) on delete restrict
);

create table SchemaSources (
    collect_nr integer not null,
    schema_hash text not null references Schemas (content_hash) on delete cascade,
    filepath text not null,
    primary key (collect_nr, schema_hash, filepath),
    foreign key (collect_nr, filepath) references CollectedFiles (collect_nr, filepath) on delete cascade
);

-- The mantra configs that resulted in the collected data.
create table CollectConfigs (
    collect_nr integer not null,
    product_id text not null,
    -- The number that gets incremented for every new config of a product collection.
    -- Not auto incremented, because the number must only be unique per product in one collection.
    nr integer not null,
    -- Hash of the serialized config.
    -- This also includes origin and properties fields.
    cfg_hash text not null references GeneralJson (hash) on delete restrict,
    origin_hash text references GeneralJson (hash) on delete restrict,
    primary key (collect_nr, product_id, nr),
    foreign key (collect_nr, product_id) references Products (collect_nr, id) on delete cascade
);

-- The schemas that were collected with the related collect config.
create table ConfigCollectedSchemas (
    collect_nr integer not null,
    product_id text not null,
    schema_hash text not null references Schemas (content_hash) on delete cascade,
    cfg_nr integer not null,
    primary key (collect_nr, product_id, schema_hash, cfg_nr),
    foreign key (collect_nr, product_id, cfg_nr) references CollectConfigs (collect_nr, product_id, nr) on delete cascade
);

create view ProductRelatedSchemas (collect_nr, product_id, schema_hash) as
select distinct collect_nr, product_id, schema_hash
from ConfigCollectedSchemas;
