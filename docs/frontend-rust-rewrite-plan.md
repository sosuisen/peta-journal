# frontend を Rust(Crux + Leptos)で書き換える計画

現在の `frontend` は Angular 21 で書かれている(本体約 9,100 行、単体テスト約 3,100 行、Playwright の E2E 27 件)。これを Rust の Crux(共有コア)+ Leptos(Web シェル)で書き換えるための計画をまとめる。サーバー側の書き換え計画は [rust-rewrite-plan.md](rust-rewrite-plan.md) を参照。

結論から言うと、書き換えは可能である。Angular 側はサービス層が DOM に触らず、`ApiService` と `OutlineView` インターフェースを引数で受け取る設計になっているため、Crux の「副作用のないコア + 薄いシェル」にほぼ 1 対 1 で移せる。ただし、Rust 側に代替がなく自作になるものが 2 つある。Angular CDK の可変高さ仮想スクロールと、Flat Markdown AST のレンダラー(`flatmarkdown-ast2angular`)である。

## 0. 進捗(2026-10-16 時点)

移植は `frontend-rust/` に完了している。フェーズ 0〜5 はすべて実装済みで、次を確認した。

| 確認したこと | 結果 |
|---|---|
| `cargo test`(コア 100 件 = vitest の spec を移植したもの + view のテスト、fm-ast 17 件、fm-leptos 13 件) | すべて通過 |
| Playwright の E2E(`PETA_JOURNAL_FRONTEND=rust PETA_JOURNAL_SERVER=rust npm run e2e`、27 件。spec は無変更) | すべて通過 |
| 同じ E2E を Angular 版に向けて実行(`PETA_JOURNAL_SERVER=rust npm run e2e`) | 26 件通過。「a selection that grows over a parent」は Angular 版でも 3 回に 1 回落ちる(元からのフレーク。Rust 版では通過) |
| `cargo clippy --all-targets`(全 crate) | 警告なし |
| `trunk build --release` の wasm | 1.9 MB(gzip 後 0.7 MB) |

構成は 3 章のとおりで、crate 名は `fm-ast`、`fm-leptos`(レンダラーは `shell-leptos` から分けた)、`peta-core`、`peta-frontend`(shell-leptos)。ビルドと起動の手順は [BUILD.md](../BUILD.md) の「Rust frontend」を参照。

### 実装で決めたこと

