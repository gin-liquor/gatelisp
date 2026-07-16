# GateLisp project instructions

GateLispは、Lisp形式でハードウェア回路を記述し、将来的にVHDLへ変換するコンパイラです。

## Development policy

* 実装言語はRustとする。
* 安定版Rustでコンパイルできること。
* unsafeは使用しない。
* ライブラリ内部でpanic、unwrap、expectを原則使用しない。
* エラーはSpan付きのResultとして返す。
* 公開型には必要に応じてrustdocを記述する。
* Lexer、S式Parser、GateLisp意味解析、IR、VHDL生成を分離する。
* S式ASTから直接VHDLを生成しない。
* Typed HIRからVHDL ASTへLoweringし、VHDL ASTとformatterを分離する。
* VHDL名は専用のname mappingを介して生成する。
* VHDL出力の決定性を維持し、Golden Testと利用可能ならGHDLで検証する。
* 現在依頼された範囲を超える機能を勝手に追加しない。
* shift／rotate量はConstExprに限定し、source幅未満をgeneric defaultに頼らず静的証明する。
* logical rightはunsigned、arithmetic rightはsignedに限定し、幅以上を暗黙moduloしない。
* shift／rotateはnumeric_std関数へLoweringし、不要なhelperを生成しない。
* bit-atは静的indexだけを許可し、generic defaultに頼らずsource幅未満と証明する。
* clockedの省略edgeはrisingとし、rising/falling混在時は半周期タイミングに注意する。
* case式とclocked専用case-do文はAST/HIRで分離し、labelはselector型の静的整数として検証する。
* case-doのarmは排他的なためarm間の同一register更新を別ドライバ扱いしない。
* 変更後は必ずformat、clippy、testを実行する。
* テストしやすさと分かりやすさを、過度な抽象化より優先する。

## Required validation

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Encoded enum guidance (Stage 10.9)

Enums use a positive fixed integer width and explicit non-negative member
values. Member references are resolved to `EnumId`/`EnumMemberId`; enum types
are never implicitly interchangeable with unsigned vectors or other enums.
Use `enum-from-bits`/`enum-to-bits` for exact-width unsigned conversions.
Enum values may be used in assignments, equality, `if`, `case`, and `case-do`,
but not directly in arithmetic, ordering, slicing, concatenation, or bit
motion. The VHDL backend represents enums as unsigned and validates enum IDs,
member IDs, encoded values, and widths before emitting architecture-local
constants. Preserve the existing no-`panic!`/`unwrap`/`expect` policy.

## Language concepts

GateLispでは、Lispの構文をコンパイル時の回路生成に利用する。

実行時の動的リスト、ガベージコレクション、動的型付けを、合成対象のハードウェアへそのまま持ち込まない。

将来の変換経路は次の構成を想定する。

```text
GateLisp source
→ S-expression AST
→ GateLisp-specific AST
→ name resolution and type checking
→ typed hardware IR
→ VHDL AST
→ VHDL-2008
```
