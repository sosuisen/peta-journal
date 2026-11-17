# ADR 058: Core Behaviour Is Tested in journal-core

## Status

Accepted

## Context

ADR 029で、ジャーナルサーバーの中身をライブラリ`journal-core`として切り出した。
AndroidとiOSのアプリが、PCと同じ中身を組み込んで使うためである。
切り出しの間は、`journal-server`の結合テストが、動きが変わらないことを守った。
切り出しのあとも、そのテストは`journal-server`に残っている。

### テストの数

2026-11-18に数えた。
`journal-core`のテストは163個で、すべて`src`の中の単体テストである。
`journal-server`のテストは446個である。
そのうち437個は`tests/`の結合テストで、`src`の中の単体テストは9個である。
結合テストの多くはサーバーを立て、HTTPで確かめる。
2台の同期のテスト（`tests/it/item_merge.rs`、`atlas_sync.rs`、`definitions_sync.rs`、`page_properties.rs`）はHTTPを使わない。
ただし、サーバーの`AppState::build`で中身を組み立てるので、`journal-server`に置いている。

### 困ること

コードの大半は`journal-core`にあるのに、テストの大半は`journal-server`にある。
`journal-core`の`application/item_service.rs`、`split.rs`、`paste.rs`には、単体テストが1つもない。
貼り付け、分割、結合、バッチ、スタッシュ、移動、トランスクルージョン、項目単位の同期の規則は、`journal-server`のテストだけが確かめている。

その結果、次のことが起きている。

- AndroidとiOSが使う道を、テストが通らない。
  サーバーは`journal-core`をdefault featuresでビルドするので、拡張のない道（`extension_runtime/none.rs`）を通らない。
  サーバーの外で組み立てた`JournalCore`（`CoreHost`がSSEの`SseBroadcaster`でない場合）も通らない。
  embed版だけが呼ぶ関数（`PageService::find_doc`、`export.waits`、`export.process_whole_queue`）も、サーバーのテストからは呼ばれない。
- コアの振る舞いを1つ確かめるのに、サーバーを立てる。
  サーバーを1つのバイナリで共有するため、テストは名前をずらし（`unique_name`）、設定を変えるテストはロック（`TAGS_DIR`、`FOLDER_SCAN`）で守っている。
- HTTPの層の誤りと、コアの誤りが、同じテストで落ちる。
- `journal-core`のミュータントを確かめるには、ワークスペースの全体のテストを回す（`.cargo/mutants.toml`の`test_workspace = true`）。

### 調べたこと

446個のテストを1つずつ読み、どこで確かめるべきかを分けた。
結果は付録にある。
分けた区分は次の4つである。

- MOVE：コアの振る舞いを確かめている。HTTPなしで、`Journal`か`JournalCore`を直接呼んで書ける。
- SPLIT：コアの規則と、HTTPの層の確認が、1つのテストに入っている。
- KEEP-SERVER：REST、MCP、画面の配信、フォルダーの監視、CLI、サーバーのコマンドそのものを確かめている。
- DELETE：実際の使い方で起きる場面がないか、ほかのテストと同じことを確かめている。

| 区分 | 件数 |
|---|---|
| MOVE | 337 |
| SPLIT | 10 |
| KEEP-SERVER | 74 |
| DELETE | 25 |

調べてわかった前提は、次のとおりである。

- `Journal::new(services, events: Arc<dyn JournalEvents>)`は、知らせの受け手を外から受け取る。
  記録するだけの受け手を渡せば、SSEなしで「各ページに1回ずつ知らせる」を確かめられる。
- ファイルの取り込み（`import.sync_file`）、書き出し（`export.export_page`、`export.process_export_queue`）、フォルダーの調べ（`reconcile.scan`）には、`Journal`のメソッドがない。
  `JournalCore`（`journal-core/src/assembly.rs`）の部品を直接呼ぶ。
  embed版のシェル（`client-shell-embedded`）も、同じように直接呼んでいる。
