# ADR 068のためのRustの学習

## このファイルの目的

ADR 068は、Cruxに代わる自前のコアを、共用ライブラリ（予定の名前はmiso）として作ると決めた。
その形のうち、「非同期の処理が`Model`を変える方法」と「Cruxとの関係」は、まだ決まっていない（ADR 068の「このADRで決めないこと」）。
これを決めるADR（予定の番号は069）の議論には、開発者がRustの仕組みを理解している必要がある。
このファイルは、その学習の記録である。

このファイルは、議論のたびに参照し、更新する。
1つの項目を開発者が「理解した」と言ったら、その項目をADRに書いてよい。

## 説明の進め方

- 1つの項目ずつ説明する。
  開発者が理解したと言うまで、次の項目に進まない。
- 例は、PetaJournalの今のコードではなく、一般的な短いコードで書く。
  今のコードを例にすると、既存の名前や事情が混ざり、仕組みそのものが見えにくくなるためである。
- 例のコードは、単独でコンパイルして動くものにする。
  実際にコンパイルし、決まりから外れた例のコンパイラの出力も、そのまま載せる。
- JavaやC#に対応するものがあれば、それと並べる。

## ここまでの経緯

### 移植する言語の調査（2026-11-20）

Rustの問題は、コンパイル時間、ビルドのキャッシュの大きさ、レビューの難しさである。
Rustをやめる候補として、主な言語とマイナーな言語を比べた。

- C#とF#は、Androidで共有ライブラリを作る手段が実験的なので外した。
- Kotlin MultiplatformとSwiftは、コンパイル時間で外した。
- Odin、Zig、Flixなどのマイナーな言語は、メモリーの安全性、JVMだけで動くこと、エコシステムのどれかで外した。
- Rustをやめるなら、残る候補はGoだけである。

まずRustのままでビルドを軽くする。
検討事項として、次のissueを作った。