- **API を待つ処理の書き方**: Angular のサービスは `await api...` の前後で状態を書き換える。Crux の `update` は同期なので、非同期コマンドの中からモデルに触るために `Event::Continue(Thunk)`(`FnOnce(&mut Model) -> Command` を運ぶイベント)を用意し、`apply(&api, "ラベル", |m| ...)` で送る。コアは次の `update` でそれを実行してから、続く effect をシェルに渡す。これで `item-split.service.ts` などの流れをほぼそのままの順序で移せた(`crates/core/src/app.rs` の冒頭に説明がある)。
- **シェルからの読み取り**: 同じ仕組みで `read(&api, |m| ...)` が oneshot チャネルで値を返す。
- **通知と要求の区別**: `Ui` の操作のうち `FocusEditor`、`ReadClipboardText`、`Confirm`、`Prompt`、`LocalNow` だけが答えを待つ要求で、残り(タイトル、スクロール、クリップボード書き込み、ダイアログ、拡張機能)と `Navigate` は `notify_shell` による通知。シェルは通知を resolve してはいけない(debug ビルドでは Crux が panic する)。
- **フォーカスの扱い**: Angular の `pendingCursorPosition` は、ViewModel の `focus_request { seq, item_id, cursor, mouse_drag }` に置き換えた。エディタは自分宛ての未処理の seq を見てカーソルを置く。「次の描画後にフォーカスして保留キーを再生する」`focusEditor()` は `UiOp::FocusEditor` の要求で、シェルは `setTimeout(0)` 後に実行して resolve する。
- **Leptos 側の描画**: ViewModel の行は id ごとの `RwSignal<Row>` に差分で流し込み(`RowStore`)、ウィンドウ内の行だけを `<For>` で描く。本文の編集/表示の切り替えは `Memo<(broken, editing)>` で判定する。行全体の変化で切り替えると、1 キーごとにエディタが作り直されてフォーカスが外れる。
- **mousedown の既定動作**: 表示中の項目を押すと同じハンドラの中でエディタが現れてフォーカスを取るため、ブラウザ側の mousedown 既定動作(フォーカス移動)で外れてしまう。`preventDefault()` で止めている(Angular は描画が非同期だったので不要だった)。
- **Send 境界**: Leptos 0.8 はクロージャに `Send + Sync` を要求する。コアは thread local に置き、`CoreHandle` は `Copy` の空の型にした。コンポーネントの状態は `StoredValue`(ブラウザの型は `StoredValue::new_local`)に置く。`Copy` なので各クロージャへそのまま渡せ、コンポーネントと一緒に破棄される。`SendWrapper<Rc<RefCell<T>>>` は使わない。
- **遅延コールバックの規律**: `after_render` やタイマー、`requestAnimationFrame` の中で signal や `NodeRef` を読むときは必ず `try_` 系(`try_get_untracked`、`try_update_value`)を使う。コンポーネントが先に消えていると、通常の読み取りは「disposed」のパニックで wasm 全体を止める。
- **document のリスナー**: 選択モードのキー、メニューを閉じるクリック、サイドバーのリサイズ、エディタからのドラッグは `listeners::DocumentListeners`(`on` / `on_cancelable` / `on_capture` / `clear` / `clear_later`)で登録する。リスナーは自分自身の中からは外せないので、閉じる処理は `clear_later` を使う。
- **行の派生値はコアで**: タスクの状態(`Row.task: Option<TaskState>`)、1 行見出しかどうか(`heading_line`)、独自ズームの有無(`has_own_zoom`)はコアの `Row` が持ち、シェルは読むだけ(`tests/view.rs` で検証)。シェルのコンポーネントは `core.select(|v| ...)` で ViewModel の一部を `Memo` として受け取る。
- **カレンダーと localStorage**: サイドバー幅、折りたたみ状態、カレンダーの表示月はシェルだけの状態で、コアには入れていない。
- **highlight.js**: npm の `highlight.js/lib/common` を esbuild で 1 本にまとめて `vendor/` に置いた(再生成手順は BUILD.md)。
- **時刻**: コアは時計を持たない。`today` は `Route` イベントに、`now_ms` は `BodyChanged` / `EnterPressed` に乗せ、タイトル生成などは `UiOp::LocalNow` で聞く。

### Angular 版との差

- 画像(`![alt](url)`)は `<img>` として描く。Angular 版のレンダラーは Rust の flatmarkdown が出す `embed` ノードを扱わないため、何も表示していなかった。
- `GET /api/functions` と `POST /api/launcher/open` は Angular 版と同じく `http://localhost:8087` 固定。E2E(8187)では接続拒否のログが出るが動作には影響しない。
- 仮想スクロールは自作(`components/outline.rs`)。計測済みの高さと既定値 28px で上下の padding を決め、CDK と同じ 500px / 1000px のバッファで描く。E2E の「slow scrolling keeps a steady pace」は通過する。
- `window.PETA_DEBUG = true` にすると、コアへ送るイベントがコンソールに出る。

### 残っていること

- wasm のサイズを小さくする余地がある(`wasm-opt` は trunk が `data-wasm-opt="z"` で実行済み。comrak が大きい)。
- 拡張機能の iframe、画像の貼り付け、クリップボードへのリッチコピー、トランスクルージョンのメニューはコアのテストでは確認したが、ブラウザでは手で確かめていない。

## 1. ゴールと守ること

目的は「フロントエンドを Rust に寄せ、ロジックをブラウザから独立した形でテストできるようにすること」であり、機能の追加や見た目の変更ではない。次のものは **一切変えない**。

| 守るもの | 理由 |
|---|---|
| サーバーの REST API、SSE(`/api/events`)、JSON の形 | `journal-server`(Java)と `journal-server-rust` のどちらとも無変更でつながる |
| ルーティング(`/today`、`/pages/**`、`/docs/**`、`/settings`、`/journals/:date` のリダイレクト) | ブックマーク、拡張機能、デスクトップアプリからの URL スキームがそのまま動く |
| 拡張機能との `postMessage` プロトコル(`PageCreate`、`CreateItem`、`CreateItemResult`、`CreateItemError`) | `ext-functions/` が無変更で動く |
| localStorage のキー `peta-journal-config` | サイドバー幅の設定を引き継ぐ |
| E2E テストが見ている DOM(`app-outline-item`、`app-item-view`、`#item-editor-<id>`、`[id="<itemId>"]`、`textarea`) | `frontend/e2e/` の 27 件をそのまま回帰テストに使う |
| キーボード操作と IME の扱い(`isComposing` / `keyCode 229`) | 日本語入力の使い勝手を落とさない |

