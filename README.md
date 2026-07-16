# GateLisp

GateLisp is a Lisp-shaped hardware description language. Its frontend now has
three distinct representations:

```text
.glisp source
→ generic S-expression reader
→ GateLisp syntax AST
→ name resolution and type checking
→ Typed HIR
```

The syntax AST supports modules containing `ports`, `wire`, `reg`, and
`assign`. Hardware types are `bit`, `(unsigned width)`, and `(signed width)`.
The semantic analyzer resolves every signal to a `SignalId`, rejects duplicate
or undeclared signals and invalid drivers, and converts built-in operators to
typed enums.

Supported operators are `not`, `and`, `or`, `xor`, `+`, `-`, `=`, `/=`, `<`,
`<=`, `>`, `>=`, and `if`. Addition and subtraction return the same fixed-width
type as their operands; their hardware overflow behavior is wrapping.

Types must match exactly. GateLisp performs no implicit conversion between
`bit`, signed vectors, unsigned vectors, or different widths. An untyped integer
literal is the sole exception: it is checked against an expected type supplied
by an assignment, register initializer, branch, or typed sibling operand.

Register initializers are preserved and type-checked, but their eventual
synthesis meaning—power-up value, reset value, or simulation-only value—has not
yet been decided.

## Command line

With no mode option, `glispc` displays Typed HIR. The explicit display modes are:

```console
cargo run --bin glispc -- examples/typed_adder.glisp
cargo run --bin glispc -- --hir examples/typed_adder.glisp
cargo run --bin glispc -- --ast examples/typed_adder.glisp
cargo run --bin glispc -- --sexpr examples/typed_adder.glisp
```

Source files use the `.glisp` extension. `Position::offset` is a zero-based
UTF-8 byte offset. Line and column numbers are one-based, columns count Unicode
scalar values, and spans are half-open.

Clocked logic, resets, FSMs, module instances, macros, explicit conversions,
hardware IR lowering, and VHDL generation are not implemented. VHDL will be
generated from a later hardware representation, never directly from an
S-expression or syntax AST.
