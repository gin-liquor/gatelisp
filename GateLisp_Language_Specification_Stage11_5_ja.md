# GateLisp 言語仕様書・利用説明書

**対象実装:** Stage 11.5  
**文書版:** Draft 0.11.5  
**作成日:** 2026-07-17

---

## 1. はじめに

GateLispは、LispのS式記法を用いてFPGA向け同期デジタル回路を記述し、型検査済みの中間表現を経由してVHDLへ変換するハードウェア記述言語です。

一般的なプログラミング言語としてLispプログラムを逐次実行するのではなく、S式から組み合わせ回路、レジスタ、クロック同期処理、ROM、レジスタ配列などの回路構造を生成します。

GateLispが重視する設計原則は次のとおりです。

1. **明示的な型とビット幅**
2. **暗黙変換を最小化した予測可能な回路生成**
3. **コンパイル時に証明できない範囲アクセスの拒否**
4. **Typed HIRとBackendの二重検証**
5. **一般的なVHDLと`numeric_std`への移植性の高いLowering**
6. **FPGA上のタイミングとリソースを意識できる意味論**
7. **Lispらしい簡潔な構文と、HDLとしての静的安全性の両立**

本書は、Stage 11.5までに実装された機能を中心に、利用者向け説明と実装仕様をまとめたものです。

---

## 2. 本書の規約

本書では次の語を使用します。

- **必須**: 実装または利用者が従わなければならない規則
- **許可**: 正しいGateLispプログラムとして受理される構成
- **禁止**: コンパイルエラーにしなければならない構成
- **未実装**: 現在の言語段階では受理されない将来機能
- **静的**: コンパイル時に値または範囲を確定・証明できること
- **runtime値**: signal、port、register、ROM読出しなど、回路動作中に変化する値
- **Backend再検証**: Typed HIRが壊れていても不正なVHDLを生成しないための検査

コード例は現在のGateLispで使用している記法を基準とします。モジュール外枠、port宣言、reset指定など、リポジトリ内で既に定義されている構文の詳細は実装側のgrammarを優先します。

---

# Part I — 利用者向けガイド

## 3. GateLispで記述するもの

GateLispでは、主に次の回路を記述できます。

- 組み合わせ論理
- signed／unsignedベクタ演算
- 条件選択
- ビット列の切出し、連結、反転、シフト、ローテート
- クロック同期レジスタ更新
- Rising／Falling edge回路
- 同期／非同期resetを持つ回路
- FSM
- encoded enumによる状態・opcode表現
- compile-time固定ROM
- 動的アドレスのレジスタ配列
- マイクロコード実行機

GateLispの式は、通常、回路上の値または組み合わせ回路を表します。`clocked`内の文は、クロックエッジで実行されるレジスタ更新または配列書込みを表します。

---

## 4. 基本的な型

### 4.1 `bit`

単一ビットを表します。

概念上の値は次の2つです。

```text
0
1
```

VHDLでは通常`std_logic`へLoweringされます。

`bit`と1bitの`unsigned`／`signed`ベクタは別型です。暗黙に同一視しません。

### 4.2 `unsigned`

固定幅の符号なしベクタです。

```lisp
(unsigned 8)
```

は8bit unsignedを表します。

VHDLでは次のようになります。

```vhdl
unsigned(7 downto 0)
```

### 4.3 `signed`

固定幅の2の補数ベクタです。

```lisp
(signed 16)
```

VHDLでは次のようになります。

```vhdl
signed(15 downto 0)
```

### 4.4 encoded enum

enumは、固定幅unsignedを基底表現とする独立型です。

```lisp
(enum State
  :width 2
  (IDLE  0)
  (RUN   1)
  (DONE  2)
  (FAULT 3))
```

メンバーは修飾名で参照します。

```lisp
State.IDLE
State.RUN
```

enumは同じ幅のunsignedとは異なる型です。

```lisp
(set! state 1)
```

のようなraw整数からenumへの暗黙変換は禁止です。