Angular 版 `frontend/` はそのまま残し、`frontend-rust/` を隣に作る(サーバーの時と同じ方針)。

## 2. 現状の棚卸し

### 規模

| 区分 | 行数 |
|---|---|
| 本体 TypeScript(spec 除く) | 約 9,100 行 |
| 単体テスト(vitest、フェイク API 相手の純粋なテストが大半) | 約 3,100 行 |
| Playwright E2E(`e2e/*.spec.ts`) | 27 件、約 520 行 |

### DOM に依存しない部分(= Crux コアの候補、約 3,500 行)

| ファイル | 行数 | 内容 |
|---|---|---|
| `core/services/outline-state.service.ts` | 450 | items、編集中・選択中・折りたたみの集合、transclusion ツリー、`visibleItems` の計算 |
| `core/services/undo.service.ts` | 155 | Undo / Redo スタック、本文編集の 500ms グルーピング |
| `features/outline/outline-tree.ts` | 291 | 位置計算、`canPlace`、`canIndent`、選択範囲、`EditContext`、`TransclusionContext` |
| `features/outline/move-utils.ts` | 153 | 上下移動とページ跨ぎ移動の移動先計算 |
| `features/outline/item-split / merge / move / edit / action / paste / level-shift / journal-day-move / undo-replay / file-reload / key-relay` | 約 1,800 | 各操作。`ApiService` と `OutlineView` を受け取り、API を待ってから続きをする |
| `core/services/auto-save.service.ts` | 73 | 300ms デバウンスの保存 |
| `core/services/sse.service.ts` | 72 | 再接続(指数バックオフ、最大 10 回) |
| `core/services/api.service.ts` | 277 | fetch のラッパー約 30 本 |
| `core/utils/*.ts` | 約 200 | 日付、週番号、ファイル名検証、タイトル生成 |

### DOM に依存する部分(= Leptos シェル、約 5,500 行)

| ファイル | 行数 | DOM に触る理由 |
|---|---|---|
| `features/outline/outline.ts` | 847 | CDK の `autosize` 仮想スクロール、スクロール位置の監視、項目へのスクロール、移動のキュー |
| `features/outline/item-editor.ts` | 720 | textarea のカーソル位置計測(ミラー div)、IME、貼り付け、ドラッグ検知、キーの再生 |
| `features/outline/outline-item.ts` | 472 | 1 行の見た目、メニュー、折りたたみボタン |
| `features/outline/item-view.ts` | 330 | AST の描画(`fm-root`)、highlight.js、`caretRangeFromPoint` でクリック位置の文字オフセット |
| `features/journal/page.ts`、`journal.ts`、`page-dispatcher.ts` | 848 | ページ見出しの編集、日付跨ぎの読み込み、`document.title` |
| `features/sidebar/sidebar.ts`、`shared/calendar.ts` | 656 | カレンダー、目次、最近のページ |
| `features/search/search-bar.ts`、`settings/settings.ts` | 412 | 検索ドロップダウン(200ms デバウンス)、設定画面 |
| `layout/main-layout.ts` | 372 | ヘッダー、サイドバーのリサイズ、接続状態 |
| `features/outline/wiki-suggestions.ts`、`outline-selection.ts` | 459 | `[[` 補完のポップアップ、複数選択とキー操作 |
| `core/services/extension-function.service.ts`、`global-url-scheme-interceptor.service.ts`、`app-launcher.service.ts` | 301 | 非表示 iframe と `postMessage`、カスタム URL スキームのクリック横取り |

### 依存ライブラリと Rust 側の対応

| Angular 側 | Rust 側 | 備考 |
|---|---|---|
| Angular 21(signals、router、forms) | Leptos 0.8(`csr`)、`leptos_router` | 0.8.22 が安定版。0.9 は rc |
| `@angular/cdk/scrolling` の `autosize` 仮想スクロール | **自作**(高さキャッシュ + 上下スペーサー) | 後述 |
| `flatmarkdown-ast2angular` 0.3.5 | **自作** `fm-ast` crate(型付き AST)+ Leptos レンダラー | 後述 |
| `flatmarkdown-ast` 0.2.0(TypeScript の型 46 種) | `fm-ast` の serde 構造体 | 同じ `type` タグで `#[serde(tag = "type")]` |
| `hmtid` 1.0.0(npm) | `journal-server-rust/crates/hmtid` を path 依存 | wasm32 では `getrandom` の `wasm_js` バックエンドが必要 |
| `highlight.js` | highlight.js を `index.html` で読み込み、`wasm-bindgen` 経由で `hljs.highlightElement` を呼ぶ | syntect を wasm に入れるとバイナリが大きくなる |
| `fetch` | `gloo-net` または `web-sys` の `fetch` | |
| `EventSource` | `web-sys` の `EventSource` | |
| `localStorage` | `web-sys` の `Storage` | |
| vitest(フェイク API 相手) | `cargo test`(`App::update` を呼び、返った `Command` の effect と event を検査) | Crux 0.17 以降 `AppTester` は非推奨 |
| Playwright | そのまま。`PETA_JOURNAL_FRONTEND=rust` で `trunk serve` を起動する | |

