# GateLisp

GateLisp is a Lisp-shaped hardware description language. Its frontend now has
three distinct representations:

```text
.glisp source
→ generic S-expression reader
→ GateLisp syntax AST
→ name resolution and type checking
→ Typed HIR
→ VHDL AST
→ VHDL-2008
```

The syntax AST supports modules containing `ports`, `wire`, `reg`, `assign`,
`clocked`, and `instance`. Hardware types are `bit`, `(unsigned width)`, and `(signed width)`.
The semantic analyzer resolves every signal to a `SignalId`, rejects duplicate
or undeclared signals and invalid drivers, and converts built-in operators to
typed enums.

## Sequential logic

Registers are updated in rising-edge `clocked` blocks:

```lisp
(clocked clk
  (reset :sync rst
    (next count 0))
  (next count (+ count 1)))
```

Both active-high synchronous (`:sync`) and asynchronous (`:async`) resets are
supported. The clock edge is currently fixed to rising. A `next` target must be
a register, and a register may be driven by only one clocked block. Conditional
updates use the existing `if` expression. When a reset is present, its target
set must exactly match the normal `next` target set.

A `reg` initializer and a reset value are separate concepts and may differ.
The initializer's eventual synthesis meaning remains undecided.

Supported operators are `not`, `and`, `or`, `xor`, `+`, `-`, `=`, `/=`, `<`,
`<=`, `>`, `>=`, and `if`. Addition and subtraction return the same fixed-width
type as their operands; their hardware overflow behavior is wrapping.

Types must match exactly. GateLisp performs no implicit conversion between
`bit`, signed vectors, unsigned vectors, or different widths. An untyped integer
literal is the sole exception: it is checked against an expected type supplied
by an assignment, register initializer, branch, or typed sibling operand.

Multiple clock domains can be represented. Clock-domain-crossing safety checks
are not implemented, and GateLisp does not insert synchronizers automatically;
safe crossings are currently the designer's responsibility.

## Command line

With no mode option, `glispc` emits synthesizable VHDL-2008. VHDL lowering is
performed only from Typed HIR, through a dedicated VHDL AST and deterministic
formatter. The available display modes are:

```console
cargo run --bin glispc -- examples/typed_adder.glisp
cargo run --bin glispc -- --vhdl examples/counter.glisp -o counter.vhd
cargo run --bin glispc -- --hir examples/counter.glisp
cargo run --bin glispc -- --ast examples/typed_adder.glisp
cargo run --bin glispc -- --sexpr examples/typed_adder.glisp
```

`-o` and `--output` write VHDL to a file and are valid only in VHDL mode.

## VHDL backend

`bit` lowers to `std_logic`; unsigned and signed vectors lower to the matching
`numeric_std` types. Inputs are entity ports. Outputs use architecture-local
signals, followed by assignments to entity output ports, so reading an output
inside GateLisp has unambiguous behavior.

Comparisons return `std_logic` through a generated boolean conversion helper.
Expression-valued `if` nodes are lowered into process-local temporary variables
and complete VHDL `if` statements. Continuous assignments use `process(all)`.
Clocked blocks use rising-edge processes with active-high synchronous or
asynchronous reset structure.

Integers are emitted as fixed 64-bit hexadecimal bit strings, type-qualified as
`unsigned` or `signed`, then resized to the target width. This avoids depending
on the implementation-defined VHDL `integer` range. Integer register initializers
become VHDL signal initializations; whether an FPGA implements them depends on
the synthesis tool and target device. Non-literal register initializers are
currently rejected by the VHDL backend.

Generated names use stable IDs plus a lowercase ASCII-sanitized source name,
such as `gl_m0_and_gate`, `gl_p0_clk`, and `gl_s4_count`. IDs guarantee that
case-only differences, reserved words, Unicode, and punctuation cannot collide.

## Module hierarchy

Modules instantiate another module declared anywhere in the same program with
named port connections:

```lisp
(instance gate0 and-gate
  (ports
    (a left)
    (b right)
    (y result)))
```

Every target port must be connected exactly once; connection order is free and
is normalized to the child's declaration order. Actuals are currently signal
symbols only. Child inputs may read any parent port, wire, or register. Child
outputs may drive only parent outputs and wires. Types must match exactly, and
an instance output is a driver, so it cannot share its actual with an `assign`
or another instance output.

Instance names are case-sensitive and occupy a namespace separate from signals.
Target resolution does not depend on module declaration order. Recursive module
dependencies are rejected; VHDL design units are emitted in a stable,
child-before-parent topological order, followed by testbenches. Instances use
VHDL entity direct instantiation and named port maps, so existing testbenches can
exercise a hierarchical top module.

Constant or expression port actuals, external modules, and cross-file module
lookup are not implemented yet.

## Generic widths

Modules may declare compile-time `:natural` and `:positive` generics before
their `ports` form. Defaults are required:

```lisp
(generics
  (channels :natural 1)
  (width :positive 8))
```

Vector widths accept generic references and compile-time `+` and `*`
expressions, such as `(unsigned (+ width 1))`. Width expressions are normalized,
resolved to `GenericId`s, and must be provably at least one for every permitted
generic value. Generics are not runtime signals and cannot be read by `assign`
or other runtime expressions.

Instances use an optional named generic map before their port map:

```lisp
(instance bank0 register-bank
  (generics (width parent-width))
  (ports (input input-data) (value output-data)))
```

Bindings may contain literals, parent generics, `+`, and `*`. Omitted bindings
use the target module's default. Child port types are substituted before exact
port type checking. Testbench targets support the same `(generics ...)` form,
and their generated signals and literal checks use the substituted widths.

GateLisp does not specialize or duplicate generic modules. It emits VHDL entity
generic clauses, named generic maps, symbolic vector ranges, and symbolic
`resize` sizes. An integer used with a generic-width vector must fit at the
minimum possible width.

Boolean, string, and enum generics; subtraction and division in constant
expressions; generate constructs; and reading generic values as runtime signals
are not implemented.

## Testbenches and simulation

GateLisp sources may contain top-level `testbench` forms alongside modules. A
testbench names its DUT with `(target module-name)`, declares zero or more
generated clocks, and ends with a `stimulus` block containing `drive`, `wait`,
`wait-rising`, and `assert` statements. Supported time units are `fs`, `ps`,
`ns`, `us`, `ms`, and `sec`; a clock period denotes one complete cycle.

Testbenches may observe all DUT ports, but may drive only non-clock input ports.
Internal wires and registers are intentionally invisible. Inputs are initialized
to zero, while outputs are observation-only. After `wait-rising`, the generated
testbench waits an additional 1 fs so register and combinational delta-cycle
updates are visible to the following assertion. Stimulus completion reports a
PASS message, calls VHDL-2008 `std.env.stop`, and waits permanently.

Run testbenches with GHDL using:

```console
cargo run --bin glispc -- test examples/counter_test.glisp
cargo run --bin glispc -- test examples/counter_test.glisp --vcd target/counter.vcd
```

PowerShell uses the same commands on one line. `--testbench <name>` selects one
testbench; `--ghdl <path>` selects the executable; and `--work-dir <path>` sets
the isolated simulation work root. Each testbench receives its own work
directory. GHDL is required for `glispc test`. VCD files can be viewed with a
waveform viewer such as GTKWave.

When GHDL is installed, generated output can be analyzed as VHDL-2008:

```console
ghdl -a --std=08 counter.vhd
```

Source files use the `.glisp` extension. `Position::offset` is a zero-based
UTF-8 byte offset. Line and column numbers are one-based, columns count Unicode
scalar values, and spans are half-open.

FSMs, generics, macros, random stimulus, file I/O, and CDC analysis are not
implemented. Synchronizers are not inserted automatically.