別enum型も、幅や符号値が同じでも互換ではありません。

---

## 5. 整数リテラルと期待型

整数リテラルは、周囲から型と幅が明確に与えられる場合に型付けされます。

例として、8bit unsigned registerへ代入する値なら、

```lisp
(next counter 1)
```

の`1`を8bit unsignedとして扱える場合があります。

ただし、GateLispは無制限な暗黙型変換を行いません。

特に次の操作では、sourceに外側の期待型を伝播させない設計を採用しています。

- `reverse-bits`
- static bit motion
- `bit-at`
- `rom-read`
- `register-array-read`
- enum変換のsource

したがって次は拒否されます。

```lisp
(reverse-bits 1)
(bit-at 1 0)
(enum-from-bits Opcode 1)
```

周囲がベクタ型を期待していても、生整数を暗黙にベクタ化しません。

---

## 6. 組み合わせ式

### 6.1 `if`

`if`は値を返す式です。

```lisp
(if condition
    true-value
    false-value)
```

両branchは同じ型、幅、signednessでなければなりません。

```lisp
(if enable
    data-a
    data-b)
```

結果に外側の期待型がある場合、その期待型はbranchへ伝播できます。

### 6.2 等価比較

同じ型同士を比較し、結果として`bit`を返します。

```lisp
(= counter 0)
(= state State.IDLE)
```

enumと整数、enumとunsigned、異なるenum型同士の暗黙比較は禁止です。

### 6.3 `concat`

複数のビット列を連結します。

```lisp
(concat upper lower)
```

enumを直接`concat`へ渡すことは禁止です。必要な場合は明示変換します。

```lisp
(concat prefix (enum-to-bits state))
```

### 6.4 slice

ベクタの静的範囲を切り出します。

```lisp
(slice instruction 23 19)
```

24bit命令の上位5bitを取り出す例です。

範囲はコンパイル時に検証されます。

### 6.5 resizeと明示変換

幅やsignednessを変更する場合は、既存の明示的なresize／変換構文を使用します。

GateLispは、演算や代入のために暗黙resizeを挿入しません。

---

## 7. ビット操作

### 7.1 `reverse-bits`

ベクタ内のビット順を反転します。

```lisp
(reverse-bits data)
```

許可されるsource:

- unsigned vector
- signed vector

結果:

- sourceと同じ幅
- unsigned

signed sourceは同幅unsignedへ変換した後に処理されます。

`bit`および生整数リテラルは拒否されます。

### 7.2 static bit motion

次の5種類があります。

```lisp
(shift-left source amount)
(shift-right-logical source amount)
(shift-right-arithmetic source amount)
(rotate-left source amount)
(rotate-right source amount)
```

`amount`はruntime値ではなく、名前解決・正規化済みの静的幅式です。

型規則:

- `shift-left`: 対応するベクタ型
- `shift-right-logical`: unsigned限定
- `shift-right-arithmetic`: signed限定
- `rotate-left`: 対応するベクタ型
- `rotate-right`: 対応するベクタ型

結果はsourceの型と幅を維持します。

安全条件は静的に証明されます。

```text
amount + 1 <= source-width
```

幅以上のamountに対する暗黙moduloやclampは行いません。

### 7.3 `bit-at`

ベクタの静的indexから1bitを選択します。

```lisp
(bit-at source index)
```

例:

```lisp
(bit-at data 0)
(bit-at data (- WIDTH 1))
(bit-at (reverse-bits data) 3)
```

source:

- unsigned vector
- signed vector

結果:

- `bit`

index:

- 静的な`WidthExpr`
- runtime signalは不可

安全条件:

```text
index + 1 <= source-width
```

genericのdefault値だけを見て範囲内と判断しません。宣言上の制約から安全性を証明できる必要があります。

---

## 8. 多分岐

### 8.1 `case`式

`case`は値を返す多分岐式です。

```lisp
(case selector
  (0 value-a)
  (1 value-b)
  (2 value-c)
  (else default-value))
```

