
-- Table contains products that were collected via `mantra collect`.
-- [req("lifecycle.product.id", "report.product_data")]
create table Products (
    last_collect_nr integer not null references Collections (nr) on delete restrict,
    -- Product ID
    id text not null primary key,
    -- Name of a product.
    name text not null,
    -- Baseline of a product.
    -- e.g. git branch or commit hash
    base text not null,
    -- Optional version of a product.
    --
    -- **Note:** Version is optional, because it might not change between commits
    -- and is therefore not part of the primary key.
    version text,
    -- Optional URL to the product's homepage.
    homepage text,
    -- Optional URL to the product's repository.
    repository text,
    -- Optional license of the product.
    license text
);

create table ProductsHistory (
    nr integer primary key,
    product_id text not null,
    collect_nr text not null references Collections (nr) on delete restrict,
    operation text not null check (operation in ('insert', 'update', 'delete')),
    name text,
    base text,
    version text,
    homepage text,
    repository text,
    license text,
    metadata_hash text references GeneralJson (hash) on delete restrict
);

create trigger ProductsUpdates
after update on Products
for each row
when (
    old.name is distinct from new.name or
    old.base is distinct from new.base or
    old.version is distinct from new.version or
    old.homepage is distinct from new.homepage or
    old.repository is distinct from new.repository or
    old.license is distinct from new.license or
    old.metadata_hash is distinct from new.metadata_hash
)
begin
    insert into ProductsHistory (
        product_id,
        collect_nr,
        operation,
        name,
        base,
        version,
        homepage,
        repository,
        license,
        metadata_hash
    )
    values (
        old.id,
        (select max(nr) from Collections),
        'update',
        case when old.name is distinct from new.name then old.name else null end,
        case when old.base is distinct from new.base then old.base else null end,
        case when old.version is distinct from new.version then old.version else null end,
        case when old.homepage is distinct from new.homepage then old.homepage else null end,
        case when old.repository is distinct from new.repository then old.repository else null end,
        case when old.license is distinct from new.license then old.license else null end,
        case when old.metadata_hash is distinct from new.metadata_hash then old.metadata_hash else null end
    );
end;

create trigger ProductsInsertions
after insert on Products
for each row
begin
    insert into ProductsHistory (
        product_id,
        collect_nr,
        operation,
        name,
        base,
        version,
        homepage,
        repository,
        license,
        metadata_hash
    )
    values (
        new.id,
        (select max(nr) from Collections),
        'insert',
        new.name,
        new.base,
        new.version,
        new.homepage,
        new.repository,
        new.license,
        new.metadata_hash
    );
end;

create trigger ProductsDeletions
after delete on Products
for each row
begin
    insert into ProductsHistory (
        product_id,
        collect_nr,
        operation,
        name,
        base,
        version,
        homepage,
        repository,
        license,
        metadata_hash
    )
    values (
        old.id,
        (select max(nr) from Collections),
        'delete',
        old.name,
        old.base,
        old.version,
        old.homepage,
        old.repository,
        old.license,
        old.metadata_hash
    );
end;

create table ProductRelatedFiles (
    last_collect_nr integer not null references Collections (nr) on delete restrict,
    -- Product ID
    product_id text not null,
    filepath text not null,
    file_hash text not null references FileHashes (hash) on delete restrict,
    constraint ProductRelatedFiles primary key (product_id, filepath)
);

-- Table to map to properties of products.
create table ProductProperties (
    last_collect_nr integer not null references Collections (nr) on delete restrict,
    product_id text not null,
    -- Key of the property
    property_key text not null,
    -- Hash of a custom property of the product.
    value_hash text not null references GeneralJson (hash) on delete restrict,
    constraint ProductPropertiesPk primary key (product_id, property_key),
    foreign key (product_id) references Products (product_id) on delete cascade
);
