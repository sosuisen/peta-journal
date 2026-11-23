# ADR 080: The Border and Parts of the React Web Shell

## Status

Accepted

## Context

### ADR 079が残したこと

ADR 079は、Web版のシェルをReactで書くと決めた（#971）。
そのうえで、次のことを試作のあとに決めるとした。

- `Changes`の運び方（JSONか`serde-wasm-bindgen`か）と、TSの型の作り方。
- 仮想リストを、ライブラリで作るか、1から書くか。
- キャレットの計測に、ライブラリを使うか。
- ドラッグに、ライブラリを使うか。

このADRは、これらを決める。
あわせて、ADR 079の「wasmに残すもの」の節のうち、PR #973で違う形になった一文を置き換える。

### `Changes`の運び方

#971の試作で、1000行の`Reset`を運ぶ時間を測った。
JSONの文字列（`JSON.parse`を含む）と、`serde-wasm-bindgen`のJSの値は、どちらも約4〜6msだった。
運び方による差はなかった。

### 境界の名前と、serdeとfacet

`client-core`の型には、serdeとfacetの2つのderiveが付いている。
serdeは、値をJSONやbincodeに書く。
facetは、型の形をRustから読めるようにするリフレクションである。
`facet_generate`は、facetの形を読み、Swift、Kotlin、C#、TypeScriptの型を書く。

facetは、serdeの属性を読まない。
`client-core/src/model.rs`の`Item`には、`#[serde(rename_all = "camelCase")]`と`#[serde(rename = "type")]`が付いている。
これは、journal-serverの答えの形（`journal-core/src/dto.rs`の`ItemDto`）に合わせるためである。
そのため、同じ`Item`でも、serdeの名前は`pageId`と`type`、facetの名前は`page_id`と`item_type`になる。
ネイティブの境界はbincodeで、フィールドの名前を書かない。
そのため、このずれは今まで表に出なかった。

2026-11-23に、`facet_generate` 0.22.3の`typescript::Installer`と`JsonPlugin`で、境界の型をTSにして試した。
生成したTSは、`tsc`（TypeScript 7.0.2）の`strict`で通った。
enumは、`kind`で分ける共用体型になった。
Rustのdocコメントも、TSに付いた。

`serde_json`で書いた`Changes`は、生成したTSで読めなかった。
`Item.fromJson`が、`page_id`を探して失敗した。

`facet_json` 0.46.1で書いた`Changes`は、生成したTSで読めた。
1000行の`Reset`と14のパネルで試した。
どの項目にも、文字列と配列の`properties`を付けた。
`PropertyValue`は、`{"Text":"done"}`と`{"List":["a","b"]}`の形になった。
TSの`toJson`で書き戻したJSONは、値として元と同じだった。
Nodeで読む時間は、`JSON.parse`と`fromJson`を合わせて約7.4msだった。
`JSON.parse`だけなら約4.5msである。
同じ`Changes`をbincodeにして、`BincodePlugin`で生成したTSで読むと、約43msだった。

ADR 078は、Emacs版の境界を「`serde_json`ではなく、facetのJSONで書く」と決めた。
名前の元を、facetの属性の1つにするためである。
Emacs版は、この形で動いている（`client/apps/emacs/outliner/src/protocol.rs`）。

serdeの付け替えは、RESTの答えの型を手で写したものである。
この問題は、境界とは別に#805で扱う。
調べた結果は、#805と#875のコメントに書いた（2026-11-23）。

### 仮想リスト

#971の試作で、`@tanstack/react-virtual` 3.14.28を使った。
1000項目のページで、高さの違う行を描けた。
DOMにある行は、約41行だった。

### キャレットの計測

今のWeb版の`client/apps/web-app/outliner/src/dom.rs`は、キャレットについて次のことをしている。

- キャレットの位置を測る（`measure_marker`、約55行）。テキストエリアの写しを作り、位置の次の文字の場所を読む。
- キャレットが最初の行か最後の行かを決める（`is_on_first_line`、`is_on_last_line`）。上の計測を使う。
- マウスの下の文字の位置を求める（`textarea_offset_at_point`）。FirefoxとChrome 128以降は`caretPositionFromPoint`がテキストエリアの中の位置を返す。ほかのブラウザでは、テキストエリアの上に写しを重ねて求める。
- 表示中の本文で、マウスの下の文字までの文字を求める（`text_before_point`）。

`textarea-caret` 3.1.0（2022年に公開）が代われるのは、1つめの計測だけである。
型は、別のパッケージ（`@types/textarea-caret`）にある。