規則:

- selectorは`bit`、`unsigned`、`signed`、enum
- 通常armは1個以上
- `else`必須
- `else`は1個だけ
- `else`は最後
- labelは静的値
- label重複は禁止
- 全resultは同じ型・幅・signedness
- fallthroughなし

enum selectorでは、同じenum型のメンバーだけをlabelに使用できます。

```lisp
(case state
  (State.IDLE idle-value)
  (State.RUN run-value)
  (State.DONE done-value)
  (else fault-value))
```

### 8.2 `case-do`

`case-do`はclocked内で複数文を選択実行する文です。

```lisp
(clocked clk
  (case-do state
    (State.IDLE
      (next busy 0)
      (next state State.RUN))

    (State.RUN
      (next busy 1)
      (next state State.DONE))

    (else
      (next busy 0)
      (next state State.FAULT))))
```

規則:

- `clocked`内限定
- 値を返さない
- arm内に複数文を記述可能
- `else`は省略可能
- `else`省略時、一致しなければ何も更新しない
- clocked文脈では未更新registerは値を保持
- fallthroughなし
- 異なるarmは相互排他的

同じregisterを異なるarmで更新することは、単一のclocked driverとして許可されます。

別々のclocked blockから同じregisterを更新することは禁止です。

---

## 9. クロック同期処理

### 9.1 暗黙Rising edge

従来構文:

```lisp
(clocked clk
  ...)
```

はRising edgeを意味します。

VHDL:

```vhdl
if rising_edge(clk) then
```

### 9.2 明示Rising edge

```lisp
(clocked (rising clk)
  ...)
```

### 9.3 Falling edge

```lisp
(clocked (falling clk)
  ...)
```

VHDL:

```vhdl
if falling_edge(clk) then
```

### 9.4 エッジ混在

同じclock signalをRisingとFallingの両方で使用すること自体は可能です。

ただし、異なるedge間の信号経路は半周期タイミングになる可能性があります。

```text
Rising → Falling
Falling → Rising
```

同一clock domainでは、原則としてedgeを統一することを推奨します。

同じregisterまたはregister-arrayを、Rising側とFalling側の別clocked blockから駆動することは禁止です。

---

## 10. encoded enum

### 10.1 宣言

```lisp
(enum Opcode
  :width 5
  (NOP       0)
  (WRITE-IMM 1)
  (TRANSFER  2)
  (JUMP      3)
  (JUMP-IF   4)
  (HALT      31))
```

制約:

- `:width`必須
- 幅は1以上の具体的整数
- symbolic幅は未実装
- memberは1個以上
- member値は明示的な非負整数
- 自動採番なし
- member名重複禁止
- encoded value重複禁止
- valueは宣言幅に収まること

### 10.2 enumの許可操作

- 同じenum型への代入
- register初期値
- reset値
- equality
- `if`のresult
- `case`のselector／label／result
- `case-do`のselector／label
- port、signal、register型
- module間の同じenum型接続

### 10.3 enumの禁止操作

enumはunsignedとして暗黙利用できません。

直接は禁止:

```lisp
(+ state 1)
(shift-left state 1)
(reverse-bits state)
(bit-at state 0)
(slice state 1 0)
(concat state value)
```

必要な場合は`enum-to-bits`を使用します。

### 10.4 `enum-from-bits`

unsigned vectorをenum型へ明示変換します。

```lisp
(enum-from-bits Opcode opcode-bits)
```

条件:

- sourceはunsigned vector
- source幅とenum幅が完全一致
- signed、bit、整数リテラルは不可

未定義のencoded valueも保持できます。

例えばOpcodeで0～4だけを宣言していても、5bit値31をOpcode型として保持できます。

未定義値は`case`／`case-do`の`else`で処理します。

### 10.5 `enum-to-bits`

enumを同幅unsignedへ明示変換します。

```lisp
(enum-to-bits state)
```

これは同幅reinterpretであり、追加のハードウェアを必要としません。

