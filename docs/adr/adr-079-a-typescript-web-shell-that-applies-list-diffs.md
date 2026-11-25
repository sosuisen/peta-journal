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

1つめは、テンプレートの検証の道具が弱いことである。
Leptos 0.8.23で試すと、要素、属性、イベント、propsの名前と型の誤りは、コンパイルで見つかった（2026-11-23）。
見つからないのは、`prop:`の名前と、`style:`のCSSの名前である。
今のWeb版は、`prop:`を17か所、`style:`を11か所で使っている。
値とハンドラーの型の誤りでは、`#[component]`の行を指すエラーも1つ増える。
rustfmtは`view!`の中を整形しないので、`leptosfmt`が要る。
`view!`はマクロなので、中に書いた条件とハンドラーは、ミューテーションテストの対象にならない。
rust-analyzerの補完がマクロの中でどこまで効くかは、試していない。

この節を最初に書いたときは、「`view!`は要素や属性の名前の誤りを見つけられない」としていた。
これは確かめずに書いたことで、事実と違った。

2つめは、ブラウザ上でのエラーをデバッグしにくいと予想されることである。
wasmのpanicのスタックは読みにくく、ソースマップも弱いと見ている。
そのため、ブラウザの開発ツールで、誤りの場所までたどりにくいと予想している。
これは確かめていない。

ほかに、コアとの配線が長い。
`Memo`、`Effect`、クロージャの`clone`と`move`で、約800行ある。

### シェルの枠組みを選ぶ基準

このプロジェクトのシェルは、それぞれのプラットフォームの標準の枠組みで書く。
ネイティブのシェルは、WindowsではWinUI、AndroidではCompose、macOSとiOSではSwiftUIである。
Web版のシェルも、同じ考え方で、Webの標準の枠組みから選ぶ。
この基準は、開発者が決めた。

この基準では、まずRustの枠組み（Leptos、Dioxus、Sycamore、Yew、Dominator）が外れる。
どれも、Webの標準の枠組みではないためである。
Webのフロントエンドで、WinUIやSwiftUIに当たる水準の標準は、Reactであると開発者は見ている。

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

Web版の枠組みの比較は、この表を正とする。
#931に書いた比較の表は、この表に置き換えた。

表に入れたのは、Leptos 0.8.23、Dioxus 0.7.10、Sycamore 0.9.4、Solid 2.0.3、React 19.3.1、Angular 22.3.0、Svelte 5.59.1である。
ほかの枠組みを外した理由は、Alternativesに書く。
表を作ったのは2026-11-23である。