- 2台の同期のテストは、今も1つの一時フォルダーの下に端末ごとの`AppState::build`を作り、ファイルをコピーして届けている。
  DB、索引、設定、Markdownのフォルダーは、端末ごとに別である。
  ファイルの届く順は、テストが決めている。
- `journal-core`には、テストのために`JournalCore`を組み立てる部品がない。
  手本になるのは、`client-shell-embedded`の`lib.rs`（`JournalCore::build`を`CoreHost`の`EventQueue`と2つの`Quiet`で組み立てる）と、`tests.rs`の`App::new`である。

## Decision

コアの振る舞いは、`journal-core`のテストで確かめる。
`journal-server`には、REST、MCP、画面の配信、フォルダーの監視、CLI、サーバーのコマンドのテストだけを残す。
どのテストをどこへ移すかは、付録の表に従う。

### 置き場所と書き方

移したテストは、`journal-core/tests/`に置く。
`journal-core`の公開APIだけで書けるので、`src`の中には置かない。
テストは`Journal`のメソッドを呼ぶ。
`Journal`にメソッドがない取り込み、書き出し、フォルダーの調べは、`JournalCore`の部品を呼ぶ。
HTTPの状態番号を確かめていたところは、`AppError`の種類（`BadRequest`、`NotFound`など）を確かめる。

1つの規則だけを確かめるテストで、`Journal`を組み立てなくても書けるものは、その規則のモジュールの単体テストにする。
付録で「単体テストにする」と書いたものが、これに当たる。

### テストの部品

`journal-core/tests/common/`に、次の部品を作る。
最初のPRで部品だけを作り、移すテストはその後のPRで移す。

- テストごとに、一時フォルダーの上に`JournalCore`を1つ作る関数。
  data、index、config、markdownのフォルダーを作り、`markdownDir`を設定する。
  Markdownのフォルダーのない版も作れる（`definitions_sync.rs`の`a_device_without_a_markdown_folder_still_takes_a_definition`のため）。
  `CoreSettings`の待ち時間（コピーの待ちと上書きの待ち）を変えられる。
  `start_up()`まで呼ぶ。
  `JournalCore`の全体を返す。
  DBに直接書くテスト（付録の[db]）が、`JournalCore`の`db`を使うためである。
- 知らせを記録する`EventSink`と`JournalEvents`。
  ページの変更（どのページか、だれが変えたか）、プロパティの定義の変更、拡張の質問を記録する。
- 何もしない`MissingFilesPrompt`と`FolderWatch`。
  呼ばれた回数を数える。
- 2台の端末。
  1つの一時フォルダーの下に、上の`JournalCore`を2つ作る。
  今の`journal-server`の`Device`の道具（`export`、`deliver`、`receive`、`sync`、`file`、`conflicts`、`missing`）を、そのまま移す。
  1つのMarkdownのフォルダーを2台で共有はしない。
  共有すると、ファイルの届く順をテストが決められないためである。
- 拡張を入れる道具と、拡張の質問に答えるスレッド。
  `extensions`と`auto_template`のテストのためである。

テストごとに`JournalCore`を作るので、名前をずらす工夫と、設定を守るロックは要らなくなる。
tantivyの索引を開く時間がかかるので、最初のPRで、テスト1つあたりの時間を測る。

### サーバーに足すテスト

移したあと、項目のルートの多く（字下げ、移動、スタッシュ、貼り付け、分割、バッチ、結合、トランスクルージョン、タスク、プロパティ、削除）は、HTTPで確かめるテストがなくなる。
そこで、`journal-server`に、ルートの表を回すテストを1つ足す。
各ルートに正しい要求を1回送り、状態番号と答えの包み（`item`、`items`、`created`など）を確かめる。
このテストは、項目のテストを移すPRより前に足す。
ルートが、テストのない状態を経ないためである。

### features

`journal-core`のテストは、default featuresと`--no-default-features`の両方で回す。
`--no-default-features`は、AndroidとiOSがビルドする形である。
拡張を使うテスト（`extensions`と`auto_template`から移すもの）には、`#[cfg(feature = "extensions")]`を付ける。
CIにも、`cargo test -p journal-core --no-default-features`を足す。