### wasm32 でのビルド確認(2026-10-16)

スクラッチの crate で `cargo check --target wasm32-unknown-unknown` を通した。

| crate | 版 | 結果 |
|---|---|---|
| crux_core | 0.20.0 | 通過 |
| leptos(`csr`) | 0.8.22 | 通過 |
| flatmarkdown | 0.4.3(comrak 0.51) | 通過 |
| hmtid(リポジトリ内) | path | `getrandom = { features = ["wasm_js"] }` と `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'` で通過 |

Crux の Rust シェルはコア crate を直接依存して `Core::process_event` / `resolve` / `view` を呼ぶので、bincode の直列化層は要らない。公式の `examples/counter/web-leptos` も依存は `leptos 0.8 (csr)` と `shared` だけである。Crux 自体は pre-1.0(「本番利用可だが API の破壊的変更あり」と明記)なので、バージョンは固定する。

## 3. 構成

```
frontend-rust/
  Cargo.toml                 ワークスペース
  .cargo/config.toml         wasm32 を既定ターゲットに、getrandom の wasm_js を cfg
  crates/
    fm-ast/                  Flat Markdown AST の型(serde)。UI 非依存
    core/                    Crux コア(peta_core)。DOM 非依存、cargo test で動く
      src/model/             Item、Page、SearchResult、ItemPosition(サーバーの JSON と同じ形)
      src/outline/           state(items、選択、折りたたみ、transclusion)、tree(位置計算)、undo
      src/ops/               split、merge、move、level_shift、edit(自動保存)、action、paste、
                             journal_day_move、undo_replay、file_reload、key_relay
      src/effects/           Api、Timer、Sse、Ui、Navigate、Storage の Operation 型
      src/app.rs             App(Event、Model、ViewModel、update、view)
    shell-leptos/            Leptos の Web シェル(trunk でビルド)
      Trunk.toml             /api を 8087 にプロキシ
      index.html             highlight.js の読み込み、styles.css
      src/core.rs            Core を持ち、Effect を実行して resolve するループ
      src/effects/           api(fetch)、timer、sse、ui(focus / scroll / clipboard)、storage
      src/components/        main_layout、sidebar、calendar、search_bar、settings、page、journal、
                             outline、outline_item、item_editor、item_view、wiki_suggestions
      src/dom/               textarea の計測、caretRangeFromPoint、virtual_list
      src/fm_render.rs       fm-ast → Leptos view
```

### Crux コアの形

- **Event**: UI からの入力(`EditRequested { item_id, cursor }`、`BodyChanged`、`EnterPressed { cursor }`、`Indent`、`Outdent`、`MoveUp`、`Paste { text, html, cursor }`、`Undo`、…)と、シェルからの結果(`ApiDone(ApiResult)`、`TimerFired(id)`、`SseMessage`、`PageLoaded`、…)。
- **Model**: `OutlineState`(Angular の `OutlineStateService` と同じ)、`UndoStacks`、現在ページ、設定、検索、サイドバーのデータ、SSE の接続状態。
- **Effect**(`#[effect]`):
  - `Render` … 標準。
  - `Api(ApiOp)` … `GetPage { id }`、`CreateItem { … }`、`UpdateItem { … }`、`MoveItems { … }` など、`api.service.ts` の 1 メソッドが 1 variant。Output は `Result<ApiOutput, String>`。URL 文字列をコアに持ち込まない。
  - `Timer(TimerOp)` … `Start { id, ms }` / `Cancel { id }`。自動保存の 300ms、検索の 200ms、SSE の再接続待ちに使う。
  - `Sse(SseOp)` … `Connect`。`stream_from_shell` で `Connected` / `PagesUpdated` / `Error` を受け取る。
  - `Ui(UiOp)` … `FocusEditor { item_id, cursor }`、`ScrollToItem { item_id }`、`SetTitle`、`CopyToClipboard { text }`、`OpenWindow { url, new_window }`。シェルが DOM で済ませる通知。
  - `Navigate(url)` … ルーター遷移。
  - `Storage(StorageOp)` … localStorage の読み書き。
