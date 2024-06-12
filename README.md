# mantra

![build-test](https://github.com/mhatzl/mantra/actions/workflows/rust.yml/badge.svg?branch=main)
[![docker](https://github.com/mhatzl/mantra/actions/workflows/docker.yml/badge.svg?branch=main)](https://hub.docker.com/r/manuelhatzl/mantra)

**M**anuels **AN**forderungs-**TRA**cing (or **MAN**aged **TRA**cing)

*mantra* is a tool for easier tracing between requirements, implementation, and tests.

Checkout the [usage example](/mantra/examples/) folder
to see how requirement tracing with *mantra* works.

## Core Concepts

IDs are used to identify requirements, and reference them in the implementation and/or tests.
These requirement IDs must be set manually on the implementation and test side.
*mantra* then adds available requirements and found traces into a SQL database for further analysis.

Using the trace information, test code coverage may be passed to *mantra*
to get requirement coverage information.

### Requirement ID

Every requirement must have a unique requirement ID.
A requirement hierarchy may be created by using the parent ID as prefix followed by `.`.

**Example:**

```
req_id

req_id.sub_req_id
```

### Requirement tracing

To get requirement traces, IDs may be referenced in the implementation and/or tests of your system/project.
How requirement IDs are referenced may vary between programming languages.
If no special syntax is defined for a file type, the default is to search for references
having the form `[req(<requirement id(s)>)]`.

**Language specific tracing:**

- **Rust**: Uses [`mantra-rust-trace`](/langs/rust/mantra-rust-trace/README.md) to collect requirement traces

  Add [`mantra-rust-macros`](/langs/rust/mantra-rust-macros/README.md) to your dependencies to create requirement traces using
  the attribute macro `req` or the fn-like macro `reqcov`.

  **Example:**

  ```rust
  use mantra_rust_macros::{req, reqcov};

  #[req(req_id)]
  fn some_fn() {
    reqcov!(function_like_trace);
  }
  ```

## Usage
### Per CLI

*mantra* may be installed using `cargo install mantra`.

All information is stored in a SQL database and the connection may be set
before any command using `url`. By default, the URL is `sqlite://mantra.db?mode=rwc`.

**Note:** Only SQLite is supported for now, because some SQL queries contain SQLite specific syntax.

- Collect all data at once

  `mantra collect [<filepath>]`

  This will look for a TOML file at the given path.
  By default, the path is set to `mantra.toml`.

  **File structure:**

  ```toml
  # Collect requirements from local Markdown files.
  #
  # **Note:** Only one option to collect requirements may be given.
  [requirements.from-wiki]

  # Root path to start looking for requirements.
  # Empty means current directory.
  root = ""

  # Base URL for all requirements
  link = "https://github.com/mhatzl/mantra-wiki/tree/main/5-Requirements/"


  # Collect requirements from a JSON file adhering to the `RequirementSchema`.
  #
  # **Note:** Only one option to collect requirements may be given.
  [requirements.from-schema]

  # The path to a JSON file containing requirements.
  filepath = "requirements.json"


  # Collect traces from local files
  #
  # **Note:** Only one option to collect traces may be given.
  [traces.from-source]

  # Root path to start looking for traces.
  # Empty means current directory.
  root = ""

  # If 'false', the filepath will be stored relativ to the root path.
  keep-path-absolute = false


  # Collect traces from a JSON file adhering to the `TraceSchema`.
  #
  # **Note:** Only one option to collect traces may be given.
  [trace.from-schema]

  # The path to a JSON file containing traces.
  filepath = "traces.json"


  # Collect coverage from a JSON file adhering to the `CoverageSchema`.
  [coverage]

  # Path to a JSON file containing coverage.
  filepath = "coverage.json"


  # Collect reviews from TOML files adhering to the `ReviewSchema`.
  [reviews]

  # List of review files to add.
  files = ["first_review.toml"]
  ```

- Generate a report

  `mantra report --formats=html,json <file path>`

  This will create an HTML and JSON report at the given file path.
  Optionally, a template file may be given via `--template`.
  Templates may use the [Tera](https://keats.github.io/tera/docs/) template language.
  The JSON form is passed to the template.
  If no template is given, the [report_default_template](/mantra/src/cmd/report_default_template.html) is used.

  To render custom data like requirement info and test-run metadata,
  the arguments `--info-template` and `--test-run-template` may be set to template files.
  These templates are then pre-rendered using [Tera](https://keats.github.io/tera/docs/),
  and the rendered content is made available as `rendered_info` and `rendered_meta` next to the regular `info` and `meta` fields.

  Project name, version, and link may be set using the arguments `--project_name`, `--project_version`, and `--project_link`.
  A tag name and link may also be set using the arguments `--tag-name` and `--tag-link`.
  Tags should be used to indicate the requirements-snapshot/tag the report was generated with.

- Adding requirements from local Markdown files

  `mantra requirements from-wiki <root> <link>`

  The `root` argument must point to an existing file/folder containing requirements.
  The `link` is the base URL of the requirements.

  This command does **not** delete requirements that were already stored in the database.
  *Generation* counter are used to detect if an existing requirement was not present
  in the latest *extraction*.

  Use `mantra delete-old` to remove all requirements not added/updated in the latest call.

- Adding traces from local sources

  `mantra trace from-source [--keep-path-absolute] <root>`

  The `root` argument must point to a file or folder to search for traces in text files.
  By default, file paths for traces are added as relative paths to the given root.
  This may be changed by setting `--keep-path-absolute`.

- Adding coverage

  Because coverage output from tests highly depends on the language and tooling,
  *mantra* only provides to add coverage data via files adhering to the `RequirementSchema`.

  `mantra coverage --data-file <path to JSON file>`

- Adding reviews

  `mantra review <reviews>`

  One or more file paths may be given that point to *mantra* reviews.

  **Note:** Only TOML is supported as review format for now.

  **File structure:**

  ```toml
  name = <review name>
  date = <yyyy-mm-dd HH:MM[optional [:SS.fraction]]>
  reviewer = <reviewer of this review>
  comment = <general comment for this review>

  [[requirements]]
  id = <verified requirement ID>
  comment = <optional comment for this specific ID>

  [[requirements]]
  id = <verified requirement ID>
  comment = <optional comment for this specific ID>
  ```

# License

MIT Licensed
