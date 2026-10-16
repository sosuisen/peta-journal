# journal-server を Rust + axum で書き換える計画

現在の `journal-server` は Java 25 + Quarkus 3.31 で書かれている(本体とテストで約 12,000 行)。これを Rust + axum に書き換えるための計画をまとめる。

結論から言うと、書き換えは現実的である。層の分かれ方(domain / application / infrastructure / presentation)はそのまま Rust のモジュールに移せる。依存ライブラリも、すべてに Rust 側の対応物がある。本体の markdown パーサー(flatmarkdown)はもともと Rust 製なので、むしろ Java 版より単純になる。

## 0. 進捗(2026-10-16 時点)

移植は `journal-server-rust/` に完了している。フェーズ 0〜4 はすべて実装済みで、次を確認した。

| 確認したこと | 結果 |
|---|---|
| `cargo test`(単体テスト 54 件 + API テスト 40 件。Java の `*ApiTest` と `FolderSyncTest` を移植したもの) | すべて通過 |
| Playwright の E2E(`PETA_JOURNAL_SERVER=rust npm run e2e`、27 件。フロントエンドは無変更) | すべて通過 |
| REST / SSE / MCP(26 ツール)/ `/api/sync/pending-deletes` / assets / functions / CORS を実バイナリに対して curl で確認 | 同じ URL と JSON で動作 |
| 日本語検索(`空港` → 「東京国際空港」、`行く` → 「行きました」、`コンピュータ` → 「コンピューター」、`#旅行` のタグ検索) | Kuromoji と同じ構成(分割モード、原形化、品詞ストップ、NFKC、カタカナ語幹、小文字化)で一致 |
| `cargo clippy --all-targets` | 警告なし |

残っている判断(本文 8 章)のうち決めたもの:

- ディレクトリ名は `journal-server-rust`。Java 版はそのまま残してある。
- hmtid は `journal-server-rust/crates/hmtid` に置いた(npm 版と同じ仕様、同じテストベクタ)。
- MCP のパスは `/mcp`(quarkus-mcp-server-http の既定と同じ)。
- 検索インデックスは起動時に Lucene 形式を検出すると消して再構築する。ログに出す。
- `frontend/playwright.config.ts` は `PETA_JOURNAL_SERVER=rust` で Rust バイナリを起動する。

ビルドと起動の手順は [BUILD.md](../BUILD.md) の「Rust server」を参照。

## 1. ゴールと守ること

書き換えの目的は「JVM を無くして配布と起動を軽くすること」であり、機能の追加や仕様の変更ではない。次のものは **一切変えない**。

| 守るもの | 理由 |
|---|---|
| REST API の URL、メソッド、JSON の形(`/api/...`) | `frontend`、`ext-web-app/todo`、`ext-desktop-app/stickies`、`ext-functions` がそのまま動く |
| SSE の URL(`/api/events`)とイベント JSON(`connected`、`pagesUpdated`) | 同上 |
| MCP のツール名、引数名、戻り値の JSON(26 本) | 利用側の MCP クライアント設定を変えずに済む |
| HTTP ポート `8087`、CORS の許可オリジン | 同上 |
| SQLite のスキーマ(`~/.peta-journal/data/journal.db`) | 既存データをそのまま開く。マイグレーションは不要 |
| `~/.peta-journal/config.json`、`functions/` の配置 | 同上 |
| Markdown ファイルの形式(Flat Markdown v0.2)と `file_sync_state` の判定 | 他 PC との同期が壊れない([folder-sync-journeys.md](folder-sync-journeys.md) 参照) |
| HMTID の形式と時刻順の性質 | ID の衝突判定(古い ID が勝つ)がそのまま動く |

変えてよいのは **検索インデックスだけ**(Lucene → tantivy は形式に互換性がない。起動時に再構築する。後述)。

## 2. 現状の棚卸し

### 構成

```
journal-server/src/main/java/com/petajournal/server/
  domain/          FractionalIndexer, HashtagExtractor, IdGenerator, ItemBody, model/*, repository/*(interface)
  application/     ItemService, PageService, JournalService, ConfigService, CombinedSearchService,
                   FlatMarkdownSerializer/Splitter, TreeMarkdownConverter, SearchIndexRebuilder ...
  infrastructure/
    persistence/   SqliteItemRepository, SqlitePageRepository, PropertyRepository, DatabaseInitializer
    search/        LuceneSearchService
    export/        MarkdownExportService, MarkdownImportService, MarkdownReconcileService, PageMatcher, ItemMerger ...
    watch/         MarkdownFolderWatcher
    parser/        FlatmarkdownParser
    launcher/      AppLauncher
    startup/       ConfigurationStartup, SchemaVersionCheck
  presentation/
    rest/          *Resource(REST)、SseBroadcaster、dto/*
    mcp/           Mcp*Tools(26 ツール)
```

