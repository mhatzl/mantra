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

Every requirement must have a heading or title starting with a unique requirement ID followed by `:`.
A requirement hierarchy may be used to create a structure of high and low-level requirements.

**Example:**

```
# my_req_id: Some requirement title

A high-level requirement.

## my_req_id.sub_req_id: Some sub-requirement title

A low-level requirement of `my_req_id`.
```

## Referencing

Requirement IDs may be referenced in the implementation and/or tests of your system/project using the syntax `[req:your_req_id]`.
This syntax should be independent enough in most programming languages that *mantra* can distinguish an expression from a requirement reference.

**Example:**

```rust
/// This function does something.
///
/// [req:my_req_id]
fn my_function() {
  //...
}
```

# License

MIT Licensed