2026-11-23に、ヘッドレスのChrome 157で、折り返しのある4つの文を1文字ずつ比べた。
英語、日本語、URLの混ざった文、改行のある文である。
キャレットの行は、`textarea-caret`も`dom.rs`のやり方も、すべての位置で正しかった。
正しい行は、`caretPositionFromPoint`で各行の頭の位置を読んで決めた。
FirefoxとSafariでは試していない。

### ドラッグ

今のWeb版のドラッグは、次の2種類である。

- 本文から始めて、本文を出ると項目の選択に変わるドラッグ（`components/item_drag.rs`）と、端でのスクロール（`components/edge_scroll.rs`）。
  どちらも、判断はもうコアにある（`client_core::drag::ItemDrag`、`edge_step`）。
- HTMLのドラッグ＆ドロップ。画像のファイルのドロップ、タグの並べ替え、アトラスのカードである。

`@dnd-kit/react` 0.5.3は、ポインターのイベントで動く。
パッケージのコードに`dataTransfer`がないので、OSからのファイルのドロップは扱えないと見ている。
`@atlaskit/pragmatic-drag-and-drop` 4.0.0は、HTMLのドラッグ＆ドロップを包む。
ファイルのための`external`がある。
どちらも、パッケージの中身を見ただけで、動かしてはいない。

### 効果の振り分け

ADR 079の「wasmに残すもの」の節には、次の一文がある。

> 確認のダイアログを1つずつ出す待ち行列。`client-native-bridge`の`Lanes`の振り分け表と質問の列を、スレッドに依存しない形で`client_core::policy`へ移し、Web版とネイティブで共有する。

振り分け表とは、コアの効果（`Effect`）をだれがこなすかを決める`match`である（`client-native-bridge/src/shell.rs`の`NativeShell::new`）。
この表の中身は、ネイティブとWebで違う。

| 効果 | ネイティブ | Web（今のLeptos版） |
|---|---|---|
| `UiOp::Confirm`、`UiOp::Prompt` | 列に並べてシェルへ渡す | wasmが`window.confirm`、`window.prompt`で答える |
| `UiOp::ConfirmDestructive` | 列に並べてシェルへ渡す | 自前のダイアログで答える |
| `UiOp::SetTitle`、クリップボード | シェルへ渡す | wasmが`window`のAPIでこなす |

そのため、PR #973では、質問かどうかの判定（`client_core::policy::is_question`）と、質問の列（`client_core::policy::Questions`）だけをコアへ移した。
振り分けの`match`は、`client-native-bridge`に残した。

## Decision

### `Changes`の運び方

wasmとTSの境界は、JSONの文字列で運ぶ。
wasmは、`Changes`を`facet_json`で書く。
TSから来る`Event`、`KeyPress`、`Typing`、`DragMove`と、効果への答えも、wasmが`facet_json`で読む。
ADR 078のEmacs版と同じ形である。

境界では、serdeの名前を使わない。
境界の名前は、Rustのフィールド名とfacetの属性だけで決まる。
serdeの名前の付け替えは、RESTのためだけに残る。

### TSの型の作り方

TSの型は、`facet_generate`の`typescript::Installer`と`JsonPlugin`で生成する。
生成のもとにする型は、ネイティブの`client-native-bridge/src/codegen.rs`の`border_types`と同じにする。
ネイティブ、Emacs Lisp、TypeScriptの型は、すべて同じfacetのリフレクションから作られる。

TSは、`JSON.parse`のあとに、生成した`fromJson`で値を読む。
`fromJson`は、JSONの形が型と違うときに、その場で失敗する。

### 仮想リスト

仮想リストは、`@tanstack/react-virtual`で作る。

### キャレットの計測

キャレットの計測は、今の`dom.rs`の処理をTSに移す。
`textarea-caret`は使わない。

### ドラッグ

ドラッグは、ライブラリを使わずに書く。
本文から項目の選択に移るドラッグと、端でのスクロールは、ポインターのイベントをコアへ渡すだけにする。
画像のファイル、タグ、アトラスのカードは、ブラウザのHTMLのドラッグ＆ドロップで扱う。

### 効果の振り分け

ADR 079の「wasmに残すもの」の節の、確認のダイアログの待ち行列の一文を、次のように置き換える。

コアで共有するのは、質問かどうかの判定（`client_core::policy::is_question`）と、質問の列（`client_core::policy::Questions`）である。
効果の振り分けは、ネイティブのブリッジとWebの境界のクレートが、それぞれ持つ。
どの効果をだれがこなすかが、プラットフォームで違うためである。

## Alternatives

### Alternative 1: `serde-wasm-bindgen`のJSの値で運ぶ

試作では、JSONと同じ速さだった。
しかし、`serde-wasm-bindgen`はserdeで書くので、境界にserdeの名前（`pageId`）が出る。
facetから作るTSの型と、名前が合わない。
PR #969で、使っていない依存として外したばかりでもある。
そのため、選ばなかった。