| 観点 | Leptos 0.8 | Dioxus 0.7 | Sycamore 0.9 | Solid 2.0 | React 19 | Angular 22 | Svelte 5 |
|---|---|---|---|---|---|---|---|
| **描画** | | | | | |  |  |
| 書き方 | `view!`（HTMLに似たマクロ） | `rsx!`（独自のマクロ） | `view!`（関数呼び出しに似たマクロ） | TSX | TSX | `.html`のテンプレート（独自の構文） | `.svelte`（独自の形式） |
| 更新の仕方 | signalで細かく更新する | 仮想DOMとsignal | signalで細かく更新する | signalで細かく更新する | 仮想DOM | signalで細かく更新する。zone.jsなしが既定 | signal（runes）で細かく更新する |
| 入力欄の状態 | DOMを作り直さない | キーが同じならDOMを作り直さない | DOMを作り直さない | DOMを作り直さない | キーが同じならDOMを作り直さない | `track`が同じならDOMを作り直さない | キーつきの`{#each}`なら、DOMを作り直さない |
| `ListChange`の当て方 | signalの`Vec`に当て、`<For>`で描く | `Store<Vec<T>>`に当てる | signalの`Vec`に当て、`Keyed`で描く | storeの配列に`splice`で当てる | 新しい配列を`setState`に渡す | signalの配列に当て、`@for`の`track`で描く | `$state`の配列に`splice`で当て、キーつきの`{#each}`で描く |
| **検証（テンプレートの型チェック）** | | | | | |  |  |
| 要素、属性、イベントの名前 | 見つかる | 見つかる | 見つかる | 見つかる | 見つかる | 要素と、`[ ]`でつなぐ名前は見つかる。静的な属性とイベントの名前は通る | 属性とイベントの名前は見つかる。知らない要素（`<dvi>`）は通る |
| 属性の値の型 | 見つかる | 表示できる値なら通る | 見つかる | 一部通る（`class={42}`） | 見つかる | DOMの属性は通る（`[disabled]="'yes'"`） | 見つかる |
| `aria-`の名前 | 見つかる | 見つかる | 通る | 通る | 通る | 通る | 見つかる |
| `data-`の名前 | 通る | 通る | 通る | 通る | 通る | 通る | 通る |
| `prop:`の名前 | 通る | （`prop:`はない） | 通る | 見つかる。使う`prop:`は型を宣言する | （`prop:`はない） | （`prop:`はない。`[value]`の名前は見つかる） | （`prop:`はない。`bind:`の名前は見つかる） |
| CSSのプロパティの名前 | `style:`は通る | 見つかる（属性として書く） | `style`は文字列だけ | 見つかる | 見つかる | 名前も値も通る（`[style.colr]`） | `style:`は名前も値も通る |
| propsの型と、必須のpropsの抜け | 見つかる | 見つかる | 見つかる | 見つかる | 見つかる | 見つかる | 見つかる |
| エラーの位置 | 誤りの場所。値とハンドラーの誤りでは、`#[component]`を指すエラーも1つ増える | 誤りの場所 | 値とハンドラーの誤りは、`view!`の頭を指す | 誤りの場所 | 誤りの場所 | 誤りの場所（`.html`の行と列） | 誤りの場所 |
| **検証（ほかの道具）** | | | | | |  |  |
| formatter | `leptosfmt` | `dx fmt` | 見つからなかった | Prettier、Biome | Prettier、Biome | Prettier | Prettierのプラグイン |
| テンプレートのミューテーション | マクロの中は対象にならない | マクロの中は対象にならない | マクロの中は対象にならない | `{}`の中の式が対象になる | `{}`の中の式が対象になる | 対象にならない | `{}`の中の式と、`{#if}`の条件が対象になる |
| **開発** | | | | | |  |  |
| ブラウザでのデバッグ | panicするとwasmが止まる。Rustのソースで追うには、ChromeのDWARFの拡張が要る | Leptosと同じ。`dx`がデバッガつきでChromeを開く | Leptosと同じ | ソースマップで、開発ツールから追える | ソースマップで、開発ツールから追える | ソースマップで、開発ツールから追える | ソースマップで、開発ツールから追える |
| 直しを見る速さ | 今のWeb版で、1ファイルを変えたあとの`cargo build`が約1.9秒 | Rustのコードをホットパッチで差し込む | wasmを作り直す | ViteのHMR | ViteのHMR | `ng serve`のHMR | ViteのHMR |
| AIで書くとき（見立て） | 0.6以前の書き方が混ざりやすい | 公式の`llms.txt`がある | 学習データが少ない | 1.xの書き方が混ざりやすい | 学習データが一番多い | NgModuleや`*ngIf`の古い書き方が混ざりやすい | Svelte 4の書き方（`export let`、`$:`）が混ざりやすい |
| **性能** | | | | | |  |  |
| `Changes`を運ぶ手間 | ない | ない | ない | JSの値に変える。試作で測る | JSの値に変える。試作で測る | JSの値に変える。試作で測る | JSの値に変える。試作で測る |
| 起動の速さと、配る大きさ | 試作で測る | 試作で測る | 試作で測る | 試作で測る | 試作で測る | 試作で測る | 試作で測る |
| キーを打ってから画面が直るまで | 試作で測る | 試作で測る | 試作で測る | 試作で測る | 試作で測る | 試作で測る | 試作で測る |
| **保守** | | | | | |  |  |
| 版を上げるとき | 0.7で中身をほぼ作り直した（2024-11） | 0.5でsignalに替え、0.7で`Store`とホットパッチを入れた | 0.9でsignalを作り直した（2024-11） | 1から2で互換が壊れた | 19で古いAPIを消した。その前に18.3で警告を出した | 12か月ごとにメジャー版を出す。`ng update`が自動で直す | 5で書き方をrunesに替えた（2024-10） |
| 公開されたセキュリティの勧告 | なし | なし | なし | 1件（2025-02、JSXのフラグメントのXSS） | `react`に2件（2020）、`react-dom`に1件（2019） | `core`、`common`、`compiler`に19件。うち18件は2025-11以降で、XSSとHTTPクライアントのもの | 14件。うち9件はSSRのもので、シェルは使わない |
| 実行時の依存の数 | 172 | 123 | 32 | 5 | 3 | 5 | 19 |
| 直近30日のダウンロード | 約101万 | 約57万 | 約8千 | 約4,900万 | 約11億 | 約3,500万 | 約4,100万 |
| 仮想リスト | 自分で書く。`leptos_virtual_scroller`は90日で59件 | 自分で書く。`dioxus-nox-virtualize`は90日で138件 | 見つからなかった | `@tanstack/solid-virtual`（30日で約550万） | `@tanstack/react-virtual`（30日で約1.8億） | CDKは行の高さが同じときだけ。高さが変わる行は`@tanstack/angular-virtual`（30日で約16万） | `@tanstack/svelte-virtual`（30日で約78万） |
| 言語とビルド | Rustの1系統 | Rustの1系統 | Rustの1系統 | RustとTSの2系統 | RustとTSの2系統 | RustとTSの2系統 | RustとTSの2系統 |
| 落ちたときの範囲 | シェルがpanicすると、wasm全体が止まる | 同じ | 同じ | シェルの例外では、wasmは止まらない | シェルの例外では、wasmは止まらない | シェルの例外では、wasmは止まらない | シェルの例外では、wasmは止まらない |

