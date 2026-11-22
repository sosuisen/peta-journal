# ADR 079: A TypeScript Web Shell That Applies List Diffs

## Status

Accepted

## Context

### 今のWeb版

Web版のシェル（`client/apps/web-app/outliner`）は、Leptos 0.8で書いたwasmのアプリである。
コアと同じwasmの中で動き、`Core`をRustの型のまま呼ぶ（ADR 024）。
大きさは、Rustで5,807行である。
ADR 052は、Web版をLeptosのまま残し、JSのフレームワークを併用しないと決めた。

ADR 076とADR 078で、画面の差分は`client-core`の`Changes<B>`になった。
`Changes<B>`は、行の変化の一覧（`ListChange`）と、変わったパネル（`Panel`）を持つ。
#858で、ネイティブの4つのシェル（WinUI、Compose、macOSとiOSのSwiftUI）は、`Changes`を当てるだけの形になった。
Emacs版も、ADR 078で同じ差分を受け取る形になった。
Web版だけが、今も`RowStore`（`components/outline.rs:40`）で全部の行を自分で比べている。
ADR 078は、Web版の受け取り方を#931で決めるとした。

### Leptosで困っていること

1つめは、テンプレートを型で守れないことである。
`view!`はマクロなので、要素や属性の名前の誤りがコンパイルで見つからないことがある。
エラーの出る位置も、誤りの場所からずれる。
rust-analyzerの補完と、rustfmtの整形も、マクロの中には効かない。

2つめは、ブラウザ上でのエラーをデバッグしにくいことである。
wasmのpanicのスタックは読みにくく、ソースマップも弱い。
ブラウザの開発ツールで、誤りの場所までたどりにくい。

ほかに、コアとの配線が長い。
`Memo`、`Effect`、クロージャの`clone`と`move`で、約800行ある。

### TypeScriptにしたときの量

#931で、Web版の5,807行を仕事の種類ごとに数えた（2026-11-23）。
行数は手で数えた目安である。
TypeScriptのシェルにすると、TSに残るのは約2,200〜2,600行と見積もった。

| 中身 | 今のRustの行数 | TSでの見込み |
|---|---|---|
| 描画（マークアップ） | 約1,350 | 約1,100 |
| ブラウザ固有（`components/`） | 約1,300 | 約1,000〜1,300 |
| ブラウザ固有（`dom.rs`など） | 約300 | 約250 |
| `Changes`を当てる | 約540 | 約90 |
| アトラスの配線 | 約260 | 約40 |
| wasmの初期化と`ui`の受け口 | — | 約120 |

ブラウザ固有で大きいのは、仮想リスト（`outline.rs:88-314`、約225行）、キャレットの計測（`dom.rs:107-314`、約210行）、ドラッグ（`item_drag.rs`と`edge_scroll.rs`）である。
これらは、どの枠組みでも同じ量が要る。
iOSとAndroidのシェルは、どちらも約3,100行である。
ただし、Webにある画面のうち約1,900行ぶんがない。
`ListChange`と`Panel`を当てる部分は、どちらも約70行だった。

### TSに書かなくてよいもの

`effects/api.rs`、`sse.rs`、`timer.rs`は、DOMに触れない。
APIの要求の順番とタイマーの方針は、もう`client_core::policy`の`InOrder`と`Timers`にある。
`window`のAPI（title、alert、confirm、clipboard、localStorage）と、`history.pushState`と`popstate`も、wasmから扱える。
そのため、fetch、SSE、タイマー、ページの移動は、wasmに残せる。

本文は、今は`AstBody`（`body.rs:12`）である。
これをTSで描くと、本文のASTをHTMLへ変える処理がTSに要る。
本文の形をHTMLの文字列にすれば、`flatmarkdown_ast2html`と画像の大きさの計算はwasmに残る。

### コアへ移せる計算

Web版には、DOMに関係のない計算が約500行ある。
主なものは次のとおりである。

