
create table AnnotatedFileOrigins (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    filepath text not null,
    base_origin_hash text not null references GeneralJson (hash) on delete restrict,
    primary key (product_id, filepath),
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade
);

-- Table to store all traces.
-- [req("trace.origin", "changes.track")]
create table Traces (
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at in the file.
    line integer not null,
    -- Trace kind (0 = clarifies, 1 = satisfies, 2 = verifies, 3 = links).
    -- [req("trace.kind")]
    kind integer not null,
    primary key (file_hash, line),
    foreign key (file_hash) references FileHashes (hash) on delete restrict
);

-- Table to store custom properties of traces.
-- [req("trace.properties")]
create table TraceProperties (
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    -- Custom property of the trace. e.g. "critical"
    property_key text not null,
    property_value text references GeneralJson (hash) on delete restrict,
    primary key (file_hash, line, property_key),
    foreign key (file_hash, line) references Traces (file_hash, line) on delete cascade
);

-- Table to store requirement IDs linked to traces.
--
-- **Note:** Actual mapping to the Requirements table is done indirectly via ProductRelatedFiles.
-- [req("trace.id", "trace.mult_reqs")]
create table DirectReqTraces (
    -- Requirement ID that is directly set on the trace.
    req_id text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace was detected at.
    line integer not null,
    primary key (req_id, file_hash, line),
    foreign key (file_hash, line) references Traces (file_hash, line) on delete cascade
);

create table DirectProductReqTraces (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    req_id text not null,
    filepath text not null,
    file_hash text not null,
    line integer not null,
    primary key (product_id, req_id, filepath, file_hash, line),
    foreign key (product_id, req_id) references Requirements (product_id, id) on delete cascade,
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (req_id, file_hash, line) references DirectReqTraces (req_id, file_hash, line) on delete cascade
);

-- Table to store language elements such as functions, tests, structs, enums, classes, ...
--
-- Note: Elements are uniquely identifiable by filepath and line number.
-- Due to feature flags or language semantics, idents may be declared multiple times, and are therefore not unique.
-- [req("trace.element")]
create table Elements (
    -- Name of the element.
    --
    -- **Note:** The fully qualified identifier is stored in ElementIdents.
    name text not null,
    -- Hash of the file content.
    file_hash text not null references FileHashes (hash) on delete restrict,
    -- Line the element is defined at.
    definition_line integer not null,
    -- Line the element span starts.
    -- [req("trace.element.span")]
    start_line integer not null,
    -- Line the element span ends.
    -- [req("trace.element.span")]
    end_line integer not null,
    -- Type of the element.
    -- [req("trace.element.kind")]
    kind integer not null,
    -- Optional hash of the content of the element.
    content_hash text references GeneralTexts (hash) on delete restrict,
    primary key (file_hash, definition_line),
    constraint start_le_end check (start_line <= end_line),
    constraint def_in_span check (start_line <= definition_line and definition_line <= end_line)
);

create table ElementIdents (
    last_collect_nr bigint not null references Collections (nr) on delete restrict,
    product_id text not null,
    -- File the element is defined in.
    filepath text not null,
    -- Hash of the file content.
    file_hash text not null,
    -- Line the element is defined at.
    definition_line integer not null,
    ident text not null,
    primary key (product_id, filepath, file_hash, definition_line),
    foreign key (product_id, filepath) references ProductRelatedFiles (product_id, filepath) on delete cascade,
    foreign key (file_hash, definition_line) references Elements (file_hash, definition_line) on delete cascade
);

-- Table to store language code blocks that are linked to traces.
-- [req("trace.code_block")]
create table TracedCodeBlocks (
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace related to the code block is set.
    traced_line integer not null,
    -- Line the code block span starts.
    -- [req("trace.code_block.span")]
    start_line integer not null,
    -- Line the code block span ends.
    -- [req("trace.code_block.span")]
    end_line integer not null,
    -- The code block kind. other=0, if=1, else-if=2, else=3, loop=4, while=5, for=6, match/case=7,
    kind integer not null,
    -- Optional hash of the code block.
    content_hash text references GeneralTexts (hash) on delete restrict,
    primary key (file_hash, traced_line),
    foreign key (file_hash, traced_line) references Traces (file_hash, line) on delete cascade,
    constraint start_le_trace_le_end check (start_line <= traced_line and traced_line <= end_line)
);

-- Table to store direct links between elements and traces.
--
-- ```rust
-- #[derive(Debug)]         ... <- element start line
-- #[req("trace.element")]  ... <- traced line
-- fn foo() {               ... <- definition line
--   //...
-- }                        ... <- end line
-- ```
--
-- [req("trace.element")]
create table DirectTracedElements (
    -- Hash of the file content.
    file_hash text not null,
    -- Line the trace related to the element was detected at.
    traced_line integer not null,
    -- Line the element is defined at.
    element_definition_line integer not null,
    primary key (
        file_hash,
        traced_line,
        element_definition_line
    ),
    foreign key (file_hash, element_definition_line) references Elements (file_hash, definition_line) on delete cascade,
    foreign key (file_hash, traced_line) references Traces (file_hash, line) on delete cascade
);

-- Table to store line spans that must be excluded from code coverage analysis.
--
-- TODO: add req trace
create table CoverageBlockExcludes (
    -- Hash of the file content.
    file_hash text not null references FileHashes (hash) on delete restrict,
    -- First line that must be excluded from code coverage analysis until the `end_line`.
    start_line integer not null,
    -- Last line that must be excluded (inclusive) from code coverage analysis.
    end_line integer not null,
    -- Hash of the comment explaining why the spanned lines must be excluded from code coverage calculations.
    comment_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (file_hash, start_line),
    constraint start_le_end check (start_line <= end_line)
);

-- Table to store lines that must be excluded from code coverage analysis.
--
-- TODO: add req trace
create table CoverageLineExcludes (
    -- Hash of the file content.
    file_hash text not null references FileHashes (hash) on delete restrict,
    -- Line that must be excluded from code coverage analysis.
    line integer not null,
    -- Hash of the comment explaining why the line must be excluded from code coverage analysis.
    comment_hash text not null references GeneralTexts (hash) on delete restrict,
    primary key (file_hash, line)
);