どの枠組みでも、シェルはネイティブのシェルと同じ「`Event`を送り、`Changes`を当てる」形にできる。
`ListChange`は、行の一覧に順に当てる。
キーで比べ直すのは、枠組みの中の処理である。
SwiftUIの`ForEach`やComposeの`LazyColumn`も、同じようにキーで比べている。
そのため、シェルのコードに`RowStore`のような比べ直しは残らない。

### 仮想DOMとsignalの差

このWeb版では、「更新の仕方」の行の差は小さいと見ている。
これは見立てで、測ってはいない。

コアが`ListChange`を出しても、一覧のキーの比べ直しは、どちらの仕方にも残る。
Solidの`<For>`とSvelteのキーつきの`{#each}`も、配列が変わると、一覧のすべてのキーを比べるためである。
そのため、signalの側だけがコアの差分をそのまま使える、ということはない。

差が出るのは、1つの行の中身を直すときである。
signalでは、その行のテキストのノードだけを書き換える。
仮想DOMでは、一覧のコンポーネントを描き直し、変わった行を描き直す。
変わらない行は、`memo`で飛ばせる。
ただし、変わらない行のオブジェクトを使い回すことが前提になる。

この差は、次の2つの理由で小さくなる。
1つめに、仮想リストを使うので、DOMにある行は画面に見えている数十行だけである。
2つめに、キーを1回打って変わる行は、ふつう1行だけである。

差が出そうなのは、大きなページを開いたときの`Reset`と、多くの行をまとめて動かす操作である。
この2つは、「試作」の節で「キーを打ってから画面が直るまで」と一緒に測る。

### 表の確かめ方

テンプレートの型チェックの行は、すべて試した。
枠組みごとに小さなプロジェクトを作り、テンプレートに誤りを1つずつ入れた。
Rustの3つは`cargo check --target wasm32-unknown-unknown`、SolidとReactは`tsc`（TypeScript 7.0.2）で確かめた。
Solid 2.0では、DOMのJSXの型が`@solidjs/web`に移ったので、`jsxImportSource`を`@solidjs/web`にした。
TSXでは、名前に`-`を含む属性を、TypeScriptが名前で調べない。
そのため、`aria-`と`data-`の名前の誤りは、SolidとReactでは通る。
Angularは、`ng new`で作ったプロジェクトに`strictTemplates`を入れ、`ngc`で確かめた。
`strictUnclaimedEventNames`も入れたが、`(clik)`は通った。
AngularのコンパイラーはDOMの属性の値の型を調べない（`checkTypeOfDomBindings`が常に`false`）。
`clas="a"`のような静的な属性は、ただのHTMLの属性として扱われ、名前を調べない。
Svelteは、`svelte-check` 4.7.8とTypeScript 6で確かめた。
TypeScript 7.0には、安定したJSのAPIがない（`exports`は`unstable/`だけ）。
そのため、TypeScriptのAPIを使う道具には、TypeScript 6が要る。
`svelte-check`のpeerDependenciesは`^5.0.0 || ^6.0.0`で、Angularの`@angular/compiler-cli`は`>=6.0 <6.1`である。
`svelte-check`には、TypeScript 7で調べる`--tsgo`があるが、試していない。
TSXは`tsc`だけで調べられるので、TypeScript 7をそのまま使えた。

ほかの行の確かめ方は、次のとおりである。

