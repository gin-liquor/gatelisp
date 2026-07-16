# GateLisp S-expression reader

This crate implements the first GateLisp compiler stage: a generic lexer and
S-expression parser. It deliberately performs no GateLisp-specific semantic
analysis.

`Position::offset` is a zero-based UTF-8 byte offset. Line and column numbers
are one-based; columns count Unicode scalar values.

Run the example with:

```console
cargo run -- examples/and_gate.gatelisp
```