### 移す順

1. `journal-core/tests/common/`の部品を作る。
2. `journal-server`に、ルートの表を回すテストを足す。
3. ファイルか節ごとに、1つのPRで移す。
   同じPRで、`journal-server`の元のテストを消す。
   付録のDELETEのテストも、そのPRで消す。
   SPLITのテストは、コアの部分を移し、サーバーに残す部分を書き直す。
4. すべて移したら、`.cargo/mutants.toml`の`test_workspace`を見直す。

テストを消すと、それだけが通っていたコードが残ることがある。
消すPRでは、カバレッジで通っていない行を見て、コードを消すかテストを残すかを決める。

## Alternatives

### Alternative 1: テストを`journal-server`に残す

今のまま、サーバーの結合テストでコアを確かめる。
移す手間はない。
しかし、AndroidとiOSが使う道をテストが通らないままになる。
コアの1つの振る舞いを確かめるのにサーバーを立てることも、ロックで守ることも残る。
そのため選ばなかった。

### Alternative 2: サービスごとの単体テストにする

移すテストを、`item_service.rs`や`split.rs`の中の`#[cfg(test)]`に置く。
規則の近くにテストが置かれる。
しかし、移すテストの多くは、DB、索引、知らせ、トランスクルージョンをまたぐ場面である。
サービスを1つだけ組み立てても書けない。
アプリが呼ぶ入口は`Journal`なので、そこを通るテストのほうが、実際の使い方に近い。
そのため、1つの規則だけを確かめるものを除いて、選ばなかった。

### Alternative 3: `client-shell-embedded`のテストとして書く

AndroidとiOSは、`client-shell-embedded`を通してコアを使う。
そこでテストすれば、アプリと同じ道を通る。
しかし、クライアントの層（`client-core`の画面とイベント）まで一緒に動き、コアの誤りとクライアントの誤りが同じテストで落ちる。
今のHTTPのテストと同じ問題が、別の層で起きる。
そのため選ばなかった。

### Alternative 4: `JournalCore`を1つのテストバイナリで共有する

今のサーバーのテストのように、`JournalCore`を1つだけ作り、すべてのテストで使う。
tantivyの索引を開くのが1回で済み、速い。
しかし、名前をずらす工夫と、設定を変えるテストのロックが、そのまま残る。
テストが互いの作ったページに影響されるので、「最近のページ」のように、相対的な順しか確かめられないテストもある。
そのため選ばなかった。
ただし、最初のPRで測った時間が長すぎれば、見直す。

## Confidence Level

方向（コアのテストを`journal-core`に置き、サーバーには層のテストだけを残す）には、確信が高い。
2026-11-13に開発者が決めた方針で、446個を読んだ結果も、その方針で分けられた。

テストの部品の形には、確信が中くらいである。
次のときに見直す。

- テストごとに`JournalCore`を作ると、テストの時間が大きく延びたとき。
  Alternative 4か、索引だけを共有する形を考える。
- `Journal`に、取り込み、書き出し、フォルダーの調べのメソッドができたとき。
  テストは`JournalCore`の部品でなく、`Journal`を呼ぶように書き直す。
- 付録の分け方が、移すときに読み直して違うとわかったとき。
  そのテストの行を直す。

## Consequences

### 良い結果

- AndroidとiOSが使う道（`--no-default-features`、サーバーの外で組み立てた`JournalCore`、embed版だけが呼ぶ関数）を、テストが通る。
- コアの振る舞いを確かめるのに、サーバーを立てない。
  テストごとに`JournalCore`を作るので、名前をずらす工夫とロックが要らない。
- コアの誤りと、HTTPの層の誤りが、別のテストで落ちる。
- `journal-core`のミュータントを、`journal-core`のテストだけで確かめられる見込みがある。
- テストが25個減る。

### 悪い結果

- 336個のテストを書き直す手間がかかる。
  多くは道具の置き換えで済むが、PRの数は多くなる。