- テンプレートのミューテーション: cargo-mutantsの説明書の制限事項の節と、StrykerJSの`string-literal-mutator.ts`のソースで確かめた。StrykerJSは、JSXの要素を消したり入れ替えたりしない。属性の文字列と、要素の間の文字も変えない。
  Angularのテンプレートは、StrykerJSが対象にしていない。StrykerJSの開発者が、仕組みの上で難しいと答えている（stryker-js #5156）。
  Svelteは、StrykerJS 10.1.1とVitestで回した。`{name.toUpperCase()}`の式と、`{#if show === true}`の条件が変えられた。
  カバレッジは、AngularとSvelteだけ試した。`@if`と`@else`（Svelteでは`{#if}`と`{:else}`）のテンプレートでカバレッジを取ると、通っていない`else`の行が出た。
  ほかの枠組みのカバレッジは試していない。
- 直しを見る速さ: Leptosは、今のWeb版で`outline.rs`を変えて3回測った。wasm-bindgenとTrunkの手順は含まない。Dioxusのホットパッチは、0.7のリリースノートで読んだ。Web（wasm）でも動くとある。試してはいない。
- ブラウザでのデバッグ: ドキュメントで読んだ。試していない。
- 公開されたセキュリティの勧告: GitHub Advisory Databaseで、各パッケージを調べた。Rustの勧告は、RustSecから取り込まれている。
- 実行時の依存の数: Rustは、小さなプロジェクトの`cargo tree -e normal --target wasm32-unknown-unknown`で数えた。`wasm-bindgen`と`web-sys`も含む。コアもこの2つを使うので、増える数はこれより少ない。TSは`npm ls`で数えた。Viteなどのビルドの道具は含まない。
- ダウンロード: crates.ioとnpmの数である。数え方が違うので、同じ側どうしで比べる。
- 仮想リスト: crates.ioで`virtual-list`、`virtual-scroll`、`virtualized`を検索した。90日の数は、crates.ioの「最近のダウンロード」である。
- 版を上げるとき: LeptosとSycamoreは、リリースノートで確かめた。Dioxusは、0.7のリリースノートで確かめた。Reactは、React 19の移行ガイドで確かめた。Solidは、#931で調べた結果である。Angularは、angular.devの「Angular versioning and releases」で確かめた。Svelteは、npmの5.0.0の公開日で確かめた。
- AIで書くとき: 見立てである。確かめていない。
- 性能の3行は、「試作」の節で測る。

## Decision

Web版のシェルを、LeptosからTypeScriptとJSX（TSX）で書くシェルに替える。
枠組みは、Contextの基準のとおり、Webの標準の枠組みから選ぶ。
枠組みは、Reactにする。
SolidとReactを#971で試作して比べ、開発者がReactに決めた（2026-11-23）。
判断の結果と理由は、#971のコメント（https://github.com/sosuisen/peta-journal/issues/971#issuecomment-7094381181）にある。

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

アウトラインの画面を、SolidとReactの両方で試作する（#971）。
試作には、編集欄、検索欄、仮想リストを含める。
比べるのは次のことである。

- TSの行数。
- 1つのキーを打ってから画面が直るまでの時間。
- IMEで日本語を打つときの振る舞い。
- 1000項目のページを開くときに、`Reset`で全部の行を運ぶ時間。運び方は、JSONと`serde-wasm-bindgen`の両方で測る。画面に出るまでの時間も測る。
- 多くの行をまとめて動かしたときに、画面が直るまでの時間。
- 読みやすさ（開発者が判定する）
- 保守性。必要なセキュリティアップデートがどの程度出ているか。一部の機能しか使わないのに、巻き添えでアップデートの必要が多いとすれば手間。

### このADRで決めないこと

次のことは、試作のあとに決める。

- `Changes`の運び方（JSONか`serde-wasm-bindgen`か）と、TSの型の作り方（`facet_generate`のTSか、ほかの方法か）。
- 仮想リストを、ライブラリ（`@tanstack/virtual`など）で作るか、1から書くか。ライブラリが、PetaJournalの要件を満たさない場合があるため、比べて決める。
- キャレットの計測に、ライブラリ（`textarea-caret`など）を使うか。
- ドラッグに、ライブラリを使うか。ドロップする場所と端でのスクロールは、もうコアが決めている（`drag(DragMove)`、`place_in_list`、`edge_step`）。

## Alternatives