---

## 11. ROM

### 11.1 宣言

```lisp
(rom microcode
  :address-width 8
  :data-width 24
  :default 0

  (0  0)
  (1  524544)
  (2  1049346))
```

ROMはcompile-time固定の読出し専用メモリです。

制約:

- address-widthは具体的な正整数
- data-widthは具体的な正整数
- dataはunsigned
- `:default`必須
- sparse entryを許可
- entry address重複禁止
- addressとdataは非負整数
- addressはaddress-widthに収まること
- dataはdata-widthに収まること
- symbolic幅は未実装
- ROM書込みは存在しない

エントリ順は意味に影響しません。Backendは決定的なVHDL生成のため、アドレス順へ正規化できます。

### 11.2 `rom-read`

```lisp
(rom-read microcode micro-pc)
```

address:

- ROMのaddress-widthと同じ幅のunsigned
- integer literalは期待型により受理可能
- signed、bit、enum、幅不一致は禁止

結果:

```text
unsigned(data-width)
```

`rom-read`自体は組み合わせ式です。

```lisp
(define instruction
  (rom-read microcode micro-pc))
```

addressが変化すればinstructionも組み合わせ的に変化します。

### 11.3 登録読出し

ROM出力をregisterへ取り込む場合は、clocked内で明示します。

```lisp
(register instruction (unsigned 24) 0)

(clocked clk
  (next instruction
    (rom-read microcode micro-pc)))
```

GateLispは`rom-read`へ暗黙の1サイクル遅延を追加しません。

### 11.4 複数read

同じROMを複数のaddressから読むことを許可します。

```lisp
(rom-read table address-a)
(rom-read table address-b)
```

それぞれ論理的なread portです。

合成ツールによってはROM複製や追加MUXが発生します。GateLispは自動port共有を行いません。

---

## 12. レジスタ配列

### 12.1 宣言

```lisp
(register-array registers
  :address-width 8
  :data-width 8
  :initial 0)
```

意味:

- 256要素
- 各要素8bit unsigned
- power-up initialは0

制約:

- concrete widthのみ
- unsigned要素
- 全要素共通initial
- sparse initialは未実装
- arrayをportとして直接公開する機能は未実装

`:initial`はreset値ではありません。

### 12.2 組み合わせread

```lisp
(register-array-read registers address)
```

address:

- exact-width unsigned
- integer literalは期待型により受理可能
- signed、bit、enum、幅不一致は禁止

result:

```text
unsigned(data-width)
```

読出しは組み合わせです。

```lisp
(define source-data
  (register-array-read registers source-address))
```

複数readは複数の論理read portを表します。

### 12.3 同期write

```lisp
(clocked clk
  (register-array-write
    registers
    destination-address
    write-data))
```

規則:

- `clocked`内限定
- addressはexact-width unsigned
- valueはexact-width unsigned
- 1配列につき1つの論理write port
- writeされないサイクルは内容保持
- 暗黙のwrite enable構文は不要
- 同一実行経路で同一配列へ2回writeは禁止

### 12.4 `case-do`からのwrite

異なるarmは相互排他的なので、同一配列へのwriteを許可します。

```lisp
(clocked clk
  (case-do opcode
    (Opcode.WRITE-IMM
      (register-array-write
        registers
        destination-address
        immediate-data))

    (Opcode.TRANSFER
      (register-array-write
        registers
        destination-address
        source-data))

    (else
      (next fault 1))))
```

同じarm内で同一配列へ複数writeすることは禁止です。

別clocked blockから同じ配列へwriteすることも禁止です。

### 12.5 read-after-write意味論

同じclocked blockでreadとwriteを行う場合、RHSは更新前の配列内容を読みます。

```lisp
(clocked clk
  (next captured
    (register-array-read registers address))

  (register-array-write
    registers
    address
    new-value))
```

意味:

```text
clocked RHSのread  = old value
edge後のcomb read  = new value
```

