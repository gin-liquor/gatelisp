# GateLisp

GateLisp is a Lisp-shaped hardware description language. The compiler frontend
currently provides both a generic S-expression reader and conversion into a
GateLisp-specific AST.

Supported module syntax includes `ports`, `wire`, `reg`, and `assign`. Types are
`bit`, `(unsigned width)`, and `(signed width)`. Expressions may be references,
signed integers, or general call forms.

Source files use the `.glisp` extension. Parse a file into the GateLisp AST with:

```console
cargo run --bin glispc -- examples/and_gate.glisp
```

Inspect the intermediate S-expression AST with:

```console
cargo run --bin glispc -- --sexpr examples/and_gate.glisp
```

`Position::offset` is a zero-based UTF-8 byte offset. Line and column numbers
are one-based; columns count Unicode scalar values. Spans are half-open.

Name resolution, type checking, clocked logic, resets, FSMs, module instances,
hardware IR, and VHDL generation are not implemented yet. GateLisp-specific AST
nodes will be lowered through typed hardware IR; VHDL will not be generated
directly from the S-expression or GateLisp AST.
