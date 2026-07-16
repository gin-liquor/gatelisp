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

## Explicit conversions

GateLisp never converts vector widths or signedness implicitly. Four explicit
operations make the intended hardware behavior visible:

```lisp
(resize (unsigned 16) input)
(truncate (unsigned 8) wide-value)
(as-signed unsigned-value)
(as-unsigned signed-value)
```

`resize` accepts only the same signedness and an equal or wider target. It zero
extends unsigned values and sign extends signed values. `truncate` accepts only
the same signedness and an equal or narrower target, preserving the low-order
bits for both signed and unsigned vectors. `as-signed` and `as-unsigned`
reinterpret the existing bits without changing their width. Equal-width
conversions are permitted.

Targets may use generic width expressions. The compiler conservatively proves
the required width relationship from normalized `WidthExpr` structure,
constants, minimum values, and additive terms. If the relationship cannot be
proved for every permitted generic value, compilation fails. A conversion
target does not provide an inferred source width, so a raw integer literal
cannot be the conversion source.

For example, operands in a wider adder must each be extended explicitly:

```lisp
(assign sum
  (+ (resize (unsigned 9) a)
     (resize (unsigned 9) b)))
```

VHDL lowering uses `numeric_std.resize` for extension, `signed(...)` and
`unsigned(...)` for reinterpretation, and architecture-local truncate helpers
that explicitly select the low-order bits. In particular, signed truncation is
not lowered directly to `numeric_std.resize`.

Conversions between `bit` and vectors, saturation, and rounding are not
implemented.

## Bit slicing and concatenation

`(slice source offset width)` extracts a statically known range measured from
the least-significant bit. Offset zero starts at the LSB, and the compiler must
prove `offset + width <= source width`. Offset and width accept the same
compile-time literals, generic references, `+`, and `*` expressions as generic
vector widths. A slice always produces `unsigned`; use `as-signed` when signed
interpretation is intended.

`(concat upper ... lower)` joins two or more raw bit sequences. Its first
operand becomes the MSB side and its last operand becomes the LSB side. `bit`,
`unsigned`, and `signed` operands may be mixed, but standalone integer literals
have no width and are rejected. The result is always `unsigned`, with a width
equal to the checked sum of all operand widths. Nested concatenations are
flattened without changing operand order.

Slice sources and concat operands do not inherit an outer expected type. If a
symbolic boundary cannot be proved safe for every generic value, compilation
fails. VHDL lowering emits explicit ranges and `&` concatenation; bit operands
use an architecture-local one-bit vector helper. The offset/width form was
chosen to make LSB-oriented hardware fields and generic field sizes explicit.

Runtime indices, high/low slice syntax, and individual bit indexing are not implemented.

## Static shifts and rotates

`shift-left`, `shift-right-logical`, `shift-right-arithmetic`, `rotate-left`,
and `rotate-right` preserve the source width and signedness. Left shift and both
rotates accept signed or unsigned vectors; logical right accepts only unsigned,
and arithmetic right accepts only signed. Rotates operate on the raw bit sequence.

The amount is a non-negative compile-time expression made from literals, the
current module's generics, `+`, and `*`. It must be statically provable to be
less than the source width for every legal generic value. Zero is an identity;
width-sized amounts are errors and are never implicitly reduced modulo width.
For a signed logical shift, convert explicitly, for example
`(as-signed (shift-right-logical (as-unsigned value) 1))`.

Nested forms such as `(reverse-bits (rotate-left input 1))` are supported, as
are conversions, slices, concatenations, clocked updates, and testbenches. VHDL
lowering directly uses the `numeric_std` `shift_left`, `shift_right`,
`rotate_left`, and `rotate_right` overloads. Runtime shift amounts, bit sources,
funnel shifts, and carry-producing shifts are not implemented.

## Static bit selection and clock edges

`(bit-at source index)` selects one `bit` from an unsigned or signed vector.
The index is static and must be proven less than the source width without
relying on generic defaults. Runtime indices, negative indices, implicit
modulo, and wraparound are rejected. `(- width 1)` selects the most-significant
bit of a positive generic width. Generated VHDL uses one on-demand `gl_bit_at`
helper per architecture and casts signed sources to equal-width unsigned.