- HTTPの状態番号の確認は、ルートの表を回すテストと、SPLITで残す数個のテストだけになる。
  エラーの種類から状態番号への変換は、ルートごとには確かめない。
- 取り込み、書き出し、フォルダーの調べのテストは、`Journal`でなく`JournalCore`の部品に頼る。
  部品の名前や形を変えると、テストも変わる。

### 中立的な結果

- 2台の同期のテストには、日付をずらすための`sleep(5ms)`が残る。
  コアがシステムの時計を直接読むためである。
  時計を差し替えられるようにするかは、`docs/plans.md`の「journal-core」の節で、移し終えてから決める。
- `journal-core`のテストが、サーバーの結合テストに頼らなくなる。
  移し終えたら、`journal-core`をリポジトリの直下に出すかを決める（ADR 039のAlternative 3）。
- 名前どおりのことを確かめていないテストが2つある（付録の`a_week_54_is_created_as_the_us_numbering_reaches_it_in_a_leap_year_starting_on_saturday`と`an_item_property_change_alone_rewrites_the_page_file`）。
  どちらもDELETEにした。
  米国式の週番号と、プロパティの変更だけで書き出しの列に入ることを確かめるなら、移すときに書き直す。
- バッチで存在しない項目を更新すると400、消すと404が返る（`api.rs`の`a_batch_that_names_an_item_that_is_gone_is_refused_and_writes_nothing`）。
  移すテストは、この不揃いをそのまま`AppError`の種類で確かめる。
  そろえるかは、別に決める。
- 呼ぶクライアントのないRESTのルート（`/api/items/parse`、`/api/items/import`、`/api/items/export-page`、`/api/items/export-subtree`、`/api/sync/status`、`/api/search/tags`、`/api/sync/scan`）は、消さない。
  ADR 060で、RESTとMCPは同じ操作を持つと決めているためである。
  これらのルートは、ルートの表を回すテストで確かめる。

## 付録：テストの分け方

2026-11-18の`main`の行番号である。
MOVEのテストは数が多いので、節ごとにまとめ、呼ぶメソッドと注意だけを書く。
SPLIT、KEEP-SERVER、DELETEのテストは、1つずつ書く。
表に出てこない`journal-server`のテストは、すべてMOVEである。

印の意味は次のとおりである。

- [ev]：知らせを数える。記録する`EventSink`か`JournalEvents`を使う。
- [db]：DBに直接書く。`JournalCore`の`db`を使う。
- [S]：`JournalCore`の取り込みか書き出しを呼ぶ。
- [R]：`JournalCore`の`reconcile.scan`を呼ぶ。

### `tests/it/api.rs`（242個）

#### MOVE