GateLispは自動bypass、write-first、no-change modeを生成しません。

### 12.6 reset

reset中は通常のarray writeを実行しません。

Stage 11.5では、resetによる全要素初期化は行いません。配列内容は保持します。

理由:

- 全要素resetは大規模回路になり得る
- RAM推論を妨げる
- 1 write portの意味論と合わない

---

# Part II — 言語仕様

## 13. 型同一性

型の一致は、単なるbit幅だけでは決まりません。

次はそれぞれ異なる型です。

```text
bit
unsigned(1)
signed(1)
State
Opcode
```

同じ幅の別enumも異なる型です。

```text
State(width=2)
Mode(width=2)
```

代入、比較、`if`、`case` resultでは型同一性を検査します。

---

## 14. signedness

GateLispはsignednessを明示的に扱います。

- unsignedとsignedの暗黙変換は禁止
- logical rightはunsigned限定
- arithmetic rightはsigned限定
- enumの基底表現はunsignedだが、GateLisp上ではunsignedとは別型
- ROMとregister-arrayのdataはunsigned

signed値をunsignedとして扱う場合は明示変換が必要です。

---

## 15. 幅式

幅は具体的整数または、実装が対応する正規化済みsymbolic expressionで表現されます。

Stage 10.5では`bit-at`のMSB表現のため、限定的に次を扱います。

```lisp
(- WIDTH 1)
```

この減算表現を一般的なruntime arithmeticへ拡大してはいけません。

幅・index・amountの安全性は、genericのdefault値ではなく、宣言された制約と正規化された幅式から証明します。

---

## 16. 静的範囲証明

### 16.1 static bit motion

```text
amount + 1 <= source-width
```

### 16.2 `bit-at`

```text
index + 1 <= source-width
```

証明できないアクセスは拒否します。

暗黙modulo、wraparound、clampは行いません。

---

## 17. selectorと期待型

`case`のresultに外側の期待型を伝播できます。

```lisp
(next output
  (case selector
    (0 1)
    (1 2)
    (else 3)))
```

ただし、その結果期待型をselectorへ伝播してはいけません。

selectorはselector自身の情報で型付けされます。

同様に、ビット操作やROM読出しのsourceへ外側の期待型を誤って伝播してはいけません。

---

## 18. 静的case label

通常の`case`／`case-do` labelはcompile-time値です。

整数selectorでは、selector型へ正規化して範囲検査します。

enum selectorでは、同じenum型のenum memberのみを許可します。

label重複は、ソース表記ではなく型付け・正規化後の値で判定します。

---

## 19. clocked region

1つの`clocked`は1つのclocked regionを形成します。

regionは少なくとも次を持ちます。

- clock signal
- edge: RisingまたはFalling
- reset情報
- enable情報
- body
- register driver集合
- register-array write driver集合

同じscalar registerまたはregister-arrayを複数regionから更新することは禁止です。

---

## 20. driver規則

### 20.1 scalar register

許可:

- 同じ`case-do`の異なるarmから更新
- 同じclocked region内の相互排他的更新

禁止:

- 別clocked blockから更新
- 異なるclockから更新
- RisingとFallingの別regionから更新

### 20.2 register-array

許可:

- 1つのclocked regionからwrite
- 同じ`case-do`の異なるarmから1回ずつwrite
- 異なるarrayへ同時に各1回write

禁止:

- 同一実行経路で同一arrayへ複数write
- 同じarm内で同一arrayへ複数write
- 別clocked regionからwrite
- Rising／Fallingの別regionからwrite

---

## 21. Backend再検証

GateLisp BackendはTyped HIRを完全には信用しません。

少なくとも次を再検証します。

- 型種別
- 幅
- signedness
- GenericId
- EnumId／EnumMemberId
- RomId
- RegisterArrayId
- module所属
- static index／amount範囲
- case label型・範囲・重複
- result型
- clock型
- edge
- driver競合
- ROM初期値
- register-array初期値
- VHDL名衝突

