# ADR 078: One List Diff in client-core for Every Shell

## Status

Accepted

## Context

ADR 076は、ネイティブのシェルのために、リストの差分と変わったパネルをRustで1回だけ作ると決めた。
ADR 068の進め方の2は、EmacsとWebのシェルも差分を受け取る形にするかを調べるとした。
作業のissueは#859である。
このADRは、差分をどのシェルでも使える形にし、Emacs版がそれを受け取る形を決める。
Emacs版の画面の形（表示モードと編集モード）は、ADR 077で決める。

### 差分の置き場所

ADR 076の差分は、`client-native-bridge`にある。
`diff.rs`の`list_changes`と`ListChange`、`changes.rs`の`Changes`、`Panel`、`OutlinePanel`、`Shown`である。
EmacsとWebのシェルは、`client-native-bridge`を使わず、`client-core`だけを使う。
そのため、今のままでは差分を呼べない。

`list_changes<T>`は、どの型の並びでも比べられる。
一方で、`Changes`の行は`Vec<ListChange<Row<RichText>>>`である。
`Shown::changes`も、`ViewModel<RichText>`だけを受け取る。
本文の形（`BodyModel`）は、シェルごとに違う。
ネイティブは`RichText`、Webは`AstBody`（`client/apps/web-app/outliner/src/body.rs`）、Emacsは`()`である。

### Emacs版のプロトコル

Rustのプログラム（`outliner-emacs`）とLisp（`lisp/peta-journal.el`）は、stdinとstdoutのJSON Linesでやり取りする。
メッセージの型は、`src/protocol.rs`の`Inbound`と`Outbound`である。
JSONは`serde_json`で書く。
Lispは、同じ名前を文字列として自分で書いている。
たとえば、`(plist-get message :page_id)`や、メッセージの種類の`"rows"`である。
2つが合っているかは、Rustのテスト（`rows_are_written_as_json_for_emacs`）とE2Eで見ているだけである。
プロトコルの知識が、RustとLispの2か所にある。

### 型の生成の道具

ネイティブの型は、`facet`で作る。
`facet`は型の形をRustから読めるようにするリフレクションで、`#[derive(Facet)]`で付ける。
`facet_generate`（0.22.3）は、その形を読み、Swift、Kotlin、C#、TypeScriptの型を書く。
運ぶ形は、bincodeとJSONの両方に対応している。
`client-native-bridge/src/codegen.rs`が、これでネイティブの型を作っている。
`schemars`は別の仕組みで、serdeの属性からJSON Schemaを作る。
このリポジトリでは、`journal-api-types`がMCPのツールのために使っている。
`facet_generate`も`schemars`も、Emacs Lispは書けない。

### 計測

#859で、1000項目のページを測った（2026-11-22、AMD Ryzen 7 7700、Emacs 31.1）。
字下げで行が変わるとき、Rustのプログラムは0.68msで、全部の行を120KBのJSONで送る。
Lispは、それを読んで書き直すのに4.28msかかる。
そのうちJSONを読むのは0.45msで、残りは行の位置をたどる処理である。
本文に見た目とリンクを付けた1000行を1度に送る形も、バイトコンパイルしたLispで比べた。
JSONを読み、`put-text-property`で見た目とリンクを付けると1.36msだった。
Lispの読める形（`#("..." 0 2 (face bold))`）を`read`すると0.67msだった。
差分の形なら、1つのキーで送るのは変わった行だけである。
全部の行を送るのは、ページを開いたときだけになる。

### Web版

Web版（Leptos）は、コアと同じwasmの中で動く。
運ぶ形はなく、Rustの値をそのまま受け取る。
`RowStore`（`components/outline.rs`）が全部の行を比べ、`<For key>`がIDの並びを比べる。
#931で、Leptosをやめることも検討している。
候補は、Leptos、Dominator（futures-signals）、ピュアTypeScriptである。

## Decision

差分を`client-core`に移し、本文の形で一般化する。
Emacs版は、その差分を、facetのJSONで書いたJSON Linesで受け取る。
Lispのプロトコルの型は、facetの型の情報から生成する。

