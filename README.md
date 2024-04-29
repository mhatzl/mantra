# mantra

![build-test](https://github.com/mhatzl/mantra/actions/workflows/rust.yml/badge.svg?branch=main)
[![docker](https://github.com/mhatzl/mantra/actions/workflows/docker.yml/badge.svg?branch=main)](https://hub.docker.com/r/manuelhatzl/mantra)
![mantra-sync](https://github.com/mhatzl/mantra/actions/workflows/mantra.yml/badge.svg?branch=main)

**M**anuels **AN**forderungs-**TRA**cing

*mantra* is a tool for easier tracing between requirements, implementation, and tests.

Checkout the [requirements](https://github.com/mhatzl/mantra/wiki/5-Requirements) section in [mantra's wiki](https://github.com/mhatzl/mantra/wiki)
to see how requirement tracing with *mantra* looks.

## Core Concepts

IDs are used to identify requirements, and reference them in the implementation and/or tests.
These requirement IDs must be set manually on the implementation and test side.
*mantra* then adds available requirements and found traces into a SQL database for further analysis.

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
How requirements IDs are referenced may vary between programming languages.
If no special syntax is defined for a file type, the default is to search for references
having the form `[req(<requirement id(s)>)]`.

**Language specific tracing:**

- **Rust**: Uses `mantra-rust-trace` to collect requirement traces

  **Example:**

  ```rust
  #[req(req_id)]
  fn some_fn() {}
  ```

# License

MIT Licensed