不正HIRに対して`panic!`、`unwrap`、`expect`で停止せず、Backend errorを返すことを原則とします。

---

# Part III — VHDL Lowering

## 22. 基本型

| GateLisp | VHDL |
|---|---|
| `bit` | `std_logic` |
| `unsigned(N)` | `unsigned(N-1 downto 0)` |
| `signed(N)` | `signed(N-1 downto 0)` |
| encoded enum | 同幅`unsigned` |

GateLisp enumの型安全性はfrontendとBackendで保証します。VHDLネイティブenumにはLoweringしません。

---

## 23. ビット操作

static bit motionは`numeric_std`へ直接Loweringします。

| GateLisp | VHDL |
|---|---|
| `shift-left` | `shift_left` |
| `shift-right-logical` | `shift_right` |
| `shift-right-arithmetic` | `shift_right` |
| `rotate-left` | `rotate_left` |
| `rotate-right` | `rotate_right` |

`reverse-bits`と`bit-at`は、必要なarchitectureにのみhelperを1回生成します。

---

## 24. `case`／`case-do`

Stage 10.75では、VHDLの型付き`if`／`elsif`／`else`列へLoweringします。

selectorが複合式の場合はtemporaryへ一度だけ評価します。

概念例:

```vhdl
case_selector_temp <= selector_expression;

if case_selector_temp = LABEL_A then
  ...
elsif case_selector_temp = LABEL_B then
  ...
else
  ...
end if;
```

fallthroughは生成しません。

---

## 25. enum

enum memberはarchitecture-local constantへLoweringします。

```vhdl
constant GL_ENUM_STATE_IDLE :
  unsigned(1 downto 0) := "00";
```

enum signal／registerはunsignedです。

```vhdl
signal state : unsigned(1 downto 0);
```

`enum-from-bits`と`enum-to-bits`は同幅unsigned間のreinterpretなので、原則としてゼロコストです。

---

## 26. ROM

概念的なVHDL:

```vhdl
type gl_rom_microcode_t is array (
  0 to 255
) of unsigned(23 downto 0);

constant gl_rom_microcode : gl_rom_microcode_t := (
  0 => "000000000000000000000000",
  1 => "000010000000000100000000",
  others => "000000000000000000000000"
);
```

read:

```vhdl
gl_rom_microcode(to_integer(address))
```

大きなdata値は、VHDL integer範囲へ依存しない固定幅bit-stringとして生成します。

---

## 27. register-array

概念的なVHDL:

```vhdl
type gl_register_array_registers_t is array (
  0 to 255
) of unsigned(7 downto 0);

signal gl_register_array_registers :
  gl_register_array_registers_t :=
  (others => "00000000");
```

read:

```vhdl
gl_register_array_registers(to_integer(read_address))
```

write:

```vhdl
if rising_edge(clk) then
  gl_register_array_registers(
    to_integer(write_address)
  ) <= write_data;
end if;
```

GateLispはBRAM利用を保証しません。組み合わせread＋同期writeは、デバイスと合成ツールによりFF配列、distributed RAM、LUT RAMなどへ推論されます。

---

# Part IV — 実用例

## 28. enumを使ったFSM

```lisp
(enum State
  :width 2
  (IDLE  0)
  (RUN   1)
  (DONE  2)
  (FAULT 3))

(register state State State.IDLE)
(register busy bit 0)
(register done bit 0)

(clocked clk
  (case-do state
    (State.IDLE
      (next busy 0)
      (next done 0)
      (next state
        (if start
            State.RUN
            State.IDLE)))

    (State.RUN
      (next busy 1)
      (next done 0)
      (next state
        (if finished
            State.DONE
            State.RUN)))

    (State.DONE
      (next busy 0)
      (next done 1)
      (next state
        (if clear
            State.IDLE
            State.DONE)))

    (else
      (next busy 0)
      (next done 0)
      (next state State.FAULT))))
```

これは出力もclockedで更新する登録出力型FSMです。

---

