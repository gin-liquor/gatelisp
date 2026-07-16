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
* 現在依頼された範囲を超える機能を勝手に追加しない。
* 変更後は必ずformat、clippy、testを実行する。
* テストしやすさと分かりやすさを、過度な抽象化より優先する。

## Required validation

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

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
