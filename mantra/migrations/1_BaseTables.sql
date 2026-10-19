-- Table to store plain text and the related hash.
-- This reduces duplication of unchanged content.
--
-- [req("changes.show", "changes.compact_content")]
create table GeneralTexts (
    -- Hash of the content
    hash text not null primary key,
    -- Content that in general has no structure that is queryable by a regular SQL database.
    content text not null
);

-- Table to store JSON content and the related hash.
-- This reduces duplication of unchanged content.
--
-- TODO: map requirement
create table GeneralJson (
    -- Hash of the content
    hash text not null primary key,
    -- JSON content that may contain user defined information.
    content text not null
);

-- Table to store hashes of file contents from which data was collected.
-- [req("changes.track.traces.files")]
create table FileHashes (
    -- Hash of the file content.
    hash text not null primary key,
    -- Optional content that resulted in the hash.
    -- Only optional, because e.g. test runs may return a file hash but no file content.
    content text
);

-- Base table used to track changes over multiple `mantra collect` runs.
-- [req("lifecycle.product", "changes.track")]
create table Collections (
    nr integer primary key autoincrement,
    collected_at_utc text not null,
    -- Optional hash of the arguments set when calling `mantra collect`.
    arguments_hash text references GeneralJson (hash) on delete restrict,
    -- Optional hash of the environmental variables set that are relevant for mantra
    -- when calling `mantra collect`.
    env_vars_hash text references GeneralJson (hash) on delete restrict
);

create table CollectedFiles (
    collect_nr integer not null references Collections (nr) on delete cascade,
    filepath text not null,
    -- Optional reference to the hash of the file content
    file_hash text references FileHashes (hash) on delete restrict,
    -- Optional MIME/media type of the stored content.
    media_type text,
    primary key (collect_nr, filepath)
);

-- In case a file was collected with different hash values in one collection.
-- This may happen if annotations or requirements are collected from a file,
-- and test runs contain coverage data for the same file but with a different file hash.
create table ConflictingCollectedFiles (
    collect_nr integer not null,
    filepath text not null,
    file_hash text not null,
    primary key (collect_nr, filepath, file_hash),
    foreign key (collect_nr, filepath) references CollectedFiles (collect_nr, filepath) on delete cascade
);

-- Table to store logs that were encountered while executing `mantra collect`.
-- e.g. review mentions unknown requirement ID
create table CollectionLogs (
    collect_nr integer not null references Collections (nr) on delete cascade,
    timestamp text not null,
    -- null=print, 0=trace, 1=debug, 2=info, 3=warning, 4=error
    level integer,
    msg_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (collect_nr, timestamp)
);