### 外部インターフェース(変えないもの)

REST(`presentation/rest/`):

| リソース | エンドポイント |
|---|---|
| health | `GET /api/health` |
| pages | `GET /api/pages/{id}`、`POST /api/pages/doc`、`PUT /api/pages/{id}`、`DELETE /api/pages/{id}`、`GET /api/pages/recent`、`GET /api/pages/{id}/backlinks`、`GET /api/pages/{id}/children` |
| items | `POST /api/items`、`GET /api/items?types=`、`GET/PUT/DELETE /api/items/{id}`、`POST/DELETE /api/items/{id}/transclude`、`POST /api/items/{id}/cycle-task`、`POST /api/items/move`、`POST /api/items/shift-level`、`PUT /api/items/{id}/properties/{key}`、`POST /api/items/export`、`/export-page`、`/export-subtree`、`/import`、`/parse` |
| journals | `POST /api/journals/{name}`、`GET /api/journals/before`、`GET /api/journals/weekly/before` |
| search | `GET /api/search/pages`、`/combined`、`/tags` |
| config | `GET/PUT /api/config`、`GET /api/config/property-definitions`、`POST /api/config/browse-folder` |
| sync | `GET /api/sync/status`、`POST /api/sync/scan`、`GET/POST /api/sync/pending-deletes`(HTML を返す) |
| assets | `POST /api/assets/upload`(multipart)、`GET /assets/{fileName}` |
| functions | `GET /api/functions`、`GET /api/functions/{dirName}/{path}` |
| launcher | `POST /api/launcher/open` |
| events | `GET /api/events`(SSE) |

MCP(`presentation/mcp/`、Streamable HTTP):`create_item`、`cycle_task`、`delete_item`、`delete_page`、`export_page_markdown`、`export_subtree_markdown`、`export_tree_markdown`、`get_backlinks`、`get_config`、`get_item`、`get_items_by_type`、`get_most_recent_journal_before`、`get_or_create_doc`、`get_or_create_journal`、`get_page_by_id`、`get_recent_pages`、`import_tree_markdown`、`move_items`、`remove_transclusion`、`search_combined`、`search_page_names`、`set_transclusion`、`update_config`、`update_item`、`update_page`、`upsert_property`。

定期処理(`@Scheduled`):

| 処理 | 間隔 |
|---|---|
| `MarkdownExportService` export_queue の書き出し | 5 秒(起動 5 秒後から) |
| `MarkdownReconcileService` フルスキャン | 60 秒(起動 10 秒後から) |
| `MarkdownReconcileService` 削除確認の再表示判定 | 2 秒 |

設定(`application.properties` の `petajournal.*`):`data.dir`、`data.db`、`index.dir`、`export.dir`、`config.dir`、`lib.dir`、`export.interval`、`import.enabled`、`import.debounce-ms`、`import.scan-interval`、`import.delete-prompt-cooldown`、`quarkus.http.port`。E2E テスト(`frontend/playwright.config.ts`)は `-Dquarkus.http.port=8187 -Dpetajournal.data.dir=...` で上書きしているので、Rust 版でも同じ項目を環境変数か引数で上書きできる必要がある。

### 依存ライブラリと Rust 側の対応

