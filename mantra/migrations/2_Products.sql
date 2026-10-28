
-- Table contains products that were collected via `mantra collect`.
-- [req("lifecycle.product.id", "report.product_data")]
create table Products (
    latest_collection integer not null references Collections (id) on delete cascade,
    -- Product ID
    id text not null,
    -- Name of a product.
    name text not null,
    -- Optional baseline of a product.
    -- e.g. git branch or commit hash
    base text,
    -- Optional version of a product.
    version text,
    -- Optional URL to the product's homepage.
    homepage text,
    -- Optional URL to the product's repository.
    repository text,
    -- Optional license of the product.
    license text,
    -- Optional description of the product.
    description_hash text references GeneralTexts (hash) on delete restrict,
    -- Optional MIME/media type of product related general texts (e.g. description).
    media_type text,
    -- hash of the product metadata
    data_hash text not null,
    primary key (id)
);

create table ProductRelatedFiles (
    latest_collection integer not null references Collections (id) on delete cascade,
    -- Product ID
    product_id text not null,
    filepath text not null,
    primary key (product_id, filepath),
    foreign key (product_id) references Products (id) on delete cascade,
    foreign key (filepath) references CollectedFiles (filepath) on delete restrict
);

create table ProductRelatedHashedFiles (
    latest_collection integer not null references Collections (id) on delete cascade,
    -- Product ID
    product_id text not null,
    filepath text not null,
    file_hash text not null references FileHashes (hash) on delete restrict,
    primary key (product_id, filepath),
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade
);

-- Table to map to properties of products.
create table ProductProperties (
    latest_collection integer not null references Collections (id) on delete cascade,
    product_id text not null,
    -- Key of the property
    property_key text not null,
    -- Hash of a custom property of the product.
    value_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (product_id, property_key),
    foreign key (product_id) references Products (id) on delete cascade
);

create table AddedProducts (
    product_id text not null,
    primary key (product_id),
    foreign key (product_id) references Products (id) on delete cascade
);

create table ChangedProducts (
    product_id text not null primary key references Products (id) on delete cascade
);

create table RemovedProducts (
    product_id text not null primary key
);