| 節 | 呼ぶもの | 注意 |
|---|---|---|
| Items: creating | `create_item`、`move_items`、`get_page`、`set_page_property` | 65、86、103は[db]。219（`setting_a_property_counts_as_a_change_of_the_page`）はページのテストなので、Pagesの節へ移す |
| Items: updating | `update_item`、`set_item_property`、`add_property_definition` | 207は、236の確認（ASTと型）を足して受ける |
| Items: levels | `shift_levels` | 409は[ev] |
| Items: moving | `move_items`、`set_transclusion` | 586は[ev] |
| Items: the stash | `stash` | 663は[ev] |
| Items: tasks | `cycle_task`、`get_or_create_journal`、`items_by_types` | |
| Items: transclusion | `set_transclusion`、`remove_transclusion`、`transcluded_items`、`delete_items` | |
| Items: merge | `merge_items` | |
| Items: paste | `paste` | 1496、1541、1626は[ev] |
| Items: split | `split` | 1735、2186、2201、2326は[ev] |
| Items: batch | `apply_batch` | 2434は[db]。2650、2678は[ev]。2461は、2401の確認（`created`が空）を足して受ける |
| Items: unclosed blocks | `update_item`、`create_item`、`apply_batch` | |
| Items: group delete | `delete_items`、`delete_item` | 2822は、2896の確認（`created`が空）を足して受ける |
| Pages | `get_or_create_doc`、`get_page`、`update_page`、`demote_page`、`recent_pages`、`backlinks` | |
| Journals | `get_or_create_journal`、`move_items_to_journal`、`most_recent_journal_before`、`most_recent_weekly_journal_before` | 3571は[ev] |
| Search | `search_page_names`、`search_combined`、`search_by_tag` | 3755（`a_hashtag_in_the_body_is_found_as_a_tag`）は`search_by_tag`を呼ぶ |
| Markdown conversions | 単体テスト | 3817（`flat_markdown_is_split_into_items_at_block_boundaries`）の「空行で区切ったリストは1項目のまま」を`flat_markdown_splitter`の単体テストにする。3917（`a_subtree_exports_like_its_items`）の「兄弟は入らない」を`export_subtree_markdown`のテストにする |
| Folder sync | [S]、[R]、`update_page`、`delete_page` | 4047は[R]と[ev]（取り込みの知らせ） |
| Properties | `add_property_definition`、`delete_property_definition`、`set_item_property`、`set_page_property`、`delete_page_property`、`child_pages`、[S]、[R] | 4386は、4363の`_list`の場合を足して受ける |
| the atlas page of a directory | `update_page`、`set_item_property`、`add_atlas_link`、`group_atlas_cards`、`add_atlas_callout` | |
| Assets | 単体テスト | 5702（`an_upload_without_a_plain_extension_is_saved_under_the_hmtid_alone`）は`asset_name::saved_name`の単体テストにする |

#### SPLIT

| 行 | テスト | コアへ | サーバーに残す |
|---|---|---|---|
| 832 | `items_listed_by_type_can_leave_out_those_with_a_property` | `items_by_types`で`end`のない項目 | `withoutProperty`のクエリ（todoが使う） |
| 3003 | `get_page_returns_its_items_with_only_the_fields_asked_for` | `with_fields`と、トランスクルージョンの木も絞ること | `?fields=`の読み取りとJSONのキー名 |
| 3293 | `renaming_to_the_name_of_another_page_is_refused_unless_it_is_overwritten` | 名前の衝突と上書き | 409 |
| 3339 | `a_deleted_page_is_gone` | `delete_page`のあと`get_page`が見つからない | 204と404 |
| 3851 | `exported_items_come_back_through_import_and_parse_unchanged` | `export_tree_markdown`の木の形とメタデータ | `/api/items/export`の答え（ASTを返さない） |
| 4242 | `health_is_ok_and_the_config_is_read_and_updated` | `config`、`update_config`の部分更新、`property_definitions`。4272の`homeLink`が変わらない確認を足す | `/api/health` |
| 5671 | `an_upload_is_saved_under_an_hmtid_with_the_extension_of_its_name` | `asset_name::saved_name`の単体テスト（拡張子を小文字にする） | multipart、答えの`fileName`と`originalName`、ファイルへの保存 |

#### KEEP-SERVER

| 行 | テスト | 確かめているもの |
|---|---|---|
| 878 | `items_listed_by_type_have_only_the_fields_asked_for` | `GET /api/items`の`types`と`fields`のクエリ（todoが使う） |
| 1528 | `the_clients_hear_which_client_changed_the_page` | `X-Client-Id`のヘッダーが、SSEの`clientId`に載ること |
| 3989 | `the_status_names_the_folder_and_says_it_is_there` | `/api/sync/status`のハンドラーが、サーバーの設定から答えを作ること |
| 5687 | `two_uploads_with_one_name_are_saved_as_two_files` | ハンドラーが、アップロードごとにIDを作ってファイルを書くこと |
| 5716 | `an_image_just_under_the_limit_is_saved_as_it_is` | `DefaultBodyLimit`（50MB）の内側 |
| 5732 | `an_image_over_the_limit_is_refused_and_not_saved` | `DefaultBodyLimit`の外側 |

#### DELETE