| Java 側 | 使い方 | Rust 側 | 備考 |
|---|---|---|---|
| Quarkus REST + Jackson | 約 40 エンドポイント | axum + serde / serde_json | tower-http の `CorsLayer` で CORS |
| SSE(Mutiny `Multi`) | `SseBroadcaster` | axum の `Sse` + `tokio::sync::broadcast` | |
| quarkus-scheduler | 上の 3 本 | `tokio::time::interval` を spawn | |
| sqlite-jdbc | 3 リポジトリ + `DatabaseInitializer` | rusqlite(`bundled` feature) | WAL、`foreign_keys=ON` も同じ PRAGMA |
| Lucene + Kuromoji + Highlighter | `LuceneSearchService` | tantivy 0.26 + lindera-tantivy 5(`embed-ipadic`) | 後述 |
| flatmarkdown(Maven 版 + 同梱 .so) | `pageToAst`、`bodyToAst` | [petajournal/flatmarkdown](https://github.com/petajournal/flatmarkdown) crate を直接依存 | comrak ベース。AST JSON は同一 |
| hmtid(Java 版) | 生成 2 か所、`HMTID.of` 約 54 か所、`compareTo` 5 か所 | Rust 版を書く(`hmtid` crate として切り出す) | 後述 |
| quarkus-mcp-server-http | 26 ツール | rmcp(公式 Rust MCP SDK)の Streamable HTTP を axum に載せる | |
| `java.nio.file.WatchService` | `MarkdownFolderWatcher` | notify + notify-debouncer-full | 2 秒デバウンスはそのまま |
| `ProcessBuilder`(zenity / osascript / PowerShell) | `FolderChooserService` | `std::process::Command` | 同じスクリプトを呼ぶだけ |
| `java.awt.Desktop.browse` | `AppLauncher` | open crate | |
| JUnit + REST Assured | API テスト 7 本 + 単体テスト | Rust の `#[tokio::test]` + reqwest(または `tower::ServiceExt::oneshot`) | 後述 |

## 3. 個別に検討が必要なもの

### 3.1 flatmarkdown

本体は Rust crate(`flatmarkdown` 0.4.3、依存は comrak 0.51 と serde_json)。Java 版はこれを別途ネイティブ化して Maven Central から配っている。Rust 版サーバーでは `Cargo.toml` に crate をそのまま書けばよい。`lib/linux/libpeta_markdown.so`、`lib/windows/libpeta_markdown.dll` と `petajournal.lib.dir` は不要になる。

`MarkdownParser` インターフェースの 2 メソッドは次に対応する。

| Java | Rust |
|---|---|
| `parseToAst(markdown)` → `FlatmarkdownJava.bodyToAst` | `flatmarkdown::body_to_ast` |
| `parsePage(markdown)` → `FlatmarkdownJava.pageToAst` | `flatmarkdown::page_to_ast` |

Maven 版のラッパー(`com.sosuisha:flatmarkdown:1.0.1`)の中身は flatmarkdown 0.4.3 そのもの(同梱の `libflatmarkdown_java.so` に crate 名とバージョンが残っている)に、C ABI の薄い関数 4 つ(`body_to_ast`、`body_to_html`、`page_to_ast`、`free_string`)を足したものだった。独自機能の追加はない。Java 側の `markdownToAst` / `markdownToHtml` は `bodyToAst` / `bodyToHtml` の別名。`BodyOptions` は `link_resolution = On { prefixes: ["./", "../"] }` 相当で、Rust 版サーバーも同じ値にしてある(12 種類の本文で AST の JSON がバイト単位で一致することを確認した)。

### 3.2 HMTID

`HMTID` は `2026-09-29-00-00-01-AAAAAAA` の形で、時刻部とランダム部からなる。サーバーは次の性質に依存している。

- `monotonicFactory(random, '-', true)` で単調増加に生成する(ページ用とアイテム用に生成器が 1 つずつ)
- 文字列からパースし、不正なら `IllegalArgumentException`(`PageMatcher.parseId` が `null` に落とす)
- `compareTo` が時刻順になる(同期時に「古い方の ID が勝つ」判定に使う)
- フロントエンドの npm 版 `hmtid` と同じ文字列を生成する(クライアント側でも ID を作る)

Rust 版の公開 crate は無いので、npm 版の実装を仕様として `crates/hmtid` に書き直した。Java 版(`com.sosuisha:hmtid:2.1.0`)の `HMTID.of` を実際に呼んで突き合わせた結果、受け付ける形式(22 文字 / 27 文字、`_` と `-`、区切りの混在は不可)、ランダム部の文字集合、日付の妥当性検査(13 月や 2 月 30 日は拒否)、表記違いの同値性(`2026-09-29-...` と `20260929..._...` は等しい)、`compareTo` の順序がすべて同じになるようにしてある。

### 3.3 検索(Lucene → tantivy + lindera)

`LuceneSearchService` が使っているのは次だけ。

- `JapaneseAnalyzer` の既定構成
- フィールド:`type`(page / item)、`pageId`、`pageTitle`(TextField)、`itemId`、`body`(TextField)。すべて Store
- ページ名検索:`QueryParser` で `query*`、無ければ `*query*`(AND 既定)
- 全文検索:`QueryParser`(AND 既定)+ `Highlighter`(`<mark>`、100 文字の `SimpleFragmenter`)
- 削除:`itemId` / `pageId` の Term 削除、`deleteAll`
- `isEmpty()` で空なら `SearchIndexRebuilder` が SQLite から再構築

tantivy 側の対応:

| Lucene | tantivy / lindera |
|---|---|
| `JapaneseAnalyzer` = JapaneseTokenizer(search mode)+ BaseForm + PartOfSpeechStop + CJKWidth + KatakanaStem + LowerCase + StopWords | `LinderaTokenizer`(`Mode::Decompose`)+ `japanese_base_form` + `japanese_stop_tags` + `unicode_normalize`(NFKC)+ `japanese_katakana_stem` + `lowercase` + `stop_words` |
| 辞書 IPADIC(Kuromoji 同梱) | `embed-ipadic` feature でバイナリに埋め込む |
| `QueryParser` AND 既定 | `QueryParser::set_conjunction_by_default()` |
| `Highlighter` + `SimpleHTMLFormatter("<mark>")` | `SnippetGenerator`(`set_max_num_chars(100)`、タグ差し替え) |
| `query*` / `*query*` | `RegexQuery`(件数が少ないので SQLite の `LIKE` に逃がす選択肢もある) |
| `StringField` の Term 削除 | `delete_term` |
| `writer.commit()`(保存のたび) | `IndexWriter::commit()` はバックグラウンド(1 秒ごと)。検索の直前に未コミット分があればその場でコミットするので、検索結果は保存直後でも揃う。`flush()` と終了時にもコミットする。読み側は `IndexReader::reload()` |

注意点:

- **インデックス形式に互換性はない。** 初回起動で `~/.peta-journal/index` が tantivy の形式でなければ中身を消して `SearchIndexRebuilder` 相当で再構築する。Lucene の `segments_*` ファイルの有無で判定できる。
- 分かち書きの結果は同じ IPADIC なのでほぼ一致するが、未知語と `Decompose` の penalty 既定値で差が出うる。E2E の検索シナリオで確認する。
- Lucene の `commit` が遅いときの警告(2 秒超で antivirus を疑うログ)は同じ意味で残す。

### 3.4 MCP(quarkus-mcp-server-http → rmcp)

rmcp の `#[tool_router]` / `#[tool]` 属性で、`McpItemTools` などの各メソッドを 1 対 1 で移す。ツール名は Java のメソッド名(snake_case)と一致させ、引数の `description` もそのまま写す。戻り値は Java 版と同じく JSON 文字列。

確認すること:Java 版のエンドポイントのパス(quarkus-mcp-server-http の既定は `/mcp`)と、利用側のクライアント設定にある URL。Rust 版は同じパスで axum のルートに `StreamableHttpService` を `nest` する。

### 3.5 ネイティブ連携

`FolderChooserService` は OS ごとのコマンド(`zenity` / `kdialog`、`osascript`、`powershell.exe` の `FolderBrowserDialog`)を子プロセスで起動して標準出力を読んでいるだけなので、`std::process::Command` で同じスクリプトを実行する。`AppLauncher` は open crate(内部で `xdg-open` / `open` / `cmd /c start`)。

### 3.6 `/api/sync/pending-deletes` の HTML

`SyncPendingDeletesResource` はフォーム付き HTML を文字列で組み立てて返し、`Origin` ヘッダを見て POST を受ける。テンプレートエンジンは要らないので、文字列組み立てをそのまま Rust に移す(`html_escape` crate でエスケープ)。

## 4. Rust 側の構成案

単一バイナリ、ワークスペース 2 crate。

```
journal-server-rust/
  Cargo.toml                 [workspace]
  crates/hmtid/              HMTID(生成・パース・比較)。npm 版と同じテストベクタ
  crates/server/
    src/main.rs              設定読み込み → DB 初期化 → 検索初期化 → Router 組み立て → 定期タスク起動
    src/config.rs            application.properties 相当。既定値 + 環境変数(PETAJOURNAL_DATA_DIR など)+ CLI 引数
    src/domain/              fractional_index.rs, hashtag.rs, id_generator.rs, item_body.rs, model/, repository/(trait)
    src/application/         item_service.rs, page_service.rs, journal_service.rs, config_service.rs,
                             combined_search.rs, flat_markdown_{serializer,splitter}.rs, tree_markdown.rs,
                             search_index_rebuilder.rs, extension_functions.rs, folder_chooser.rs
    src/infrastructure/
      persistence/           sqlite_{item,page}_repository.rs, property_repository.rs, database_initializer.rs
      search/                tantivy_search_service.rs
      export/                markdown_{export,import,reconcile}_service.rs, page_matcher.rs, item_merger.rs ...
      watch/                 markdown_folder_watcher.rs
      launcher.rs
    src/presentation/
      rest/                  pages.rs, items.rs, journals.rs, search.rs, config.rs, sync.rs, assets.rs,
                             functions.rs, launcher.rs, events.rs, health.rs, dto/
      mcp/                   tools.rs(rmcp)
```

設計上の決めごと:

- **状態は `Arc<AppState>` に集める。** Quarkus の `@ApplicationScoped` DI の代わり。`SqliteConnectionProvider` は `Mutex<rusqlite::Connection>`(Java 版も単一接続。WAL なので読み書きはこれで足りる)。
- **ブロッキング処理は `spawn_blocking` に逃がす。** SQLite、tantivy の commit、子プロセス起動。axum のハンドラを塞がない。
- **エラーは `thiserror` で型にし、`IntoResponse` で HTTP に写す。** Java 版の `IllegalArgumentExceptionMapper`(400)に対応するものを用意する。
- **ログは `tracing` + `tracing-subscriber`。** `quarkus.log.category` 相当は `RUST_LOG` で。
- **設定の優先順位**:CLI 引数 > 環境変数 > 既定値。E2E の `-Dquarkus.http.port=8187 -Dpetajournal.data.dir=...` を `--port 8187 --data-dir ...` に置き換え、`frontend/playwright.config.ts` を直す。

## 5. 進め方(フェーズ)

各フェーズの終わりで Java 版と同じ入出力になることを確認してから次へ進む。Java 版は最後まで残し、ポートを変えて並行起動できるようにしておく(比較に使う)。

### フェーズ 0: 土台(hmtid、設定、DB を開く)

1. `crates/hmtid` を書き、npm 版 / Java 版と同じ文字列・同じ順序になるテストを通す
2. `config.rs`、`database_initializer.rs`(スキーマ作成と `SchemaVersionCheck`、`html → ast` などの ALTER も含めて移す)
3. `GET /api/health` だけの axum サーバーを起動し、既存の `journal.db` を WAL で開けることを確認

### フェーズ 1: ページとアイテム(REST の中核)

1. `domain/`(`FractionalIndexer`、`HashtagExtractor`、`ItemBody`、model)と単体テストを移す
2. `SqlitePageRepository`、`SqliteItemRepository`、`PropertyRepository` を移す
3. `PageService`、`ItemService`、`JournalService` を移す。flatmarkdown crate をここで繋ぐ
4. `/api/pages`、`/api/items`、`/api/journals`、`/api/config` の REST を移す
5. `ItemsApiTest`、`PagesApiTest`、`JournalsApiTest`、`MarkdownApiTest` 相当を Rust で書き直して通す

このフェーズが終わると、検索と同期以外はフロントエンドが動く。

### フェーズ 2: SSE と Markdown フォルダ同期

1. `SseBroadcaster` → `broadcast` チャネル、`/api/events`
2. `infrastructure/export/` 一式(`MarkdownExportService`、`MarkdownImportService`、`MarkdownReconcileService`、`PageMatcher`、`ItemMerger`、`FileSyncStateRepository`、`ExportQueueRepository`、`PendingDeletePrompt`)
3. `MarkdownFolderWatcher`(notify + 2 秒デバウンス)
4. 定期タスク 3 本
5. `/api/sync/*`、`/api/assets`、`/assets`、`/api/functions`、`/api/launcher`
6. `FolderSyncTest` 相当と、`docs/folder-sync-journeys.md` のジャーニー 1〜(ページ作成・削除・リネーム・競合)を手で再現

ここは Java 版のロジックが一番込み入っている(ハッシュによる自己書き込み判定、競合バックアップ、ID の付け替え `rekey`)。**書き換えではなく翻訳**に徹し、条件分岐の順序を変えない。

### フェーズ 3: 検索

1. `TantivySearchService`(3.3 の構成)
2. `SearchIndexRebuilder`、起動時の Lucene 形式検出と再構築
3. `CombinedSearchService`、`/api/search/*`
4. `SearchApiTest`、`CombinedSearchServiceTest` 相当。日本語の検索語(活用形、カタカナ長音、全角半角)で Java 版と結果を突き合わせる

### フェーズ 4: MCP

1. rmcp で 26 ツールを移す(application 層を呼ぶだけなので薄い)
2. 既存の MCP クライアント設定で接続し、各ツールを 1 回ずつ叩く

### フェーズ 5: 切り替え

1. `frontend/playwright.config.ts` を Rust 版の起動コマンドに変える。E2E を全部通す
2. `BUILD.md`、`README.md` の起動手順を更新。`cargo build --release` と各 OS 向けバイナリ
3. CI(あれば)を Rust に変える
4. Java 版は 1 リリース分は残し、問題がなければ `journal-server` を削除して `journal-server-rust` を `journal-server` に改名

## 6. テスト方針

| Java 版 | Rust 版 |
|---|---|
| 単体テスト(`FractionalIndexerTest`、`ItemBodyTest`、`FlatMarkdownSerializerTest` など) | 同じケースを `#[test]` で移す。テーブル駆動にしやすい |
| `*ApiTest`(`IntegrationTestProfile` で一時ディレクトリにサーバーを 1 回起動) | 一時ディレクトリに `AppState` を組み、`Router` に対して `tower::ServiceExt::oneshot` か、実ポートで起動して reqwest。`config.json` に `markdownDir` を先に書いておく点も同じ(無いとフォルダ選択ダイアログが開く) |
| `FolderSyncTest` | 同上 + 実ファイルを書き換えて watcher を待つ |
| Playwright E2E(`frontend/e2e`) | **変更なしで通す。** これが「フロントエンドから見て同じ」ことの最終確認 |

加えて、移行中だけの**差分テスト**を用意すると安全になる。Java 版と Rust 版を別ポートで同じ `journal.db` のコピーに対して起動し、同じリクエスト列を両方に送って JSON を比較するスクリプト(ID や日時は正規化する)。

## 7. リスクと対策

| リスク | 対策 |
|---|---|
| HMTID の仕様の写し間違い(他 PC や既存ファイルと ID が噛み合わない) | フェーズ 0 で npm 版とテストベクタを共有する。既存の `journal.db` と Markdown ファイルの ID を全部パースできることを確認 |
| 同期ロジックの翻訳ミス(ファイルを壊す、重複ページを作る) | フェーズ 2 でジャーニー文書を手で再現。Dropbox フォルダを使う前に、ローカルのコピーで試す |
| 日本語検索の結果が微妙に変わる | 同じ IPADIC なので大きくは変わらない。差が出た語を集めて penalty と stop tags を調整 |
| tantivy の commit が Lucene より遅い / 速い | 同じ「2 秒超で警告」を残して観測する |
| rmcp の Streamable HTTP の細部(セッション、再接続)が quarkus 版と違う | フェーズ 4 で実クライアントで確認。SSE 版が必要なら rmcp の `SseServer` も足せる |
| Windows / macOS のビルドと配布 | lindera の `embed-ipadic` は辞書をバイナリに埋め込むのでランタイムのファイル配置は不要。各 OS で `cargo build --release` を CI に載せる |
| 初回ビルド時に IPADIC(約 50 MB)を Lindera.dev からダウンロードする | ネットワークのない環境では `LINDERA_BUILD_DICTIONARY_CACHE_DIR` にビルド済み辞書を置いておく。辞書のソースは mecab-ipadic 2.7.0 なので、npm の `mecab-ipadic-seed` などからも作れる |
| axum の multipart、SSE、CORS の設定ミス | フロントエンドの画像アップロードと SSE 再接続を E2E に含める |

## 8. 着手前に決めること

1. **ディレクトリ名**:`journal-server-rust` で並行開発し最後に改名するか、最初から `journal-server` を置き換えるか。並行開発を推奨(差分テストができる)。
2. **hmtid の置き場所**:このリポジトリ内の crate にするか、`flatmarkdown` と同じく別リポジトリ + crates.io にするか。
3. **MCP のパス**:現在の Java 版が公開しているパス(`/mcp` か `/mcp/sse` か)を確認して固定する。
4. **検索の再構築の案内**:初回起動で再構築が走ることをログに出すだけでよいか、`/api/sync/status` に出すか。
5. **E2E の起動コマンド**:`playwright.config.ts` が Java 版と Rust 版のどちらを起動するかを環境変数で切り替えられるようにしておくと、移行中に両方で E2E を回せる。
