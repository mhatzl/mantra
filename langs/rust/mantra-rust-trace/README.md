# mantra-rust-trace

Crate providing a collection function to collect requirement traces from rust code,
using the `AstCollector` from `mantra-lang-tracing`.
It collects traces set using `req` or `reqcov` from `mantra-rust-macros`,
and traces set in doc-comments using the form `[req(<requirement id(s)>)]`.

**Examples:**

```rust
use mantra_rust_macros::{req, reqcov};

#[req(fn_trace)]
fn some_fn() {
    reqcov!(function_like_trace);
}

#[req(struct_trace)]
struct SomeStruct {
    /// Attribute macros cannot be set for fields.
    /// But setting a trace in doc-comments works: [req(doc_comment_trace)]
    some_field: bool,
}
```
