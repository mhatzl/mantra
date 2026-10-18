
-- Table contains products that were collected via `mantra collect`.
-- [req("lifecycle.product.id", "report.product_data")]
create table Products (
    collect_nr integer not null references Collections (nr) on delete restrict,
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
    primary key (collect_nr, id)
);

create table ProductRelatedFiles (
    collect_nr integer not null,
    -- Product ID
    product_id text not null,
    filepath text not null,
    primary key (collect_nr, product_id, filepath),
    foreign key (collect_nr, product_id) references Products (collect_nr, id) on delete cascade,
    foreign key (collect_nr, filepath) references CollectedFiles (collect_nr, filepath) on delete restrict
);

create table ProductRelatedHashedFiles (
    collect_nr integer not null,
    -- Product ID
    product_id text not null,
    filepath text not null,
    file_hash text not null references FileHashes (hash) on delete restrict,
    primary key (collect_nr, product_id, filepath, file_hash),
    foreign key (collect_nr, product_id, filepath) references ProductRelatedFiles (collect_nr, product_id, filepath) on delete cascade
);

-- Table to map to properties of products.
create table ProductProperties (
    collect_nr integer not null references Collections (nr) on delete restrict,
    product_id text not null,
    -- Key of the property
    property_key text not null,
    -- Hash of a custom property of the product.
    value_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (collect_nr, product_id, property_key),
    foreign key (collect_nr, product_id) references Products (collect_nr, id) on delete cascade
);

create table AddedProducts (
    first_collect_nr integer not null,
    product_id text not null,
    primary key (first_collect_nr, product_id),
    foreign key (first_collect_nr, product_id) references Products (collect_nr, id) on delete cascade
);

create table ChangedProducts (
    new_collect_nr integer not null,
    old_collect_nr integer not null,
    product_id text not null,
    primary key (new_collect_nr, product_id),
    foreign key (new_collect_nr, product_id) references Products (collect_nr, id) on delete cascade,
    foreign key (old_collect_nr, product_id) references Products (collect_nr, id) on delete cascade
);

create table ExistingProducts (
    latest_collect_nr integer not null,
    product_id text not null,
    primary key (product_id),
    foreign key (latest_collect_nr, product_id) references Products (collect_nr, id) on delete cascade
);

create table RemovedProducts (
    removed_collect_nr integer not null references Collections (nr) on delete restrict,
    last_collect_nr integer not null,
    product_id text not null,
    primary key (last_collect_nr, product_id),
    foreign key (last_collect_nr, product_id) references Products (collect_nr, id) on delete cascade
);
