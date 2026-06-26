# Contributing to *mantra*

Thank you for considering contributing to *mantra*, it means a lot to us!\
Below are the most important topics to get you started.

- [Discussions](#discussions)
- [Issue/PR Labels](#issuepr-labels)
- [Wiki](#wiki)
- [Development Setup](#development-setup)

## Discussions

We use [GitHub Discussions](https://github.com/mhatzl/mantra/discussions) for exchanges with the community.
It is a good place to start if you have any questions.

## Issue/PR Labels

There are two labels to help possible contributors to get involved:

- [good-first-issue](https://github.com/mhatzl/mantra/labels/good-first-issue) ... This label is used to mark issues that should be easy to implement **without** extensive understanding of the project
- [help-needed](https://github.com/mhatzl/mantra/labels/help-needed) ... This label is used to mark issues, where project members need help to resolve it

For better asynchronous communication, we use the following labels:

- [waiting-on-author](https://github.com/mhatzl/mantra/labels/waiting-on-author) ... This label is used to indicate that the assignee or reviewer is awaiting response from the author
- [waiting-on-reviewer](https://github.com/mhatzl/mantra/labels/waiting-on-reviewer) ... This label is used to indicate that the assignee or author is awaiting response from the reviewer

To keep track of feature requests, we use the following labels:

- [declined](https://github.com/mhatzl/mantra/labels/declined) ... This label is used to mark issues/PRs that they won't be considered/implemented further
- [req-missing-wiki-entry](https://github.com/mhatzl/mantra/labels/req-missing-wiki-entry) ... This label is used to mark `[REQ]` issues that there is not yet a related entry in the wiki
- [req-ready](https://github.com/mhatzl/mantra/labels/req-ready) ... This label is used to mark `[REQ]` issues that they have enough information to be implemented

## Wiki

We use the [GitHub Wiki](https://github.com/mhatzl/mantra/wiki) for developer related information.
For example, it contains [project goals](https://github.com/mhatzl/mantra/wiki/1-Goals-and-Non%E2%80%90Goals), [requirements](https://github.com/mhatzl/mantra/wiki/5-Requirements),
and [decision records](https://github.com/mhatzl/mantra/wiki/6-Decision-Records)

**Note:** Issues for the wiki must be created in this repository, but PRs are handled in the [GitHub repository of the wiki](https://github.com/mhatzl/mantra-wiki).  

## Development Setup

*mantra* is developed using Rust.
See [rustup](https://www.rust-lang.org/tools/install) on how to install the Rust tool chain.

For convenience, we use [just](https://github.com/casey/just) to group common tasks.
See [installation docs](https://just.systems/man/en/installation.html) to setup just.

Although our tests may be executed using `cargo test`, we use [cargo-nextest](https://nexte.st/)
to get a machine-readable JUnit test output. Code coverage is currently collected using [grcov](https://github.com/mozilla/grcov/).

Run `just testcov` to see if your setup is complete.

**Pipeline:**

Every contribution must pass the following steps.

1. `cargo fmt` ... To get consistent code styling
2. `cargo clippy` ... To get better code quality
3. `just testcov` ... Runs all tests via [cargo-nextest](https://nexte.st/) and collects test and code coverage results
4. `just collect` ... Uses mantra to collect requirements traceability information about itself
5. `just report` ... Uses mantra to create a report based on the newly collected data

Please make sure that these steps pass before creating a pull request, to prevent unnecessary workflow runs.