| 行 | テスト | 理由 |
|---|---|---|
| 192 | `an_item_made_without_a_body_has_an_empty_body` | 本文を省いて作るクライアントがない。client-coreは必ず送り、MCPは空の文字を入れる |
| 236 | `the_body_is_parsed_into_an_ast_and_types` | 207と同じ場面。207に確認を足す |
| 256 | `one_item_is_read_with_only_the_fields_asked_for` | `GET /api/items/{id}`に`fields`を付けるクライアントがない。`with_fields`は878と3003が確かめる（2026-11-18に戻した。ADR 060により、リポジトリの中に呼ぶクライアントがないことは、消す理由にならない） |
| 282 | `a_property_can_be_set_on_an_item` | 4466が同じ設定を準備に使う。4466に設定後の確認を足す |
| 377 | `a_deleted_item_is_gone_from_its_page` | 道具の`fill`が、すべてのテストで同じ削除をする。2937も確かめる |
| 782 | `tasks_can_be_listed_by_type` | 878が同じ一覧を確かめる |
| 800 | `items_listed_without_a_type_are_none` | 型なしで呼ぶクライアントがない。全件を返さないための守りなら、残す（2026-11-18に戻した。ADR 060により、リポジトリの中に呼ぶクライアントがないことは、消す理由にならない） |
| 812 | `an_item_without_a_page_id_is_refused` | `pageId`を省く呼び出しがない |
| 824 | `a_deletion_without_item_ids_is_refused` | `itemIds`を省く呼び出しがない。空の配列は2849が確かめる |
| 1469 | `a_paste_in_an_unknown_mode_is_refused` | client-coreのmodeは決まった3つで、知らないmodeを送る道がない（2026-11-18に戻した。ADR 060により、リポジトリの中に呼ぶクライアントがないことは、消す理由にならない）。コアのテストとして戻した |
| 2401 | `a_batch_that_leaves_an_item_on_the_page_makes_no_item` | 2461のアウトラインの確認で足りる |
| 2896 | `a_group_delete_that_leaves_an_item_on_the_page_makes_no_item` | 2822のアウトラインの確認で足りる |
| 3618 | `a_week_54_is_created_as_the_us_numbering_reaches_it_in_a_leap_year_starting_on_saturday` | テストのサーバーは`weekNumbering: "iso"`なので、米国式の道を通らない。名前の読み取りは`journal_name`の単体テストが確かめる |
| 3665 | `an_item_moves_to_another_day` | 汎用の移動で日へ動かしており、日への移動の規則と逆の振る舞いを固定している。日への移動は3459から3571のテストが確かめる |
| 3832 | `metadata_lines_carry_the_level_of_each_item` | `flat_markdown_splitter`の`splits_items_with_metadata`が同じ規則を確かめる |
| 3896 | `a_whole_page_exports_like_its_items` | MCPの`items_are_exported_as_tree_markdown_and_imported_back_as_flat_markdown`が同じことを確かめる |
| 4272 | `the_home_link_is_kept_as_the_target_of_a_link` | 正規化は`config_service`の単体テストが確かめる。部分更新の確認は4242へ移す |
| 4363 | `a_list_key_is_defined_only_with_a_name_that_ends_with_list` | 4386と同じ検査を通る。4386に`_list`の場合を足す |
| 4489 | `an_item_property_change_alone_rewrites_the_page_file` | 待つ道具が毎回自分で書き出すので、名前の場面を確かめていない。メタデータ行の形は`flat_markdown_serializer`の単体テストが確かめる |
| 5268 | `a_bounds_property_holds_four_numbers` | `bounds`の単体テストが同じ入力で確かめる |
| 5585 | `an_item_written_on_an_atlas_page_goes_when_the_atlas_is_written` | 規則は`atlas`の単体テストが、書き直しのきっかけは5357が確かめる |
| 5597 | `a_value_is_checked_by_the_type_of_its_definition_not_the_type_sent` | 5390の最後の2つの確認が同じ規則を確かめる |

### `tests/it/`のほかのファイル（155個）

#### MOVE

