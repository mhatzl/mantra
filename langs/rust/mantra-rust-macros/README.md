# mantra-rust-macros

This crate provides macros to trace requirements using `mantra`.

## Traces

Requirement traces may be created using either attribute or function-like macros.
See `mantra-lang-tracing` on how to specify requirement IDs using `req` or `reqcov`.

**Examples:**

```rust
use mantra_rust_macros::{req_satisfied, verify_req};

/// Tells mantra that `some_fn()` satisfies requirement `req-1`.
#[req_satisfied("req-1")]
fn some_fn() {
    // Tells mantra that `check_something()` verifies `req-2`. 
    verify_req!("req-2" => check_something());
}

#[req_satisfied("const_trace")]
const SOME_CONST: usize = 1;

#[req_satisfied("type_trace")]
type SomeType = bool;

#[req_satisfied("struct_trace")]
struct SomeStruct {
    /// Attribute macros cannot be set for fields.
    some_field: bool,
}

#[req_satisfied("mod_trace")]
mod some_mod {}

#[req_satisfied("trait_trace")]
trait SomeTrait {
    #[req_satisfied("trait_type_trace")]
    type A;

    #[req_satisfied("trait_fn_default_trace")]
    fn some_trait_fn() {}
}
```