### Alternative 1: Leptosのまま、`RowStore`で`Changes`を当てる

`RowStore`を、`ListChange`を当てる処理に置き換える。
言語とビルドは、Rustの1系統のままである。
しかし、`prop:`と`style:`の名前を確かめられないことと、ブラウザ上でデバッグしにくいことは残る。
そのため、選ばなかった。

### Alternative 2: Dominator（futures-signals）

`MutableVec`の変更に、`ListChange`をほぼそのまま当てられる見込みである。
しかし、Rustのマクロとwasmのデバッグの問題は、Leptosと同じである。
`RowStore`の比べ直しは、Leptosのままでも消せる。
全部のコンポーネントを書き直しても、得るものがない。
2024-08から、新しい版も出ていない。
そのため、選ばなかった。

### Alternative 3: 枠組みを使わない（ピュアTS、JSXだけ）

`ListChange`を、DOMの`insertBefore`と`remove`に1対1で当てられる。
JSXだけなら、読みやすさも得られる。
しかし、パネルを描き直すと、入力欄の状態が消える。
入力欄が多いので、状態を消さない処理を欄ごとに書くことになる。
枠組みが持っている仕組みを、自分で作り直すことになる。
そのため、選ばなかった。

### Alternative 4: Svelte 5

表のとおり、検証はSolidとほぼ同じである。
テンプレートの式はミューテーションテストの対象になり、`{#if}`の分岐もカバレッジに出る。
`aria-`の名前は、SolidとReactと違って見つかる。
しかし、知らない要素と、`style:`の名前と値は通る。
`.svelte`という独自の形式のために、型チェックに`svelte-check`が、formatterにプラグインが要る。
TypeScriptの版を上げるときは、`svelte-check`の対応を待つことがある。
今は、TypeScript 7の`--tsgo`は試験的である。
`svelte-check`の開発者は、TypeScript 6と7を並べて入れる形を勧めている（language-tools #3063）。
そのため、選ばなかった。

### Alternative 5: Angular

形が決まっていて、版を上げるときは`ng update`が自動で直す。
長期サポートの予定も公開されている。
テンプレート（`.html`）のカバレッジも取れる。
しかし、表のとおり、テンプレートの型チェックが一番弱い。
静的な属性、イベント、CSSの名前と、DOMの属性の値の型を調べない。
テンプレートは、ミューテーションテストの対象にもならない。
シェルの大半は描画なので、この穴は小さくない。
仮想リストのCDKは、高さが変わる行に使えない。
そのため、選ばなかった。

### Alternative 6: LitかPreact

Litの`repeat`とPreactの仮想DOMは、どちらも枠組みの中でキーを比べる。
これは表のほかの枠組みと同じで、差にならない。
SolidとReactより得るものは、見つけていない。
ただし、型チェックなどは試していない。
そのため、表に入れなかった。

### Alternative 7: Yew

Yew 0.23で試すと、知らない要素（`<dvi>`）と知らない属性（`clas`）が、エラーにならずに通った（2026-11-23）。
テンプレートの検証は、Leptosより弱い。
そのため、表に入れなかった。

## Confidence Level

TypeScriptとJSXのシェルに替える判断の確信度は、中くらいから高いである。
Leptosで困っていることは、TSXなら道具の側で解ける。
ただし、テンプレートの型チェックの差は、枠組みの比較の表のとおり小さい。
シェルの形も、ネイティブのシェルですでに動いている。

一方で、次のことはまだ確かめていない。

- 行数の見積もりは、手で数えた目安である。
- `Reset`で全部の行を運ぶ時間。フィールドの多い構造体では、`serde-wasm-bindgen`より`JSON.parse`のほうが2〜3割速いという報告がある。
- `update`などの関数が`Changes`を同期で返す形で、すべての画面を書けるか。

次のときは、この決定を見直す。

- 試作で、`Reset`を運ぶ時間がページを開く時間を目に見えて遅くすると分かったとき。
- 試作で、ReactではIMEで日本語を正しく打てないと分かったとき。
- 試作で、TSの行数が見積もりより大きく、Leptosの今の行数と変わらないと分かったとき。

## Consequences

良い結果は次のとおりである。

- テンプレートを、`tsc`が型で確かめる。lint、formatter、カバレッジも、テンプレートまで効く。ミューテーションテストは、テンプレートの`{}`の中の式に効く。
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
- 運び方、仮想リスト、キャレットの計測、ドラッグは、試作のあとの別のADRで決める。