| ファイル | 呼ぶもの | 注意 |
|---|---|---|
| `item_merge.rs`（52） | 2台の端末、[S]、`create_item`、`update_item`、`delete_item`、`move_items`、`set_item_property`、`set_page_property`、`set_transclusion`、`cycle_task` | 1020は[R] |
| `atlas_sync.rs`（15） | 2台の端末、[S]、`update_page`、`delete_page`、`set_item_property` | 449と495は待ち時間0秒の`CoreSettings`。552と569は[R]と、数を数える`MissingFilesPrompt` |
| `definitions_sync.rs`（28） | 2台の端末、[S]、`add_property_definition`、`delete_property_definition` | 93はMarkdownのフォルダーのない版。321は[R]。641、654、674、699は[ev]。712と746は8つのスレッドから定義する |
| `page_properties.rs`（11） | 2台の端末、[S]、`set_page_property`、`delete_page_property` | 212は、229のB側の確認を足して受ける |
| `page_files.rs`（2） | `export.waits`、`export.process_whole_queue` | 90と108。embed版だけが呼ぶ関数である |
| `find_doc.rs`（4） | `PageService::find_doc` | embed版だけが呼ぶ。`client-shell-embedded`のテストと重なる |
| `property_repository.rs`（1） | `PropertyRepository::write_scalar_in` | 一時のDBで書ける単体テストにする |

#### KEEP-SERVER

| ファイル | 件数 | 確かめているもの |
|---|---|---|
| `asset_names.rs` | 12 | サーバーのコマンド`asset_names`の`plan`、`apply`、`run` |
| `frontend.rs` | 10 | 静的ファイルの配信、SPAのfallback、ルートの優先、`Cache-Control` |
| `watch.rs` | 9 | `MarkdownFolderWatcher`のdebounce、再試行、設定の変更、`settings/`の監視 |
| `page_files.rs` | 3 | 45、59、73。サーバーのコマンド`page_files`。45は、DELETEとして73に合わせる予定だったが、73は止めたサーバーで動き、項目が変わらないことを確かめられないので残す（2026-11-18に直した） |
| `client_id.rs` | 5 | RESTのmiddlewareとCORS |

#### DELETE

| ファイル | 行 | テスト | 理由 |
|---|---|---|---|
| `page_properties.rs` | 117 | `a_key_with_no_definition_is_defined_with_the_type_its_name_gives` | `definitions_sync.rs`の408と、`page_properties.rs`の49が同じことを確かめる |
| `page_properties.rs` | 229 | `a_key_that_ends_with_list_is_taken_in_as_a_list_on_a_device_without_its_definition` | 212の最初の手順と同じ。212にB側の確認を足す |
| `asset_names.rs` | 222 | `apply::the_file_moves_to_its_new_name_with_its_content` | コマンドの361と381が、ファイルの名前が変わることを確かめる |

### `tests/mcp/main.rs`（23個）