## 29. 24bitマイクロコード形式

```text
23          19 18       16 15           8 7            0
+-------------+-----------+---------------+--------------+
| OPCODE 5bit | FLAG 3bit | ADDRESS 8bit  | DATA 8bit    |
+-------------+-----------+---------------+--------------+
```

命令例:

| Opcode | 動作 |
|---|---|
| `NOP` | 何もしない |
| `WRITE-IMM` | `registers[address] <- data` |
| `TRANSFER` | `registers[address] <- registers[data]` |
| `JUMP` | `micro-pc <- address` |
| `JUMP-IF` | 条件成立時`micro-pc <- address` |
| `HALT` | 停止 |

---

## 30. マイクロコード実行機

```lisp
(enum Opcode
  :width 5
  (NOP       0)
  (WRITE-IMM 1)
  (TRANSFER  2)
  (JUMP      3)
  (JUMP-IF   4)
  (HALT      31))

(rom microcode
  :address-width 8
  :data-width 24
  :default 16252928

  (0 524544)
  (1 2163712)
  (2 524885)
  (3 1574144)
  (4 524970)
  (5 1049346)
  (6 16252928))

(register-array registers
  :address-width 8
  :data-width 8
  :initial 0)

(register micro-pc  (unsigned 8) 0)
(register zero-flag bit          0)
(register halted    bit          0)
(register fault     bit          0)

(define instruction
  (rom-read microcode micro-pc))

(define opcode
  (enum-from-bits Opcode
    (slice instruction 23 19)))

(define flags
  (slice instruction 18 16))

(define address
  (slice instruction 15 8))

(define data
  (slice instruction 7 0))

(define source-data
  (register-array-read registers data))

(define branch-taken
  (case flags
    (0 1)
    (1 zero-flag)
    (2 (= zero-flag 0))
    (else 0)))

(clocked clk
  (case-do opcode
    (Opcode.NOP
      (next micro-pc
        (+ micro-pc 1)))

    (Opcode.WRITE-IMM
      (register-array-write
        registers
        address
        data)

      (next zero-flag
        (= data 0))

      (next micro-pc
        (+ micro-pc 1)))

    (Opcode.TRANSFER
      (register-array-write
        registers
        address
        source-data)

      (next zero-flag
        (= source-data 0))

      (next micro-pc
        (+ micro-pc 1)))

    (Opcode.JUMP
      (next micro-pc address))

    (Opcode.JUMP-IF
      (next micro-pc
        (if branch-taken
            address
            (+ micro-pc 1))))

    (Opcode.HALT
      (next halted 1))

    (else
      (next fault 1)
      (next halted 1))))
```

この構成はROMを組み合わせで読み、1クロックにつき1命令を実行します。

Block RAM向けの同期ROM構成にする場合は、instruction registerとFETCH／EXECUTE状態を使用します。

---

# Part V — 診断と安全性

## 31. 主なコンパイルエラー

### 型不一致

```lisp
(set! state 1)
```

enum registerへ整数を暗黙代入できません。

### 幅不一致

```lisp
(rom-read microcode unsigned-7bit-address)
```

8bit address ROMへ7bit値を渡せません。

### 静的範囲外

```lisp
(bit-at data 8)
```

dataが8bitなら有効indexは0～7です。

### case label重複

```lisp
(case selector
  (0 a)
  (0 b)
  (else c))
```

### driver競合

```lisp
(clocked clk-a
  (next state State.RUN))

(clocked clk-b
  (next state State.IDLE))
```

### register-array write競合

```lisp
(clocked clk
  (register-array-write regs addr-a value-a)
  (register-array-write regs addr-b value-b))
```

同一サイクルに2 write portが必要になるため拒否されます。

---

## 32. 設計上の注意

### 32.1 Rising／Falling混在

同じclockでedgeを混在させると、半周期タイミング経路が発生します。

### 32.2 複数ROM read

複数read portはROM複製につながる可能性があります。

### 32.3 複数register-array read

