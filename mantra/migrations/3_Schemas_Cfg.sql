create table Schemas (
    content_hash text not null primary key,
    origin_hash text references GeneralJson (hash) on delete restrict
);

create table SchemaSources (
    latest_collection integer not null references Collections (id) on delete cascade,
    schema_hash text not null references Schemas (content_hash) on delete cascade,
    filepath text not null,
    primary key (schema_hash, filepath),
    foreign key (filepath) references CollectedFiles (filepath) on delete cascade
);

-- The mantra configs that resulted in the collected data.
create table CollectConfigs (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    -- The number that gets incremented for every new config of a product collection.
    -- Not auto incremented, because the number must only be unique per product in one collection.
    entry_number integer not null,
    primary key (product_id, entry_number),
    foreign key (product_id) references Products (id) on delete cascade
);

-- The schemas that were collected with the related collect config.
create table ConfigCollectedSchemas (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    schema_hash text not null references Schemas (content_hash) on delete cascade,
    cfg_entry integer not null,
    primary key (product_id, schema_hash, cfg_entry),
    foreign key (product_id, cfg_entry) references CollectConfigs (product_id, entry_number) on delete cascade
);

create view ProductRelatedSchemas (product_id, schema_hash) as
select distinct product_id, schema_hash
from ConfigCollectedSchemas;