- [#874](https://github.com/sosuisen/peta-journal/issues/874)：IPADICの辞書を、埋め込まずに実行時に読む
- [#875](https://github.com/sosuisen/peta-journal/issues/875)：serdeとfacetの`derive`を、どちらかに寄せる

### エフェクトの考え方

Rustに代数的エフェクトを持ち込むライブラリとして、effing-mad（nightly専用）とcorophage（stableで動く）を調べた。
corophageをそのまま使っても、#767の内部のイベントは消えない。
`.await`のあいだは、`&mut Model`を持てないためである。
また、corophageは型を重く使うので、コンパイル時間が延びる。

開発者は、「考え方を持ち帰るだけでは甘い」と判断した。
考え方から外れたコードは、コンパイルエラーか静的解析のエラーになるべきである。
そこで、自前のコアで、次の10の決まりを静的に強制する案を出した。

## 静的に強制する10の決まり

| # | 決まり | 外れたときのエラー | 学習の状況 |
|---|---|---|---|
| 1 | エフェクトの答えの型は、エフェクトごとに決まる | コンパイルエラー | 説明中 |
| 2 | 処理が起こせるエフェクトは、宣言したものだけ | コンパイルエラー | 未 |
| 3 | 処理は、答えを待っている間`Model`を持てない | コンパイルエラー | 未 |
| 4 | `.await`のあとの`Model`の変更は、データとして送る | コンパイルエラー | 未 |
| 5 | 最新の`Model`に基づく判断は、変更のデータの中で行う | コンパイルエラー | 未 |
| 6 | 内部の答えは、シェルの`Event`を通らない | コンパイルエラー | 未 |
| 7 | 並行して待つ処理も、答えの型が決まっている | コンパイルエラー | 未 |
| 8 | コアは、I/O、時計、スレッドを直接使わない | clippyのエラー | 未 |
| 9 | コアは、I/Oをするcrateに依存しない | cargo-denyか依存の検査のエラー | 未 |
| 10 | シェルは、エフェクトのすべての種類を扱う | 各言語のコンパイルエラー | 未 |

これとは別に、決まりそのものが壊れていないことを、trybuildの「コンパイルが失敗すること」のテストで確かめる。

### 言葉

- エフェクト：コアが外にお願いすること。
  例は、「サーバーで項目を分ける」「タイマーを待つ」「画面を描き直す」である。
  1つのエフェクトを、1つの`struct`で表す。
- 処理：1回の操作で動く、非同期の手順。
- `Ctx`：処理がエフェクトをお願いする窓口。
  処理は、これを通してだけ外と話す。

## 基礎：traitの文法

traitは、「この型はこういうことができる」という約束である。
Javaの`interface`にあたる。
1番の説明の途中で、開発者がtraitの文法を分からないと言ったので、4つの段階に分けて説明した。
例はすべて、単独でコンパイルして動く。

### 段階1：メソッドを1つ持つtrait

```rust
trait Speak {                        // 「Speak」という約束を作る
    fn speak(&self) -> String;       // 約束の中身：speak を呼ぶと String を返すこと
}

struct Dog;                          // 型を2つ作る（中身のない型）
struct Cat;

impl Speak for Dog {                 // Dog は Speak の約束を守る
    fn speak(&self) -> String {      // 約束どおりの形で、中身を書く
        String::from("wan")
    }
}
impl Speak for Cat {                 // Cat も Speak の約束を守る
    fn speak(&self) -> String {
        String::from("nya")
    }
}

fn main() {
    println!("{}", Dog.speak());     // wan
    println!("{}", Cat.speak());     // nya
}
```

文法の要点を図にする。

```
trait Speak {  fn speak(&self) -> String;  }
  |     |           |      |         |    |
 約束  名前      メソッド  自分自身  戻り値 ここで終わり（中身は書かない）
```

- `trait`の中のメソッドは、`;`で終わる。
  中身（`{ ... }`）は書かず、形だけを決める。
- `&self`は、Javaの`this`にあたる。
  「自分自身を読む」という意味である。
- 中身は、`impl Speak for Dog { ... }`の中で、型ごとに書く。

Javaで書くと、次のようになる。

```java
interface Speak { String speak(); }
class Dog implements Speak { public String speak() { return "wan"; } }
class Cat implements Speak { public String speak() { return "nya"; } }
```

### 段階2：traitを引数の条件に使う

```rust
fn twice(x: &impl Speak) -> String {     // Speak の約束を守る型なら何でも受け取る
    format!("{0} {0}", x.speak())
}

fn main() {
    println!("{}", twice(&Dog));   // wan wan
    println!("{}", twice(&Cat));   // nya nya
}
```

`&impl Speak`は、「`Speak`の約束を守る型なら何でもよい」という意味である。
Javaの`String twice(Speak x)`にあたる。

### 段階3：約束を守らない型を渡すと

```rust
struct Stone;          // Stone には impl Speak を書かない

fn main() {
    println!("{}", twice(&Stone));
}
```

コンパイラの出力である。

```
error[E0277]: the trait bound `Stone: Speak` is not satisfied
  --> step3.rs:21:26
   |
21 |     println!("{}", twice(&Stone));
   |                    ----- ^^^^^^ unsatisfied trait bound
   |
help: the trait `Speak` is not implemented for `Stone`
```

「`Stone`は`Speak`を実装していない」と、動かす前に止まる。
この「約束を守らない型は通さない」が、静的な強制の基本の道具である。
10の決まりの多くは、これを使う。

### 段階4：型も約束に含める（`type`）

traitは、メソッドだけでなく、型も約束に含められる。

```rust
trait Holder {
    type Item;                       // 約束：Item という型を1つ決めること
    fn get(&self) -> Self::Item;     // 約束：get を呼ぶと、その Item を返すこと
}

struct NumberHolder;
struct WordHolder;

impl Holder for NumberHolder {
    type Item = u32;                 // NumberHolder の Item は u32
    fn get(&self) -> u32 { 42 }
}
impl Holder for WordHolder {
    type Item = String;              // WordHolder の Item は String
    fn get(&self) -> String { String::from("hello") }
}

fn main() {
    let n = NumberHolder.get();      // n は u32
    let w = WordHolder.get();        // w は String
    println!("{} {}", n + 1, w.to_uppercase());   // 43 HELLO
}
```

- `type Item;`は、「型を1つ決めること」という約束である。
  名前の`Item`は任意である。
- `Self::Item`は、「この約束を守る型が決めた`Item`」という意味である。
  `Self`は、`impl`の対象の型（ここでは`NumberHolder`か`WordHolder`）を指す。
- `impl`の中で`type Item = u32;`と書くと、`NumberHolder`については`Item`が`u32`に決まる。

1番の`trait Effect { type Output; }`は、この段階4の形から、メソッドを除いたものである。
「エフェクトごとに答えの型を1つ決めること」だけを約束にしている。

## 1. エフェクトの答えの型は、エフェクトごとに決まる

### 仕組み

エフェクトの`struct`ごとに、答えの型を1つ決める。
答える側も、使う側も、その型に従う。
答える側が別の型を返すと、コンパイルエラーになる。

### 例のコード

単独でコンパイルして動く。

```rust
// 約束：どのエフェクトも、答えの型を1つ決める
trait Effect {
    type Output;
}

// エフェクトを2つ作る。ただの struct
struct GetName { id: u32 }   // 「名前を教えて」というお願い
struct GetAge  { id: u32 }   // 「年齢を教えて」というお願い

// それぞれの答えの型を決める
impl Effect for GetName { type Output = String; }   // 名前の答えは String
impl Effect for GetAge  { type Output = u32; }      // 年齢の答えは u32

// 答える側の約束：エフェクト E を受け取り、E の Output を返す
trait Handler<E: Effect> {
    fn handle(&mut self, effect: E) -> E::Output;
}

// テスト用の偽物。2つのエフェクトに答える
struct Fake;

impl Handler<GetName> for Fake {
    fn handle(&mut self, effect: GetName) -> String {
        format!("user{}", effect.id)
    }
}
impl Handler<GetAge> for Fake {
    fn handle(&mut self, effect: GetAge) -> u32 {
        20 + effect.id
    }
}

// ロジック：エフェクトをお願いするだけ。答えの型は書いていない。
// コンパイラが、エフェクトの Output から型を知る
fn greet(h: &mut (impl Handler<GetName> + Handler<GetAge>)) -> String {
    let name = h.handle(GetName { id: 7 });   // name は String
    let age  = h.handle(GetAge  { id: 7 });   // age は u32
    format!("{name} is {age}")
}

fn main() {
    println!("{}", greet(&mut Fake));   // user7 is 27
}
```

### 流れの図

```
  greet  --- GetName {id: 7} --->  Fake  --- "user7" (String) --->  name
  greet  --- GetAge  {id: 7} --->  Fake  ---  27     (u32)    --->  age
                  |                              ^
                  +---- type Output = u32 -------+  この型以外は返せない
```

### 決まりから外れた例

偽物が、年齢の答えとして文字列を返す。

```rust
impl Handler<GetAge> for Fake {
    fn handle(&mut self, effect: GetAge) -> u32 {
        let _ = effect; String::from("twenty")   // 年齢なのに文字列を返す
    }
}
```

コンパイラの出力である。

```
error[E0308]: mismatched types
  --> ng.rs:37:25
   |
36 |     fn handle(&mut self, effect: GetAge) -> u32 {
   |                                             --- expected `u32` because of return type
37 |         let _ = effect; String::from("twenty")
   |                         ^^^^^^^^^^^^^^^^^^^^^^ expected `u32`, found `String`
```

### 効果

- 偽の答えの間違いが、テストを動かす前にコンパイルエラーになる。
- 使う側で、答えの種類を調べる`match`が要らなくなる。
- エフェクトの定義を見れば、答えの型が1行で分かる。

### 強制できないこと

答えの型は強制できるが、答えの中身が正しいかは強制できない。
中身は、今までどおりテストで確かめる。

### 質問と答え

#### `impl Effect for GetName { ... }`の構文

3つの部品がある。

- `trait Effect { type Output; }`は、約束の定義である。
  Javaの`interface`にあたる。
- `struct GetName { id: u32 }`は、データの型である。
  Javaの`record GetName(int id)`にあたる。
- `impl Effect for GetName { type Output = String; }`は、「`GetName`は`Effect`の約束を守る。その`Output`は`String`とする」という意味である。
  Javaの`implements`にあたる。

Javaで書くと、型パラメーターを持つインターフェースが近い。

```java
interface Effect<O> { }                     // trait Effect { type Output; }
record GetName(int id)
    implements Effect<String> { }           // impl Effect for GetName { type Output = String; }
```

Rustでは、データの定義（`struct`）と、約束を守ること（`impl ... for ...`）を別々に書く。
Javaでは、`implements`をクラスの宣言に書く。

Rustにも`trait Effect<O>`の書き方はある。
しかしその書き方では、1つの型が`Effect<String>`と`Effect<u32>`の両方を実装できてしまい、答えの型が1つに決まらない。
`type Output`の書き方では、1つの型が`Effect`を実装できるのは1回だけである。
そのため、答えの型は必ず1つに決まる。

#### `Output`は任意の名前か

任意の名前である。
`trait`を定義する人が決める。
`Answer`でも`Reply`でもよい。
`trait`の中で決めた名前と、`impl`の中で書く名前が同じであればよい。

`Output`を例にしたのは、Rustの標準ライブラリの`Future`トレイトが、同じ使い方で`type Output`を持つためである。
`async`の処理を`.await`すると、この`Output`の型の値が返る。
PetaJournalでは、エフェクトの答えであることがはっきりする`Answer`のほうが分かりやすいかもしれない。
名前はADRを書くときに決める。

## 次にすること

- 「基礎：traitの文法」の段階1〜4で、分からないところがないかを確かめる。
- traitの文法が分かったら、1番の例のコードに戻り、1番を理解したかを確かめる。
- 理解したら、2番に進む。