The traditional `(clocked clk ...)` form remains rising-edge triggered.
`(clocked (rising clk) ...)` and `(clocked (falling clk) ...)` lower to
`rising_edge` and `falling_edge`. Both edges may use the same clock, but paths
between them can have only half a clock period, so one edge per clock domain is
normally preferable. Automatic dual-edge or DDR logic is not generated.

## Multi-way selection

`case` is a value-producing expression with one or more statically labeled
arms and a required final `else`. Labels are range-checked using the selector's
`bit`, `unsigned`, or `signed` type, and duplicates are rejected after typing.
All result branches have exactly the same type and width. Case expressions may
be nested and used inside other expressions.

`case-do` is a clocked-only statement for mutually exclusive register updates.
Each arm contains one or more `set!` or `next` updates; its `else` is optional,
and omission means no update for unmatched values. The same register may be
updated in different arms without creating multiple drivers, while different
clocked blocks remain separate drivers. Nested `case-do` is not supported in
this stage. VHDL lowering materializes the selector once and emits a typed
if/else chain without fallthrough or self-assignments.

## Bit-order reversal

`(reverse-bits value)` reverses every bit position of an unsigned or signed
vector: the input LSB becomes the output MSB and the input MSB becomes the
output LSB. The width is preserved, including generic WidthExpr values, and the
result is always `unsigned`; wrap it in `as-signed` when signed interpretation
is required. `bit` and standalone integer operands are intentionally rejected.

FFT bit-reverse addressing is a typical use: address widths are 8, 10, 11, and
12 bits for 256, 1024, 2048, and 4096 points respectively. GateLisp requires
that address width explicitly and does not derive it from the FFT size. The
generated VHDL uses one architecture-local, unconstrained-vector helper with a
loop and `'range`, `'length`, `'low`, and `'high` attributes, so source size
does not grow with vector width. A double reversal restores the original bits.

This operation normally synthesizes mostly as rewired connections, although
physical placement and routing delay are not guaranteed to be zero. Reordering
an entire FFT data array still requires separate RAM/addressing logic; FFT
butterflies and RAM inference are outside the current language scope.

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

## Encoded enums (Stage 10.9)

GateLisp supports fixed-width encoded enums such as
`(enum State :width 2 (IDLE 0) (RUN 1) (DONE 2))`. Widths and member values
are positive, explicit integer literals checked at compile time. Enum members
are referenced as `State.IDLE`; enum types remain distinct from unsigned
vectors even when their widths match.

Use `enum-from-bits` and `enum-to-bits` for explicit exact-width unsigned
conversions. Enum equality, `if`, `case`, and `case-do` are supported, while
arithmetic, ordering, and bit-motion operations require an explicit conversion
to bits. VHDL lowers enum storage to unsigned vectors and emits architecture-
local constants for the members used by that architecture.

## Compile-time ROMs (Stage 11)

Declare a read-only sparse ROM at top level with fixed positive literal widths:
`(rom table :address-width 8 :data-width 24 :default 0 (0 1) (7 42))`.
Defaults and entries must be non-negative values that fit the declared data
width; addresses must be unique and fit the address width. Entries are sorted
by address for deterministic lowering. `(rom-read table address)` requires an
exact-width unsigned address and returns an unsigned vector of the ROM data
width. VHDL emits an architecture-local array constant with sparse assignments
and an `others` default; reads have no registered latency.

## Register arrays (Stage 11.5)

Module-local register arrays use a fixed unsigned address and data width:
`(register-array registers :address-width 8 :data-width 8 :initial 0)`.
`initial` initializes every element at elaboration; it is not a reset or a
sparse initializer. Reads use `(register-array-read registers address)` and
are combinational. Writes use `(register-array-write registers address value)`
only inside a clocked block, with one write port per array and exact-width
unsigned address/data. Different `case-do` arms may write the same array, but
conflicting writes in one arm or clocked region are rejected. Reset branches do
not clear arrays. VHDL lowers each array to an architecture-local array signal.