- **ViewModel**: 画面に出す値だけ。`OutlineView { rows: Vec<Row>, … }` の `Row` は Angular の `app-outline-item` の入力(item、インデント、選択・編集・折りたたみ・見出しレベル・子の有無・日付ヘッダー)を事前計算したもの。
- **非同期の流れ**: `item-split.service.ts` のように「API を待ってから編集を続ける」処理は `Command::new(|ctx| async move { … ctx.request_from_shell(ApiOp::…).await … })` で書く。Angular の `enqueueMove` のような直列化はコア側のキューで行う。

### Leptos シェルの形

- `Core<PetaApp>` を `Rc` で持ち、`update(event)` は `process_event` → 返った Effect を `process_effect` で処理する。`Render` は `RwSignal<ViewModel>` に書き、`Api` / `Timer` / `Sse` は `spawn_local` で実行して `core.resolve` に返し、さらに返った Effect を同じ関数で処理する(Crux の標準的なループ)。
- 行の描画は `<For each=rows key=id>` で、行ごとの `RwSignal<Row>` を ViewModel 更新時に差分で書き換える。全行を作り直さないため、1 キーごとの Render でも軽い。
- E2E が見るタグ(`app-outline-item`、`app-item-view`)はカスタム要素名としてそのまま出す。`id` と `class` も Angular 版と同じにする。

## 4. 個別に検討が必要なもの

### 4.1 仮想スクロール

`outline.ts` のコメントにあるとおり、CDK の `autosize` とブラウザのスクロールアンカリング(`overflow-anchor`)の衝突で苦労した箇所である。Leptos に成熟した可変高さ仮想スクロールはないので、同じ方式を自作する。

- 項目 ID ごとの高さキャッシュ(未計測は既定値)。描画後に `offsetHeight` を読んで更新する。
- 表示範囲 + 前後のバッファ(CDK と同じ 500px / 1000px)だけを描画し、上下は 1 枚ずつのスペーサー div で高さを埋める。
- `overflow-anchor: none` は踏襲する。
- `scrollToItem` は、未描画なら推定位置へスクロールしてから数回描画して合わせる(Angular 版の `scrollSearchStep` と同じ)。
- ジャーナルの「上にスクロールしたら前の日を読み込む」も同じ仕組みの上に載せる。

最初のスパイクで 1,000 項目を入れ、Chrome でスクロールが滑らかかを確認してから先へ進む。

### 4.2 AST レンダラー(fm-ast)

Rust の `flatmarkdown` crate は AST を JSON 文字列で返すだけで、型付き構造体を持たない。`flatmarkdown-ast` npm の 46 ノード型を serde の enum(`#[serde(tag = "type", rename_all = "snake_case")]`)として `fm-ast` crate に書く。レンダラーは `flatmarkdown-ast2angular` 0.3.5 と同じ HTML を出す。

- `wikilink` → `<a class="wiki-link" href="/pages/{url}">`、`hashtag` → `<a class="hashtag-link" href="/pages/{value}">`。クリックは `item-view` が横取りしてルーターへ渡す(Ctrl+クリックは既定に任せる)。
- `footnote_definition` → `<div class="footnote" id="fn-{name}">`、`footnote_reference` → `<sup class="footnote-ref"><a id="fnref-{name}" href="#fn-{name}">`。クリックで同一ページ内をスクロール。
- `task_item` → `<li><input type="checkbox" disabled>`。
- `code_block` → `<pre><code class="language-{info}">`。描画後に highlight.js を当て、言語ラベルを付ける。
- `html_block` / `html_inline` → `inner_html`(Angular 版と同じ扱い)。

`fm-ast` は UI 非依存なので、将来 `journal-server-rust` から AST を扱う時にも使える。型付き AST を `flatmarkdown` crate 本体に入れる選択肢もあるが、本計画では別 crate にとどめる。

### 4.3 textarea の扱い

`item-editor.ts` の処理は全部 `web-sys` で書ける。量が多いだけで、設計上の難しさはない。