- タグの一覧の操作（`page_properties.rs:160-234`）。
- 修飾キーから、新しいタブか新しいウィンドウかを決める処理。同じ判定が6か所にある。
- 遅れたキーの再生（`replay.rs:50-122`）。
- `[[`の文脈の判定（`wiki.rs`）。Emacs版にも同じ処理がある。
- タスクの時間の表示（`task_time_info`）と、索引のパスの組み立て。

これらをコアへ移すと、Web版の枠組みに関係なく、どのシェルでも同じ判断を使える。

### 入力欄の数

アウトラインの編集中の入力欄は1か所である。
しかし、サイト全体には、ページのプロパティ、タイトル、設定画面、検索欄がある。
一般のWebアプリと同じくらいの数である。
検索欄は、1文字打つごとに`Search`のパネルが届く。
パネルを丸ごと描き直すと、フォーカス、キャレット、IMEの変換中の文字が消える。

### 枠組みの比較

#931で、Solid 2.0、Svelte 5、Angular、React 19を比べた。
比べた観点は、`Changes`の当て方、入力欄の状態、読みやすさ、ランタイムの大きさ、保守のしやすさ、コードの検証のしやすさである。

| 観点 | Solid 2.0 | Svelte 5 | Angular | React 19 |
|---|---|---|---|---|
| 書き方 | TSX | `.svelte` | クラスとテンプレート | TSX |
| 入力欄の状態 | 消えない | 消えない | 消えない | キーが同じなら消えない |
| テンプレートの型チェック | `tsc` | `svelte-check` | `strictTemplates` | `tsc` |
| formatter | Prettier、Biome | Prettierにプラグインが要る | Prettier | Prettier、Biome |
| テンプレートのカバレッジとミューテーション | 対象になる | 正確に数えにくい | 対象にならない | 対象になる |
| ランタイムの大きさ | 一番小さい | 小さい | 一番大きい | 大きい |
| 版を上げるとき | 1から2で互換が壊れた | 4から5で書き方が大きく変わった | `ng update`が自動で直す | 互換を長く保つ |
| AIで書くとき | 1.xの書き方が混ざりやすい | 4の書き方が混ざりやすい | 古い書き方も動く | 学習データが一番多い |

どの枠組みも、行のDOMを保ったまま中身だけを直せる。
差が出たのは、検証のしやすさと保守のしやすさである。
SolidとReactは、テンプレートが普通のTSXなので、`tsc`、lint、Biome、Vitest、StrykerJSが追加の道具なしで全部に効く。
Solid 2.0は、2026-11-16に安定版になった。
周りのライブラリの2.0への対応は、まだ確かではない。

## Decision

Web版のシェルを、LeptosからTypeScriptとJSX（TSX）で書くシェルに替える。
枠組みは、SolidかReactのどちらかにする。
どちらにするかは、試作で比べて、別のADRで決める。

### ADR 052との関係

このADRは、ADR 052の1つめの決定（Web版をLeptosのまま残し、JSのフレームワークを併用しない）を置き換える。
ADR 052のほかの決定（キーの割り当てとエフェクトの方針をコアに置く）は、そのまま残す。
ADR 052には、この置き換えを追記する。

### シェルの形

Web版のシェルは、ネイティブのシェルと同じ形にする。
シェルは`Event`を送り、wasmから受け取った`Changes`を当てる。
行の`ListChange`は、行の一覧に順に当てる。
パネルは、変わったものだけを差し替える。

wasmとTSの境界には、新しいクレートを置く。
このクレートは、`client-native-bridge`の`NativeShell`と同じ形の関数を公開する。
`update`、`press`、`typed`、`drag`、`resolve`、`changes`である。
Web版だけに要る関数として、`pasteImage`と`beginCopy`も公開する。
`pasteImage`は、ファイルのパスの代わりに`File`を受け取る。
`beginCopy`は、ユーザーの操作のハンドラの中から同期で呼ぶ。Safariが、クリップボードへの書き込みをユーザーの操作の中でしか許さないためである。

