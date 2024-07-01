# mantra-rust-macros

This crate provides procedural macros to trace requirements using `mantra`.
It also offers features to get requirement coverage via logs.

## Traces

Requirement traces may be created using either the attribute macro `req`,
or the function-like macro `reqcov`.
See `mantra-lang-tracing` on how to specify requirement IDs using `req` or `reqcov`.

The attribute macro may be set anywhere attribute macros are allowed.
At the moment, coverage logs are only generated if the macro is set on a function.

**Examples:**

```rust
use mantra_rust_macros::{req, reqcov};

/// Coverage log is generated
#[req(fn_trace)]
fn some_fn() {
    // coverage log is generated
    reqcov!(function_like_trace);
}

#[req(const_trace)]
const SOME_CONST: usize = 1;

#[req(type_trace)]
type SomeType = bool;

#[req(struct_trace)]
struct SomeStruct {
    /// Attribute macros cannot be set for fields.
    some_field: bool,
}

#[req(mod_trace)]
mod some_mod {}

#[req(trait_trace)]
trait SomeTrait {
    #[req(trait_type_trace)]
    type A;

    /// Coverage log is generated
    #[req(trait_fn_default_trace)]
    fn some_trait_fn() {}
}
```

## Automatic documentation

Requirements set using the `#[req()]` attribute will generate a `Requirements` section in the documentation.
The requirement IDs are represented as bullet list entries.
If the environmental variable `MANTRA_REQUIREMENT_BASE_URL` is set, all IDs will be transformed to links with the content of the variable as prefix.

```
/// Documentation for IDs is automatically generated.
#[req(req_1, req_2)]
fn some_fn() {}
```

Generated documentation with `MANTRA_REQUIREMENT_BASE_URL` set to `https://github.com/mhatzl/mantra/wiki/5-Requirements/`:

```
Documentation for IDs is automatically generated.

# Requirements

- [req_1](https://github.com/mhatzl/mantra/wiki/5-Requirements/req_1)
- [req_2](https://github.com/mhatzl/mantra/wiki/5-Requirements/req_2)
```

## Coverage log

- Feature `log`

  Enabling this feature will create coverage **TRACE** logs using the `log` crate.

- Feature `stdout`

  Enabling this feature will print coverage logs to stdout.

- Feature `defmt`

  Enabling this feature will print coverage logs using the `defmt` crate.
  This is intended for embedded devices.

**Examples:**

```
#[req(fn_trace)]
fn some_fn() {}
```

The generated coverage log for the code above has the form:

```
mantra: req-id=`fn_trace`; file='<resolved by file!()>'; line='<resolved by line!()>';
```

## Trace extraction

The `extract` feature may be enabled to extract coverage data from logs.
With the feature enabled, the functions `extract_first_coverage()` or `extract_covered_reqs()` may be used.