### 差分を`client-core`に移す

`list_changes`、`ListChange`、`Changes`、`Panel`、`OutlinePanel`、`Shown`を、`client-native-bridge`から`client-core`に移す。
`Changes`と`Shown`は、本文の形`B`を型の引数に取る（`Changes<B>`、`Shown<B>`）。
差分の作り方は、ADR 076から変えない。
ネイティブのシェルは、`client-core`の`Changes<RichText>`を使う。
ネイティブのシェルが受け取るものは変わらない。
`facet_generate`が型の引数の付いた`Changes<B>`を扱えないときは、`client-native-bridge`に`RichText`で決めた型を1つ置く。
生成したネイティブの型の名前は、今のまま`Changes`にする。

### Emacs版の本文の形

Emacs版の本文の形は、`EmacsText`である。
`EmacsText`はRustの型で、`outliner-emacs`に置く。
`BodyModel::of`で、`RichText`から作る。
`RichText`から作るので、表示の文字の数え方（ADR 043、ADR 044）がネイティブと同じになる。

```rust
/// The body of an item as Emacs shows it: one text, and what each part of it looks like.
pub struct EmacsText {
    /// The shown text. A line break after each paragraph and each line of code.
    pub text: String,
    pub spans: Vec<EmacsSpan>,
}

/// A part of the text with its looks, its link or its image.
pub struct EmacsSpan {
    /// Places in `text`, in characters (code points, ADR 043).
    pub from: u32,
    pub to: u32,
    pub looks: Vec<EmacsLook>,
    pub link: Option<RichLink>,
    /// An image of the assets folder, shown in place of the text of the span.
    pub image: Option<RichImage>,
    /// False for what Rust added, such as a list mark or the spaces of a table: they are
    /// not letters of the shown text (ADR 044).
    pub letters: bool,
}
```

`EmacsLook`は、`RichRun`の`Looks`（太字、斜体、取り消し線、下線、ハイライト、コード、上付き、下付き、スポイラー）と、段落の見出しの深さ、引用、リストの記号を表すenumである。
位置を文字で数えるのは、Emacsのバッファーの位置も文字で数えるためである。
Lispは、`from`と`to`を変換せずに使える。
リストの記号、区切り線、表の列のそろえは、Rustが`text`に入れる。

### Emacs版への運び方

運び方は、今のJSON Linesのままにする。
差分の形なら、1つのキーで送る量は小さく、書式の速さはほとんど効かないためである。
行の変化は、`ListChange`の一覧で送る。
行の形は、今の`RowView`と同じで、`body`が`EmacsText`になる。
ページを開いたときは、`Reset`で全部の行を1回送る。

JSONは、`serde_json`ではなく、facetのJSONで書く。
名前の元を、facetの属性の1つにするためである。
EmacsからRustへのメッセージも、facetのJSONで読む。

### Lispの型を生成する

`facet_generate`の`RegistryBuilder`が作る`Registry`を読み、Lispのプロトコルのファイルを書く生成器を作る。
書くファイルは`lisp/peta-journal-protocol.el`である。
対象は、`Inbound`、`Outbound`、`EmacsText`、`ListChange`と、それらが使う型である。
生成するものは、次の3つである。

- フィールドを読む関数。たとえば`(peta-journal-protocol-span-from span)`。
- 決まった値の一覧。たとえば`EmacsLook`の名前、`bullet`の種類、メッセージの種類。
- メッセージの種類ごとに処理を振り分ける土台。

生成したLispはコミットする。
CIで、生成し直しても変わらないことを確かめる。
ネイティブの`generate.ps1`と同じ考え方である。
Lispの`defface`がすべての`EmacsLook`にそろっているかは、生成した一覧を使うテストで確かめる。
知識の元は、Rustの型の定義の1か所になる。
ネイティブ、TypeScript、Emacs Lispの型は、すべて同じfacetのリフレクションから作られる。

### Emacs版のパネル

Rustのプログラムは、`Changes`の`panels`で変わったパネルがあるときだけ、そのパネルの`tell_*`を動かす。
パネルを加工した値（ページのパスなど）の比較は、必要な分だけ残す。

