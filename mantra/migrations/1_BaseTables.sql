-- Table to store plain text and the related hash.
-- This reduces duplication of unchanged content.
--
-- [req("changes.show", "changes.compact_content")]
create table GeneralTexts (
    -- Hash of the content
    hash text not null primary key,
    -- Content that is either plain text or of unknown format to mantra.
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
    hash text not null
);

-- Base table used to track changes over multiple `mantra collect` runs.
-- [req("lifecycle.product", "changes.track")]
create table Collections (
    nr integer primary key autoincrement,
    run_at_utc text not null,
    -- Filepath to the `mantra.toml` file that was used to collect the data.
    -- Path is relativ to the invocation of `mantra collect`.
    -- [req("changes.track.origin")]
    collect_filepath text not null,
    -- The hash of the configuration content in `mantra.toml` for this collection.
    -- [req("cli.collect.config")]
    config_hash text not null references GeneralJson (hash) on delete restrict,
    -- Optional hash of the arguments set when calling `mantra collect`.
    arguments_hash text references GeneralJson (hash) on delete restrict,
    -- Optional hash of the environmental variables set that are relevant for mantra
    -- when calling `mantra collect`.
    env_vars_hash text references GeneralJson (hash) on delete restrict
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