`update`などの関数は、`Changes`を同期で返す。
今のWeb版には、`Event`を送った直後にview modelを読むところが3か所あるためである（`item_editor/text.rs:23-33`、`item_view.rs:79-85`、`item_editor/keys.rs:27`）。

### wasmに残すもの

次のものは、wasmに残す。
TSには書かない。

- APIの要求（`effects/api.rs`）、SSE（`effects/sse.rs`）、タイマー（`effects/timer.rs`）。
- `window`のAPI（title、alert、confirm、prompt、clipboard、localStorage）。
- ページの移動（`history.pushState`と`popstate`）。
- 確認のダイアログを1つずつ出す待ち行列。`client-native-bridge`の`Lanes`の振り分け表と質問の列を、スレッドに依存しない形で`client_core::policy`へ移し、Web版とネイティブで共有する。

TSに残るのは、DOMを使う処理だけである。
編集欄へのフォーカスと遅れたキーの再生（`FocusEditor`）、項目までのスクロール（`ScrollToItem`）、確認のダイアログの表示である。

### 本文の形

Web版の本文の形は、HTMLの文字列にする。
wasmが、`BodyModel::of`で`flatmarkdown_ast2html`を使ってHTMLを作る。
画像の`width`と`height`も、wasmがHTMLに入れる。
TSは、受け取ったHTMLを要素に入れるだけにする。

### 先にコアへ移す計算

Contextに挙げた約500行の計算は、枠組みを決める前にコアへ移す。
Leptosのままでもむだにならず、ネイティブとEmacsのシェルにも効くためである。

### 試作

アウトラインの画面を、SolidとReactの両方で試作する。
試作には、編集欄、検索欄、仮想リストを含める。
比べるのは次のことである。

- TSの行数。
- 1つのキーを打ってから画面が直るまでの時間。
- IMEで日本語を打つときの振る舞い。
- 1000項目のページを開くときに、`Reset`で全部の行を運ぶ時間。運び方は、JSONと`serde-wasm-bindgen`の両方で測る。
- 読みやすさ（開発者が判定する）
- 保守性。必要なセキュリティアップデートがどの程度出ているか。一部の機能しか使わないのに、巻き添えでアップデートの必要が多いとすれば手間。

### このADRで決めないこと

次のことは、試作のあとに決める。

- 枠組み（SolidかReactか）。
- `Changes`の運び方（JSONか`serde-wasm-bindgen`か）と、TSの型の作り方（`facet_generate`のTSか、ほかの方法か）。
- 仮想リストを、ライブラリ（`@tanstack/virtual`など）で作るか、1から書くか。ライブラリが、PetaJournalの要件を満たさない場合があるため、比べて決める。
- キャレットの計測に、ライブラリ（`textarea-caret`など）を使うか。
- ドラッグに、ライブラリを使うか。ドロップする場所と端でのスクロールは、もうコアが決めている（`drag(DragMove)`、`place_in_list`、`edge_step`）。

## Alternatives

### Alternative 1: Leptosのまま、`RowStore`で`Changes`を当てる

`RowStore`を、`ListChange`を当てる処理に置き換える。
言語とビルドは、Rustの1系統のままである。
しかし、テンプレートを型で守れないことと、ブラウザ上でデバッグしにくいことは残る。
そのため、選ばなかった。

### Alternative 2: Dominator（futures-signals）

`MutableVec`の変更に、`ListChange`をほぼそのまま当てられる見込みである。
しかし、Rustのマクロとwasmのデバッグの問題は、Leptosと同じである。
全部のコンポーネントを書き直すのに、得るものは`RowStore`の比べ直しが消えることだけである。
そのため、選ばなかった。

### Alternative 3: 枠組みを使わない（ピュアTS、JSXだけ）