- カーソルが最初の行 / 最後の行にあるかの判定、`[[` 補完ポップアップの位置 … ミラー div で計測(同じ方法)。
- IME … `KeyboardEvent::is_composing()` と `key_code() == 229`。
- 貼り付け … `ClipboardEvent::clipboard_data()` から `text/plain`、`text/html`、画像ファイル。
- `KeyRelayService`(次のエディタが出るまでに押されたキーの保留と再生)… ロジックはコア、`dispatchEvent` による再生はシェル。

### 4.4 ハイライト

`highlight.js` は `index.html` に `<script>` で読み込み、`wasm_bindgen` の `extern "C"` で `hljs.highlightElement(el)` を呼ぶ。Angular 版と同じ見た目になり、wasm のサイズも増えない。

### 4.5 拡張機能とカスタム URL スキーム

- `ExtensionFunctionService` … 起動時に `GET /api/functions` を取り、非表示 iframe を作る。`window` の `message` イベントで `CreateItem` を受け、コアの `Event::ExtensionCreateItem` に渡し、結果を `postMessage` で返す。
- `GlobalUrlSchemeInterceptorService` … `document` のキャプチャ段階の `click` で `xxx://` のリンクを横取りし、`POST /api/launcher/open` を呼ぶ。

### 4.6 ビルドと開発

- `trunk serve` が `/api` を `http://localhost:8087` にプロキシする(`Trunk.toml`)。SSE もプロキシ経由で流れることを確認する。
- 配布は `trunk build --release` の `dist/`。
- `.cargo/config.toml` に `[build] target = "wasm32-unknown-unknown"` と `rustflags = ['--cfg', 'getrandom_backend="wasm_js"']` を書く。`core` と `fm-ast` のテストは `cargo test --target x86_64-unknown-linux-gnu`(ホスト)で回す。

### 4.7 wasm のサイズ

comrak を含むので Angular 版のバンドルより大きくなる見込み。`opt-level = "z"`、`lto = "fat"`、`wasm-opt` を有効にし、スパイクの段階で実測する。

## 5. 守るべき E2E の DOM

`frontend/e2e/support.ts` と各 spec が使っているセレクタ。

| セレクタ | 用途 |
|---|---|
| `app-outline-item` | 行の数、本文一覧(`textarea` の `value` か `.item-body` / `app-item-view` の `textContent`) |
| `[id="<itemId>"]` | 行をクリックして編集を開く |
| `#item-editor-<itemId>` | 編集中の textarea。フォーカスの確認、キー入力 |
| `app-item-view`、`.item-body` | 表示モードの本文 |

## 6. 進め方

| フェーズ | 内容 | 完了の目安 |
|---|---|---|
| 0 | ワークスペースの雛形、trunk でのビルド、`/api` プロキシ、hello world | ブラウザで表示できる |
| 1 | `fm-ast` と Leptos レンダラー、highlight.js の連携 | 既存ページの AST が Angular 版と同じ HTML になる |
| 2 | Crux コア(モデル、状態、ツリー操作、Undo、各操作、effect)。vitest の spec を Rust のテストに移す | `cargo test` が通る |
| 3 | シェル: アウトライン(仮想スクロール、エディタ、表示、`[[` 補完、複数選択) | 1 ページの編集が一通りできる |
| 4 | シェル: レイアウト、ルーティング、ページ見出し、ジャーナル、サイドバー、カレンダー、検索、設定、拡張機能 | Angular 版と同じ画面がそろう |
| 5 | Playwright を `PETA_JOURNAL_FRONTEND=rust` で Rust 版に向け、27 件を通す | すべて通過 |
| 6 | BUILD.md、この文書の進捗、不足機能の記録 | |

## 7. 判断と注意

- **Crux を使う意味**: Web だけが目標なら、Crux の Event / Effect の層はやや大げさで、同じ純粋関数群を普通の crate にして Leptos から呼ぶ方が簡単である。それでも Crux を選ぶのは、(1) Angular 版のサービス層がすでに「API とビューを注入してテストする」形で、Crux の形に素直に載ること、(2) コアを Tauri / iOS / Android のシェルへ使い回す余地を残せること、の 2 点による。
- **Crux のバージョン**: pre-1.0 なので `crux_core = "=0.20.0"` に固定する。
- **二重管理**: サーバーと同じく、Angular 版は残す。E2E はどちらのフロントにも向けられるようにする。
- **手数**: サーバー移植(Java 約 12,000 行)より行数は少ないが、DOM 依存部分の割合が高く、web-sys は TypeScript より冗長なので、行数は 1.3〜1.5 倍になる見込み。
