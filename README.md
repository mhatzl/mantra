# mantra

[![crates.io](https://img.shields.io/crates/v/mantra.svg)](https://crates.io/crates/mantra)

**M**anuels **AN**forderungs-**TRA**cing (or **MAN**aged **TRA**cing)

*mantra* is a tool for easier tracing between requirements, implementation, and tests.

While requirements define the intent, implementation and testing define the actual state.
Requirements traceability is an approach to keep those two sides synchronized,
by mapping requirements to their implementations and tests. This mapping then allows to aggregate
the [state](/docs/wiki/5-Requirements/5-REQ-requirement.md#reqstate-requirement-states) of requirements
and also has the benefit that navigating through a codebase becomes easier.

**Requirement states handled by *mantra*:**
- **Failed** ... At least one test mapping to the requirement has failed
- **Verified** ... Requirements that fulfill *mantra*'s [`verified` conditions](/docs/wiki/5-Requirements/5-REQ-requirement.md#reqstateverified-verified-requirements)
- **Skipped** ... At least one test mapping to the requirement was skipped
- **Unverified** ... Requirements that don't fulfill *mantra*'s [`verified` conditions](/docs/wiki/5-Requirements/5-REQ-requirement.md#reqstateverified-verified-requirements)
- **Deprecated** ... For requirements that have been marked as [deprecated](/docs/wiki/5-Requirements/5-REQ-requirement.md#reqdeprecate-deprecate-requirements)
- **Excluded** ... For requirements that have been marked as [excluded](/docs/wiki/5-Requirements/5-REQ-requirement.md#reqexclude-exclude-requirements)

Since requirements may be structured in a hierarchical manner,
*mantra* has a set of [rules](/docs/wiki/5-Requirements/5-REQ-requirement.md#reqstate-requirement-states)
that define how requirement states of children indirectly affect the state of their parents.
This hierarchical state transfer helps to reduce the effort to track requirements,
because at best, only leaf requirements (those without children) must be traced explicitly.

**The high-level usage flow of *mantra* is:**
1. Configure where *mantra* should collect data from
2. Add or modify requirements, giving each a unique ID
3. Implement requirements, adding traces to requirements using their IDs
4. Write tests, again adding traces to requirements using their IDs
5. Optional: Create manual reviews e.g. to verify hard-to-test requirements
6. Run `mantra collect`
7. Run `mantra report` to generate a requirements traceability report
8. Repeat from step 2 until the project is complete

For a quick overview of those steps, see the [getting-started section](#getting-started) below.
More details about using and configuring *mantra* may be found in the [/docs/usage](/docs/usage) folder.
The goals, requirements, and decisions behind *mantra* are documented under [/docs/wiki](/docs/wiki/README.md), which is also published on *mantra*'s [GitHub wiki](https://github.com/mhatzl/mantra/wiki).

**Note:** Currently, focus is on support for Rust code, but built-in support for other languages is planned.
If your language is not supported directly, you may create your own tooling that extracts relevant information
and converts it into a format *mantra* understands following *mantra*'s [JSON schemas]().

## Installation
### Prerequisites

*mantra* uses the [tree-sitter](https://crates.io/crates/tree-sitter) crate to find *mantra* annotations in source code,
and [sqlx]() with bundled SQLite to store collected information.
Those crates require access to a [native C compiler](https://docs.rs/cc/latest/cc/#compile-time-requirements).

Ensure `cc` is available on `Path` if you install *mantra* from source.

### Via `cargo install`

*mantra* may be installed from source via `cargo install mantra`.

**Note:** Ensure a native C compiler is available.

## Getting Started

This section provides a high-level overview to get you started using *mantra*.
For more details, take a look at the [/usage](/docs/usage) folder.

### Configuring *mantra*

By default, *mantra* looks for a `mantra.json5` located at the current working directory.
You may change this by explicitly setting a path via the `--config-filepath` argument.
Currently, *mantra* accepts either `JSON5`, `JSON`, or `TOML` as file format for the configuration file,
with the format being automatically detected based on the file extension.

The following configuration sets up the `mantra-demo` product for the `main` baseline (e.g. branch):

```json5
products: [{
    name: "mantra-demo",
    base: "main",
    requirements: [{
        path: "reqs/",
        source: "schema",
    }],
    annotations: [{
        path: "./",
        source: "content",
        pattern: "*.rs"
    }],
    test_runs: [{
        path: "target/nextest/default",
        source: {
            test: {
                format: "junit",
                pattern: "*junit.xml",
            },
            coverage: {
                format: "cobertura_loose",
                pattern: "*cobertura.xml",
            }
        }
    }],
    reviews: [{
        path: "reviews/",
        source: "schema",
    }]
}],
```

Based on this configuration, files defining requirements are expected to be located under the `reqs/` folder
following the [RequirementSchema](schema-gen/generated/collect/RequirementSchema.json).
Annotations such as requirement traces are extracted from Rust code files located in the repository.
Test and code coverage results are expected under `target/nextest/default`, with test results following the [JUnit XML](https://llg.cubic.org/docs/junit/) format and code coverage being represented in the [Cobertura Loose XML](https://github.com/cobertura/cobertura/blob/master/cobertura/src/site/htdocs/xml/coverage-loose.dtd) format.
Files containing manual reviews are expected to be located under the `reviews/` folder
following the [ReviewSchema](schema-gen/generated/collect/ReviewSchema.json).

More details related to *mantra*'s configuration file can be found under [/docs/usage/configuration](/docs/usage/configuration.md).

### Defining Requirements

Currently, only the [RequirementSchema](schema-gen/generated/collect/RequirementSchema.json) is supported as input for requirement definitions.

The following configuration defines three requirements `gs-req-1`, `gs-req-2`, and `gs-req-1.sub-1`:

```json5
{
    requirements: [{
        id: "gs-req-1",
        title: "First requirement",
        origin: "Custom field to state where the requirement originated from",
    },{
        id: "gs-req-2",
        title: "Second requirement",
        origin: "Some origin...",
        manual_verification: true,
    },{
        id: "gs-req-1.sub-1",
        title: "First sub-requirement",
        origin: {
            url: "example.com",
            accessed_on: "2026-06-22 12:00:00+1"
        },
        parents: [{ id: "gs-req-2" }],
        optional: true,
    }]
}
```

The first requirement `gs-req-1` lists all mandatory fields. The `id` is used to identify the requirement
throughout the collected data and must be unique per product.
The `origin` field allows to add an origin where the requirement was originally defined at,
which is common for projects working with issue trackers such as Jira.

The `gs-req-1.sub-1` requirement combines the two ways to set up a hierarchical structure in *mantra*
that allows to create non-cyclical relations between requirements.
Using the *dot-notation* style, the requirement is set as child of `gs-req-1`,
and `gs-req-2` is set as parent via the explizit `parents` list.
Although good practice would be to list `gs-req-1` in the `parents` list again to help readability,
it is not strictly required.

Besides the mandatory fields, `gs-req-2` also sets `manual_verification`,
which marks the requirement and all its children to require manual verification via at least one review.
Marking `gs-req-1.sub-1` as `optional` tells *mantra* that parent requirements may be `verified`
even if `gs-req-1.sub-1` is `unverified` or `skipped`.

### Tracing Requirements in Code

Currently, *mantra* is only able to detect traces in Rust code that match the macros defined in the [mantra-rust-macros](langs/rust/mantra-rust-macros) crate.
Note that it is not required to use this crate, because trace detection is only based on macro names.
For other languages and file formats, external tools may convert extracted data into the
[AnnotationSchema](schema-gen/generated/collect/AnnotationSchema.json).

If you want to use the macros from [mantra-rust-macros](langs/rust/mantra-rust-macros),
add it to your Cargo.toml via

```sh
cargo add mantra-rust-macros
```

To express relations between requirements and traces, *mantra* supports the following kinds:
- `clarifies` ... Use to trace data that provides additional information about a requirement e.g. diagrams
- `satisfies` ... Use to trace to implementations of a requirement
- `verifies` ... Use to trace to tests or assertions that verify a requirement
- `links` ... Plain trace to link to a requirement

The following code contains satisfying and verifying traces for `gs-req-1`:

```rust
#[mantra_rust_macros::req_satisfied("gs-req-1")]
fn foo() -> bool {
    // ...
    true
}

#[test]
fn test_foo_1() {
    mantra_rust_macros::assert_req!("gs-req-1" => foo(), "Verification using assert macro");
}

#[mantra_rust_macros::req_test("gs-req-1")]
#[test]
fn test_foo_2() {
    core::assert!(foo(), "Verification using attribute macro");
}
```

### Mapping Requirements to Tests

*mantra* uses line coverage metrics collected from tests to detect if a requirement has been verified.
Consequently, a *verifying* trace alone is not enough to verify a requirement.
The benefit is that combining traces with line coverage allows users to choose between manual tracing effort and safety guarantees.
For example, if a *verifying* trace is set on a test and there is a code part with a *satisfying* trace to the same requirement, *mantra* checks if the test actually passed this code part.

For Rust projects, a convenient way to get test and coverage results that are readable by *mantra*
is to use [cargo-nextest](https://nexte.st/) with the JUnit feature and [grcov](https://github.com/mozilla/grcov/) with the Cobertura output format. See the `testcov` task in the [justfile](justfile) of the repository to see how this is set up for *mantra*.
This convenience layer is internally converted to the [TestRunSchema](schema-gen/generated/collect/TestRunSchema.json), which external tools may target directly. 

**Note:** Code coverage for Rust projects collected via `-Cinstrument-coverage` is only collected per binary.
Consequently, it is not possible to get code coverage results per test case, which worsens the safety guarantees
that *mantra* can verify.

### Write Reviews

Often, requirements cannot easily be tested automatically.
For such requirements, *mantra* allows to set the `manual_verification` flag
to explicitly state that this requirement must be verified manually in a review.
Besides verifying requirements, reviews may also be used to overwrite test results,
which is for example useful in case of flaky tests.

Currently, can only be added following the [ReviewSchema](schema-gen/generated/collect/ReviewSchema.json).
External tools may be used to extract data from other formats and convert to the schema.

The review below verifies the manual requirement `gs-req-2`:

```json5
{
    reviews: [{
        name: "First review",
        utc_date: "2026-05-17T17:00utc-01",
        authors: ["Manuel"],
        revisions: [{
            nr: 1,
            authors: ["Manuel"],
            comment: "Initial review"
        }],
        requirements: [{
            id: "gs-req-2",
            comment: "Verifying the requirement"
        }]
    }]
}
```

### Collect & Report

Once all data that should be collected by *mantra* is available, run

```sh
mantra collect
```

This will create a SQLite file in the current working directory that contains all collected data.
You may change the path to the SQLite file by setting the `--url` argument.
Running the command again will update the database and remove outdated data.

After collecting everything, run

```sh
mantra report --output-dir mantra-report/
```

This will store report output under `mantra-report`.
By default, the report format will be HTML and is built up like a static website with the entry point located at `mantra-report/index.html`.

Currently, *mantra* supports HTML and JSON as output formats. The JSON output is internally used
to generate the HTML report using Jinja2 templates.

# License

MIT Licensed
