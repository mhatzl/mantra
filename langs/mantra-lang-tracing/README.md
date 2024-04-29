# mantra-lang-tracing

This crate contains functionality needed to collect requirement traces from code or plain text files.
Traces are a link between requirement definitions and where they are implemented in code.

## Usage
### Plain text collector

The `PlainCollector` may be used to collect traces from plain text files.
Traces with the form `[req(<requirement id(s)>)]` are collected line by line.

**Example:**

```
This line has a valid trace. [req(valid_trace)]

Traces cannot span multiple lines. [req(
invalid_trace)]
```

### AST collector

The `AstCollector` may be used to collect traces from the abstract syntax tree of a code file.
It uses [`tree-sitter`](https://tree-sitter.github.io/tree-sitter/) to create the AST,
and allows to define a collector function that tries to collect traces from AST nodes.

This collector may be used as a base to create trace collectors for programming languages.

### Specifying requirement IDs

The `extract_req_ids*()` functions offer a consistent way to extract requirement IDs from traces.
They should be used in custom collector implementations, to keep the same syntax for specifying IDs.

**Syntax:**

```
ids = id , {(",", id)} ;
id = {(id_part, "." )}, id_part ;
id_part = escaped_id_part | unescaped_id_part ;
escaped_id_part = '"', ?any char except '"', '`', or '.'?, '"' ;
unescaped_id_part = ( digit | rust_identifier ) , { digit | rust_identifier } ;
```

**Examples:**

```
"escaped-id".sub_id

1234."digit-only-main-id"

first_id, second_id
```