### Web版

Web版の枠組みと受け取り方は、このADRでは決めない。
#931で、`Changes<B>`を前提に選ぶ。
今のLeptosの`RowStore`は変えない。
Leptosをやめるなら、その作業はむだになるためである。

### 置き換えるADR

ADR 019（Lispの`cl-mismatch`で行の差分を作る）は、このADRで置き換える。

## Alternatives

### Alternative 1: 差分を`client-native-bridge`に置いたままにする

移す作業がいらない。
しかし、EmacsとWebのシェルは差分を使えない。
同じ比較を、シェルごとに書くことになる。
そのため、選ばなかった。

### Alternative 2: 手で書いたJSON Schemaを元にする

JSON Schemaのファイルを1つ書き、`typify`でRustの型を、自前の生成器でLispを作る。
RustとLispは、どちらもスキーマに依存する。
しかし、ネイティブの型はfacetでRustの型から作っている。
知識の元が、スキーマとRustの型の2つになる。
そのため、選ばなかった。

### Alternative 3: `schemars`でJSON Schemaを作り、そこからLispを作る

`schemars`はserdeの属性を見るので、スキーマは`serde_json`の書くJSONと同じ形になる。
しかし、生成の道具が、facetと`schemars`の2本になる。
facetの`Registry`からも、同じようにLispを作れる。
そのため、選ばなかった。

### Alternative 4: `serde_json`のまま、名前をテストで合わせる

今のJSONの書き方を変えなくてよい。
しかし、名前の元が、serdeの属性とfacetの属性の2つになる。
合っているかを、テストで確かめ続けることになる。
そのため、選ばなかった。

### Alternative 5: Lispの読める形で送る

`read`が、見た目の付いた文字列を一度に作る。
1000行を送るときに、JSONの約半分の時間で済んだ。
しかし、差が出るのは、ページを開くときのように全部の行を送るときだけである。
RustでLispの書式を正しく書く処理も要る。
そのため、選ばなかった。

### Alternative 6: Emacsのダイナミックモジュールにする

RustをEmacsの中でDLLとして動かす。
パイプも書式もなくなる。
しかし、コアの通信やタイマーは別のスレッドで動くので、Emacsのメインスレッドに戻す仕組みが要る。
Rustが落ちると、Emacsも落ちる。
作りが大きく変わるので、選ばなかった。

### Alternative 7: `RichText`をそのままEmacsに送る

`EmacsText`の型がいらない。
しかし、Lispが`RichText`の木を平らにする処理を持つことになる。
Lispを一番薄くするため、選ばなかった。

## Confidence Level

確信度は中くらいである。
差分の置き場所と一般化は、ADR 076の作りを広げるだけなので、確信度は高い。
一方で、次のことはまだ確かめていない。

- `facet_generate`が、型の引数の付いた`Changes<B>`を扱えるか。
- facetのJSONが、`Inbound`と`Outbound`のenumを、今と同じ形で書けるか。
- `Registry`から、Lispの関数と定数を作れるだけの情報が取れるか。

次のときに見直す。

- facetのJSONや`Registry`で、Emacsのプロトコルを表せないと分かったとき。
- #931で、Web版の受け取り方が`Changes<B>`に合わないと分かったとき。

## Consequences

良い結果は次のとおりである。

- 差分を作る処理が、`client-core`の1か所になる。どのシェルも同じ差分を使える。
- Emacs版のプロトコルの知識が、Rustの型の1か所になる。名前を変えても、Lispは生成し直すだけでよい。
- ピュアTypeScriptのシェルを作るときも、`facet_generate`で同じ型から作れる。

悪い結果は次のとおりである。

- Lispの生成器を、新しく作って保つ必要がある。
- `outliner-emacs`のJSONの書き方が、`serde_json`からfacetのJSONに変わる。`protocol.rs`のテストも直す。
- facetのJSONと`Registry`は、確かめていないことが多い。

中立的な結果は次のとおりである。

- ネイティブのシェルが受け取るものは変わらない。
- ADR 019は置き換えられる。
- Web版の受け取り方は、#931で決める。