複数の組み合わせreadは大きなMUXまたはdistributed RAM複製になる可能性があります。

### 32.4 initial値

register-arrayの`:initial`が物理FPGAのpower-up値として実現されるかは、デバイスと合成ツールに依存します。

### 32.5 reset

大規模配列をresetで全clearしない設計です。必要な場合は、利用者が状態管理や初期化シーケンスを設計します。

---

# Part VI — 実装段階

## 33. 実装済み主要Stage

| Stage | 機能 |
|---|---|
| 9.5 | `reverse-bits` |
| 10 | static shift／rotate |
| 10.5 | static `bit-at`、Falling edge |
| 10.75 | `case`式、clocked限定`case-do` |
| 10.9 | encoded enum |
| 11 | compile-time固定ROM |
| 11.5 | 動的レジスタ配列 |

---

## 34. 現在意図的に未実装の機能

- runtime shift amount
- runtime `bit-at`
- dynamic slice
- 暗黙modulo
- switch fallthrough
- 複数labelを1armへまとめる構文
- range／mask／wildcard case label
- 組み合わせ文脈の`case-do`
- nested `case-do`
- VHDLネイティブenum
- symbolic enum width
- enum自動採番
- enum validity自動回路
- ROM書込み
- 外部ROM初期化ファイル
- signed ROM
- enum型ROM
- 複数register-array write port
- synchronous read RAM
- dual-port RAM
- byte enable
- vendor primitive
- BRAM使用保証
- 自動bypass
- 自動FSM符号化
- 自動マイクロコードassembler

---

## 35. 今後の候補

今後の拡張候補は次のとおりです。

1. runtime shift
2. dynamic bit／slice操作
3. nested `case-do`
4. 同期read RAM
5. dual-port memory
6. microcode assembler
7. 名前付きmicrocode label
8. enum自動採番
9. enum網羅性検査
10. 構造化microinstruction
11. GHDLによる自動VHDL解析・シミュレーション
12. ベンダー別メモリ推論属性

---

# Appendix A — 簡易構文一覧

```lisp
; 型
bit
(unsigned WIDTH)
(signed WIDTH)
EnumType

; 条件
(if condition true-expr false-expr)
(= lhs rhs)

; ビット操作
(concat a b ...)
(slice value high low)
(reverse-bits value)
(shift-left value amount)
(shift-right-logical value amount)
(shift-right-arithmetic value amount)
(rotate-left value amount)
(rotate-right value amount)
(bit-at value index)

; 多分岐式
(case selector
  (label result)
  ...
  (else default-result))

; clocked
(clocked clk ...)
(clocked (rising clk) ...)
(clocked (falling clk) ...)

; 多分岐文
(case-do selector
  (label statement ...)
  ...
  (else statement ...))

; enum
(enum Name
  :width WIDTH
  (MEMBER VALUE)
  ...)

(enum-from-bits EnumType unsigned-value)
(enum-to-bits enum-value)

; ROM
(rom name
  :address-width A
  :data-width D
  :default VALUE
  (ADDRESS VALUE)
  ...)

(rom-read name address)

; register array
(register-array name
  :address-width A
  :data-width D
  :initial VALUE)

(register-array-read name address)

(register-array-write name address value)
```

---

# Appendix B — GateLispの設計哲学

GateLispは、便利さのために回路の意味を曖昧にするよりも、明示的な記述と静的検証を優先します。

そのため、次の方針を一貫して採用します。

- signednessを暗黙に変えない
- 幅を暗黙に変えない
- enumを整数として暗黙利用しない
- 範囲外アクセスをwrapしない
- 複数writeへ暗黙の優先順位を付けない
- clock edgeを暗黙に混在させない
- memory latencyを暗黙に追加しない
- BackendでもHIRを再検証する
- FPGA固有最適化より、まず正しい一般VHDLを生成する

この方針により、生成される回路の型、幅、クロック、メモリ動作、driver構造をソースコードから予測しやすくします。