`ListChange`を、DOMの`insertBefore`と`remove`に1対1で当てられる。
JSXだけなら、読みやすさも得られる。
しかし、パネルを描き直すと、入力欄の状態が消える。
入力欄が多いので、状態を消さない処理を欄ごとに書くことになる。
枠組みが持っている仕組みを、自分で作り直すことになる。
そのため、選ばなかった。

### Alternative 4: Svelte 5

テンプレートが一番短く、読みやすい。
しかし、`.svelte`という独自の形式のために、型チェックに`svelte-check`が要る。
テンプレートの分岐は、カバレッジで正確に数えにくい。
formatterにもプラグインが要る。
検証の道具が、TSXより一段ずつ遠い。
そのため、選ばなかった。

### Alternative 5: Angular

形が決まっていて、版を上げるときは`ng update`が自動で直す。
長期サポートの予定も公開されている。
しかし、ランタイムが一番大きい。
テンプレート（`.html`）は、カバレッジとミューテーションテストの対象にならない。
シェルの大半は描画なので、この穴は小さくない。
シェルを薄くする目的にも、一番合わない。
そのため、選ばなかった。

### Alternative 6: LitかPreact

Litの`repeat`とPreactの仮想DOMは、どちらも並びを自分で比べ直す。
SolidとReactより得るものがない。
そのため、選ばなかった。

## Confidence Level

TypeScriptとJSXのシェルに替える判断の確信度は、中くらいから高いである。
Leptosで困っている2つのことは、TSXなら道具の側で解ける。
シェルの形も、ネイティブのシェルですでに動いている。

一方で、次のことはまだ確かめていない。

- 行数の見積もりは、手で数えた目安である。
- `Reset`で全部の行を運ぶ時間。フィールドの多い構造体では、`serde-wasm-bindgen`より`JSON.parse`のほうが2〜3割速いという報告がある。
- `update`などの関数が`Changes`を同期で返す形で、すべての画面を書けるか。
- Solid 2.0で、周りのライブラリが動くか。

次のときは、この決定を見直す。

- 試作で、`Reset`を運ぶ時間がページを開く時間を目に見えて遅くすると分かったとき。
- 試作で、SolidとReactのどちらでも、IMEで日本語を正しく打てないと分かったとき。
- 試作で、TSの行数が見積もりより大きく、Leptosの今の行数と変わらないと分かったとき。

## Consequences

良い結果は次のとおりである。

- テンプレートを、`tsc`が型で確かめる。lint、formatter、カバレッジ、ミューテーションテストも、テンプレートまで効く。
- ブラウザの開発ツールで、シェルのエラーを追える。
- Web版が、ネイティブのシェルと同じ「`Event`を送り、`Changes`を当てる」形になる。コアの境界の振る舞いが1種類にそろう。
- 画面を直したとき、wasmを作り直さずに確かめられる。
- コアへ移す計算は、どのシェルにも効く。
- `RowStore`とLeptosの配線の約800行がなくなる。

悪い結果は次のとおりである。

- 言語が、RustとTypeScriptの2つになる。lint、formatter、CI、ビルドも2系統になる。
- wasmの境界のクレートを、新しく作って保つ必要がある。
- `Changes`と`Event`の型が境界を越える。型の生成と、そのずれを見つける仕組みが要る。
- Web版の全部のコンポーネントを書き直す。
- ブラウザでは、閉じる前に5秒待てない。`close`の代わりに、`beforeunload`か`visibilitychange`で打った文字を保存する。

中立的な結果は次のとおりである。

- アトラスは、これまでどおり`atlas.js`のPixiJSを使う。TSから直接呼べるので、wasm-bindgenの配線はなくなる。
- `highlight.js`は、そのまま使う。
- キャレットの計測、IMEの判定、クリップボードの処理は、TSに移る。量はほぼ変わらない。
- 枠組み、運び方、仮想リスト、キャレットの計測、ドラッグは、試作のあとの別のADRで決める。
