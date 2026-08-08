
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