| 行 | テスト | 区分 | 中身 |
|---|---|---|---|
| 63 | `the_tools_of_the_java_server_are_listed` | KEEP-SERVER | 道具の一覧 |
| 105 | `a_bad_id_is_refused_as_invalid_params` | KEEP-SERVER | エラーの対応づけ |
| 117 | `an_item_is_created_read_updated_and_deleted` | KEEP-SERVER | level -1、collapsedを文字で受けること、fields |
| 166 | `an_update_to_a_body_with_an_unclosed_block_is_refused` | DELETE | `api.rs`の2771が規則を、105と117がMCPのエラーを確かめる |
| 184 | `deleting_the_last_item_of_a_page_leaves_one_empty_item` | DELETE | `api.rs`の2950が規則を確かめる |
| 197 | `items_move_between_pages_with_their_levels_shifted_or_kept` | KEEP-SERVER | `levelDelta`の受け方 |
| 239 | `a_task_is_cycled_and_listed_by_its_type` | KEEP-SERVER | `cycle_task`と`get_items_by_type`の境界 |
| 259 | `a_transclusion_mirrors_the_source_until_it_is_removed` | KEEP-SERVER | トランスクルージョンの道具の境界 |
| 282 | `a_property_is_set_on_an_item` | KEEP-SERVER | `upsert_property`の境界 |
| 303 | `items_are_exported_as_tree_markdown_and_imported_back_as_flat_markdown` | KEEP-SERVER | 答えにASTがないこと |
| 351 | `the_clients_hear_of_an_item_created_over_mcp` | DELETE | 368の一覧に`create_item`を足す |
| 368 | `the_clients_hear_of_each_change_of_an_item_over_mcp` | KEEP-SERVER | `ChangedBy::Mcp`が知らせに載ること。`create_item`、`delete_item`、`move_items`を一覧に足す |
| 417 | `the_clients_hear_of_the_pages_that_transclude_an_item_updated_over_mcp` | MOVE | `update_item`と[ev] |
| 434 | `the_clients_hear_of_the_pages_that_showed_an_item_deleted_over_mcp` | SPLIT | どのページに知らせるかを`delete_item`と[ev]でコアへ。MCPであることは368で確かめる |
| 450 | `the_clients_hear_of_both_pages_of_a_move_over_mcp` | SPLIT | 両方のページへの知らせを`move_items`と[ev]でコアへ。MCPであることは368で確かめる |
| 473 | `a_doc_page_is_created_found_again_renamed_and_deleted` | KEEP-SERVER | ページの道具の境界 |
| 513 | `the_clients_hear_of_a_page_renamed_or_deleted_over_mcp` | KEEP-SERVER | `ChangedBy::Mcp` |
| 539 | `the_most_recently_touched_page_comes_first_among_recent_pages` | KEEP-SERVER | `maxResults`の0と、`type`の空の文字の受け方 |
| 576 | `a_wiki_link_to_a_page_shows_up_in_its_backlinks` | KEEP-SERVER | `get_backlinks`の境界 |
| 602 | `page_names_and_item_text_are_searched` | KEEP-SERVER | 検索の道具の境界 |
| 648 | `a_search_gives_at_most_max_results_and_the_default_number_for_zero` | KEEP-SERVER | `maxResults`の受け方 |
| 683 | `the_config_is_read_and_updated` | KEEP-SERVER | 設定の道具の境界 |
| 735 | `a_journal_is_created_by_name_and_the_most_recent_day_before_a_date_is_found` | KEEP-SERVER | ジャーナルの道具の境界とエラー |

### `tests/extensions/main.rs`（13個）と`tests/auto_template/main.rs`（4個）

移すテストには、`#[cfg(feature = "extensions")]`を付ける。

| ファイル | 行 | テスト | 区分 | 中身 |
|---|---|---|---|---|
| `extensions` | 121、152、171、184、198、218、234、278、307、319、347 | （11個） | MOVE | `get_or_create_doc`、`get_or_create_journal`、`set_page_property`、`move_items_to_journal`、`functions.list_functions`。234は[ev]。278は[S] |
| `extensions` | 247 | `a_tag_extension_asks_the_clients_and_writes_its_items_after_a_yes` | SPLIT | 質問、答え、項目の流れはコアへ（記録する`EventSink`と`ConfirmService`への答え）。`POST /api/confirm/{id}`の200をサーバーに残す |
| `extensions` | 268 | `an_answer_to_a_question_that_no_one_waits_for_is_not_found` | DELETE | `confirm_service`の単体テストが同じ規則を確かめる（2026-11-18に戻した。HTTPの404を通るテストがなくなったためである） |
| `auto_template` | 73、91、133、150 | （4個） | MOVE | `get_or_create_journal`、`get_or_create_doc`、`set_page_property`。本物の`main.js`を使う。133と150は質問に答えるスレッドを使う |

### `journal-server/src`の単体テスト（9個）

すべてKEEP-SERVERである。

| ファイル | 件数 | 確かめているもの |
|---|---|---|
| `src/config.rs` | 4 | CLIの`--frontend-dir` |
| `src/presentation/client_id.rs` | 3 | `X-Client-Id`のヘッダーの読み取り |
| `src/application/configuration.rs` | 2 | サーバーの起動手順でフォルダーを作ること |