### Alternative 2: `serde_json`で書き、facetにも同じ名前の付け替えを書く

`#[facet(rename_all = "camelCase")]`と`#[facet(rename = "type")]`を足すと、生成したTSはserdeと同じ名前で読む。
小さな型で確かめた。
しかし、付け替えを、serdeとfacetの2か所に書くことになる。
片方だけ直すと、境界が壊れる。
ADR 078が、名前の元をfacetの属性の1つにすると決めたことにも反する。
そのため、選ばなかった。

### Alternative 3: ネイティブと同じbincodeで運ぶ

ネイティブと同じ形式になる。
しかし、生成したTSで1000行を読むと約43msで、JSONの約7.4msより遅かった。
バイト列は、ブラウザの開発ツールで中身を読めない。
そのため、選ばなかった。

### Alternative 4: TSの型を手で書く

#971の試作は、手で書いた型を使った。
しかし、Rustの型が変わっても、TSの型は変わらない。
ずれは、実行するまで分からない。
そのため、選ばなかった。

### Alternative 5: 仮想リストを1から書く

今のLeptos版は、仮想リストを約225行で書いている。
しかし、`@tanstack/react-virtual`は、試作で高さの違う行を描けた。
1から書いて得るものは、見つかっていない。
そのため、選ばなかった。

### Alternative 6: `textarea-caret`でキャレットを測る

Chrome 157では、`dom.rs`のやり方と同じく、すべての位置で正しかった。
しかし、代われるのは`dom.rs`の約55行だけで、ほかの計測は残る。
2022年から新しい版が出ていない。
そのため、選ばなかった。

### Alternative 7: ドラッグのライブラリ（`@dnd-kit/react`、`@atlaskit/pragmatic-drag-and-drop`）

落とす場所と端でのスクロールは、もうコアが決めている。
ライブラリを使うと、コアが決めることを、ライブラリがもう一度決めることになる。
`@dnd-kit/react`は、OSからのファイルのドロップを扱えないと見ている。
版も0.xである。
そのため、選ばなかった。

### Alternative 8: 振り分け表もコアで共有する

プラットフォームごとの違いを引数で渡せば、1つの表にできる。
しかし、違いを渡す仕組みの分だけ、表が読みにくくなる。
共有して得るのは、質問の扱いをそろえることで、これは質問の列の共有で足りる。
そのため、選ばなかった。

## Confidence Level

運び方と型の作り方の確信度は、中くらいから高いである。
Emacs版で、同じfacetのJSONの境界がすでに動いている。
1000行の`Changes`を、生成したTSで読めることも確かめた。

まだ確かめていないことは、次のとおりである。

- TSが書いた`Event`を、`facet_json::from_str`で読めるか。
- wasmにしたときの、`facet_json`の大きさと速さ。測ったのはNodeで、TSの側だけである。
- `facet_json` 0.46には、`flatten`の下のenumの名前を付け替えないという制限がある（`client/apps/emacs/outliner/src/protocol.rs`のコメント）。Web版の境界の型が、これに当たるか。
- キャレットの計測を、FirefoxとSafariで試していない。
- ドラッグのライブラリは、パッケージの中身を見ただけで、動かしていない。

次のときは、この決定を見直す。

- `facet_json`で、境界の型の一部を書けない、または読めないと分かったとき。
- wasmで`Changes`を書く時間が、ページを開く時間を目に見えて遅くすると分かったとき。
- `facet_json`が1.0になるなどで、0.46の版から大きく変わったとき。今の最新の安定版は0.46.1で、0.50はrc版である。

## Consequences

良い結果は次のとおりである。

- 境界の名前の元が、Rustの型とfacetの属性の1か所になる。
- ネイティブ、Emacs、Webの境界の型が、同じfacetのリフレクションから作られる。
- Rustの型が変わると、TSのシェルがコンパイルで止まる。
- 境界のJSONは、ブラウザの開発ツールで読める。
- キャレットの計測とドラッグに、新しい依存が増えない。

悪い結果は次のとおりである。

- 境界のクレートに、`facet-json`の依存が増える。
- `client-core`のfacetのderiveが、Web版のふだんのビルドでも要る。今は、`facet_typegen`の機能を付けたビルドだけで付く。
- 境界のJSONとRESTのJSONで、同じ`Item`の名前が違う（`page_id`と`pageId`）。
- キャレットの計測とドラッグのTSは、自分で保つ。

中立的な結果は次のとおりである。

- RESTの答えの型を手で写している問題は残る。#805で扱う。
- serdeとfacetのderiveは、両方残る。どちらかに寄せるかは、#875で扱う。
