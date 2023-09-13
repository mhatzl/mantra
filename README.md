# mantra

**M**anuel's **AN**forderungs-**TRA**cing

*mantra* is a tool for easier tracing between requirements, implementation, and tests.

Checkout the [requirements](https://github.com/mhatzl/mantra/wiki/5-Requirements) section in [mantra's wiki](https://github.com/mhatzl/mantra/wiki)
to see how requirement tracing with *mantra* looks.

## Core Concepts

To use *mantra*, a system/project must document requirements in a way that allows automatic edits at a textual level.
Wikis are a good way to achieve this, and most project management tools offer at least a lightweight wiki.
*mantra* is primarily built to work together with the structure of the [wiki-repo-template](https://github.com/mhatzl/wiki-repo-template) for GitHub wikis,
but it should be relatively simple to adapt other wikis to use *mantra*. 

Human-readable IDs are used to identify requirements, and reference them in the implementation and/or tests.
These requirement IDs must be set manually on the implementation and test side.
*mantra* then adds the number of references a requirement has on the implementation and test side to the wiki.
Since systems/projects may have different branches, these numbers are linked to the branch *mantra* is run against.

**Example:**

```
# my_req_id: Some requirement title

**References:**

- in branch main: 3
- in branch stable: 1
```

### Requirement Structure

Every requirement must have a heading starting with a unique requirement ID followed by `:` and a title.
A requirement hierarchy may be used to create a structure of high and low-level requirements.

**Example:**

```
# my_req_id: Some requirement title

A high-level requirement.

## my_req_id.sub_req_id: Some sub-requirement title

A low-level requirement of `my_req_id`.
```

### Referencing

Requirement IDs may be referenced in the implementation and/or tests of your system/project using the syntax `[req:req_id]`.\
This syntax should be independent enough in most programming languages that *mantra* can distinguish an expression from a requirement reference.

**Example:**

```rust
/// This function does something.
///
/// [req:req_id]
fn my_function() {
  //...
}
```

### Multiple repositories

A system/project may have multiple repositories, but only one wiki to manage requirements.
Therefore, *mantra* offers options to set the name of a repository in addition to the branch name.
The repository name is then added to the entry in the *references* list.

**Note:** It is not possible to reference multiple wikis, because requirements should be kept in one place.

**Example:**

```
**References:**

- in repo frontend_repo in branch main: 3
- in repo backend_repo in branch main: 1
```

## Usage
### Commands

*mantra* is primarily a command line tool that offers various commands.\
Use `mantra <command> --help` to see all available options per command.

***mantra* currently offers the following commands:**

- `check` ... Used to check if the wiki structure is valid, and all references refer to requirements in the wiki

  `mantra check ./requirements_folder ./project_folder`

  **Possible output:**

  ```md
  --------------------------------------------------------------------
  `mantra check` ran successfully for branch: main

  **Checks:**

  - All references refer to existing requirements
  - No deprecated requirement referenced
  - No duplicate requirement IDs in wiki
  - All entries in *references* lists are valid

  **Increased direct references for 1 requirement:**

  - req:wiki.ref_list.repo references: 6 -> 9

  Took: 87ms
  ```

  **Note:** This command outputs all found errors before the summary shown above.

- `release` ... Used to create release reports to list all *active* requirements

  `mantra release --release-tag=v0.2.10 ./requirements_folder`

  **Possible output:**

  ```md
  ***Active* requirements in release v0.2.10:**

  - check: Validate wiki and references
    - check.ci: Add check overview as PR comment
  - filter: Use ignore files to restrict the search for references
  - qa: Quality Assurance
    - qa.DoD: Have a "Definition of Done" for requirements
    - qa.pipeline: Pipeline to ensure a high library quality
      - qa.pipeline.1_style: Ensure consistent formatting
      - qa.pipeline.2_lint: Ensure good coding standard
      - qa.pipeline.3_build: Ensure *evident* builds
      - qa.pipeline.4_tests: Ensure tests still pass
    - qa.sustain: Consider sustainability during design and development
    - qa.tracing: Use requirement IDs in [mantra](https://github.com/mhatzl/mantra)
  - ref_req: Reference requirements
    - ref_req.ignore: Ignore requirement references
    - ref_req.test: Test requirement referencing
  - release: Release report
    - release.checklist: Checklist for requirements marked with *manual* flag
  - req_id: Requirement ID
    - req_id.sub_req_id: Sub-requirements for high-level requirements
  - status: Show wiki status
    - status.branch: See status for one branch in the wiki
    - status.cmp: Compare status of two branches in the wiki
  - sync: Synchronize wiki, implementation, and tests
    - sync.ci: CI support for *mantra sync*
  - wiki: Documentation for requirements
    - wiki.ref_list: *References* list
      - wiki.ref_list.branch_link: Link to branches
      - wiki.ref_list.deprecated: Mark requirements as *deprecated* in specific branches
      - wiki.ref_list.manual: Mark requirements to require manual verification
      - wiki.ref_list.repo: Handle multiple repositories for one wiki

  Took: 40ms
  ```

  **Note:** The option `--checklist` may be set to create a checklist for all requirements that require *manual* verification.

- `status` ... Shows the current state of the wiki

  `mantra status --detail-ready ./requirements_folder`

  **Possible output:**

  ```md
  **Wiki status for branch `main`:**

  - 1 requirement is *ready* to be implemented
  - 30 requirements are *active*
  - 0 requirements are *deprecated*
  - 0 requirements are need *manual* verification

  ***Ready* requirements:**

  - qa.ux: Experience using *mantra*

  Took: 34ms
  ```

  **Note:** The `--detail-<phase>` options may be used to list requirement IDs that are in this phase 

  **Compare branches:**

  It is possible to compare two branches using the `status` command.
  
  `mantra status --branch=main --cmp-branch=stable ./requirements_folder`

  **Possible output:**

  ```md
  **Wiki differences between `main` and `stable`:**

  | REQ-ID         | main       | stable |
  | -------------- | ---------- | ------ |
  | `ref_req.test` | deprecated | active |
  ```

- `sync` ... Used to synchronize reference counter between wiki and project

  `mantra sync ./requirements_folder ./project_folder`

  **Note:** This command stops at the first encountered error. You may want to use `mantra check` to get all errors at once.

### CI/CD

For better automation, some commands may be used in CI/CD pipelines.
A docker image is available at [manuelhatzl/mantra](https://hub.docker.com/r/manuelhatzl/mantra) for easier integration.
The image exposes `mantra` without any predefined command, so it may be used like an installed command line tool.

**This repository itself uses *mantra* in the following GitHub workflows:**

- [mantra_pr.yml](/.github/workflows/mantra_pr.yml) ... Uses `check` to create a comment in PR conversations
- [mantra.yml](/.github/workflows/mantra.yml) ... Uses `sync` to synchronize references between wiki and project
- [release-please.yml](.github/workflows/release-please.yml) ... Uses `release` to add a release report to a created release

### Skip content for the reference search

Files and folders may be ignored for the references search, by adding them to `.gitignore` or `.mantraignore` files.
These files and folders are then skipped when searching references in the project.

It is also possible to ignore only the next reference inside a file, by setting `[mantra:ignore_next]` directly before the reference.

**Example:**

```
[mantra:ignore_next]
[req:ignored_req]
```

# License

MIT Licensed
