# サーバーの機能

サーバー（journal-server）のテストを要約した、APIの機能の一覧である。APIに渡すものと返るものを書き、内部の仕組みは書かない。

2026-11-27時点のテストから作った。

## 起動の設定

- `--port`で待ち受けるポートを決める
- `--throwaway-dir`を渡すと、使い捨てのサーバーになる
  - データ（`data/`、DBは`data/journal.db`）、索引（`index/`）、設定（`config/`）、Markdownフォルダー（`markdown/`）が、そのフォルダーの下に決まる
  - `--data-dir`、`--db`、`--index-dir`、`--config-dir`と一緒には使えない
  - 設定ファイルが外のMarkdownフォルダーを指していても、自分のフォルダーの下の`markdown/`を使う。外のフォルダーには何も書かない
  - 外のMarkdownフォルダーへの設定の変更は受け付けず、答えも今のフォルダーのままである。自分のフォルダーの下への変更は受け付ける
  - この機械からの接続だけを受ける
- `--throwaway-dir`のないサーバーは、Markdownフォルダーを設定ファイルのとおりに読み、どこからでも接続を受ける
- この機械だけから受けるサーバーは、IPv6とIPv4のループバックだけで待ち受け、どちらからの接続も受ける
- `--frontend-dir`で、配信する画面のフォルダーを決める
  - 指定がなければ、実行ファイルの隣の`frontend/`を使う。それもなければ、画面は配信しない
  - 指定したフォルダーは、実行ファイルの隣の`frontend/`より優先される
- Markdownフォルダーには、`assets/`、`docs/`、`journals/`のフォルダーが作られる。フォルダーが設定されていなければ、何も作らない

## 手で実行するコマンド

- サーバーを止めて実行する`write-pages`は、すべてのページのMarkdownファイルを今のサーバーの形で書き直す
  - 項目の更新日時が書かれていないファイルに、それが書かれる。項目は変わらない
  - まだ取り込まれていないファイルの編集は、先に取り込まれ、失われない
  - 書いたファイルの数と、ページの数を伝える（`1 of 3 page files written`）
- サーバーを止めて実行する`asset-names`は、`assets/`のファイルをIDの名前に変え、リンクも書き換える。対応表のファイルを`--file`で渡す
  - `plan`は、対応表をファイルに書くだけで、ノートは変えない
    - 新しい名前は、ファイルの日時から作ったIDと、元の拡張子である。日時は更新日時と作成日時の古いほうで、古いファイルから並ぶ
    - すでにIDの名前を持つファイルは、対応表に入らない
  - `apply`は、対応表のとおりに、ファイルとリンクを変える
    - 項目の本文、ページのMarkdownファイル、検索の結果の本文のリンクが、新しい名前を指す。項目とページの更新日時は変わらない
    - 書き換えたあとでフォルダーを走査しても、取り込むものはない
    - まだ取り込まれていないファイルの編集は、先に取り込まれ、失われない
    - ファイルがすでに新しい名前になっていても、リンクは書き換わる
  - `revert`は、対応表を逆にたどって、ファイルとリンクを元に戻す

## APIの共通の決まり

- 変更を送るクライアントは、`X-Client-Id`ヘッダーでUUIDのIDを名乗る
  - IDを名乗らない変更は400で断られ、メッセージが`X-Client-Id`ヘッダーを挙げる。何も変わらない
  - データを変えないPOSTも、PUTも、DELETEも、IDがなければ断られる。UUIDでない値も断られる
  - GETは、IDがなくても答える
  - ファイルが消えたときの確認ページのフォームは、IDがなくても受け付ける
  - ほかのオリジンのページは、`X-Client-Id`ヘッダーを付けたリクエストを送ってよい
- APIでの変更は、`/api/events`のストリームで、変更したページのidとともに知らされる
  - 知らせの`source`は`app`で、`clientId`は変更を送ったクライアントのIDである
- 日付（`createdDate`、`modifiedDate`）は、既定ではミリ秒の数である
  - クエリ`dateFormat=iso`で、ページと項目の日付がUTCのISO 8601の文字列になる。ほかの値は変わらない。`dateFormat=millis`は既定と同じである
  - 変更のルートも、同じクエリを受ける
  - 利用者のプロパティは、日付の欄と同じ名前でも書き換えられない
  - 知らない形式は400で断られ、何も変わらない
  - イベントのストリームは、`dateFormat`があってもそのまま流れる
- `/api/`で始まり、ルートのないパスは404になる
- `GET /api/health`は、`status`が`ok`の答えを返す

## 項目

- 項目のルートは、正しい要求に200で答える。作成、取得、更新は`item`を、削除は204を返す
  - 移動（`move`）、バッチ（`batch`）、字下げ（`shift-level`）、貼り付け（`paste`）、スタッシュ（`stash`）は`items`を返す
  - 結合（`merge`）、トランスクルードの設定と解除（`transclude`）、タスクの状態を進める操作（`cycle-task`）、プロパティの設定と削除は`item`を返す
  - 分割（`split`）とまとめての削除（`items/delete`）は`created`を、トランスクルードの問い合わせ（`transcluded`）は`itemIds`を返す
- 型での一覧（`GET /api/items?types=task`）は、その型の項目を返す。型を指定しない一覧は空である
  - `withoutProperty`を渡すと、そのプロパティを持つ項目を除く。終えたタスクを`withoutProperty=end`で除ける
- 型での一覧、1件の取得、ページの取得は、`fields`で頼まれたフィールドだけを各項目に持つ。`items`、`item`、`page`の包みは変わらない
  - 名前は答えのJSONの名前である（例: `id`、`ast`、`body`、`level`、`parentItemId`）
  - 表示だけのクライアントは、本文のMarkdownを受け取らずに`ast`から描ける

## ページ

- ページのルートは、正しい要求に200で答える
  - `POST /api/pages/doc`はページを作るか見つけて、`page`と`items`を返す。`dirPath`に`フォルダー/`を渡すと、その下に作る
  - 最近のページ（`recent`）と子ページ（`children`）は`pages`を、名前での検索（`find`）は`page`を、バックリンク（`backlinks`）は`entries`を返す
  - ページのプロパティの設定と削除、改名、フォルダーのページを下ろす操作（`demote`）は`page`を返す
- 別のページと同じ名前への改名は409で断られる
- ページの削除は204で答え、消したページの取得は404になる

## アトラス

- アトラスのページ（`index_atlas`）は普通のページで、項目のルートで編集する。アトラス専用のルートはない（ADR 053）
- サーバーは、アトラスの本文を自分では書かない。ページを作っても、名前を変えても、消しても、カードは足されも消されもしない
- 古い形のアトラスは、起動のときと、古い形のファイルを取り込んだときに、普通のページの形に移る。移す前のファイルは、1回だけバックアップされる

## ジャーナル・検索・設定

- `POST /api/journals/{日付}`はジャーナルを作るか見つけて、`page`を返す。`/move`で項目をその日へ移す
- 日の前の直近のジャーナル（`before?date=`）と、週の前の直近のジャーナル（`weekly/before?week=`）は、`found`を返す
- ページ名の検索（`search/pages`）、本文の検索（`search/combined`）、タグの検索（`search/tags`）は、`results`を返す
- 設定の読み書きは`markdownDir`を含む答えを返す。プロパティ定義の一覧、追加、削除は`propertyDefinitions`を返す
- 拡張の問い合わせへの答え（`POST /api/confirm/{requestId}`）は、問い合わせが待っていれば200で答え、その答えが問い合わせに届く
  - 問い合わせはイベントのストリームで`requestId`とともに知らされる
  - 待っていない問い合わせへの答えは404になる

## Markdownとの変換

- 書き出し（`POST /api/items/export`）は、`treeMarkdown`と`flatMarkdown`だけを返し、ASTを持たない
- ページの書き出し（`export-page`）と部分木の書き出し（`export-subtree`）は、クエリでidを受ける
- 木のMarkdownの取り込み（`import`）は`flatMarkdown`を、Flat Markdownの解析（`parse`）は`items`を返す

## アセット

- `POST /api/assets/upload`で送ったファイルは、`assets/`にIDの名前で、中身のまま保存される
  - 答えには、保存した名前（`fileName`）と元の名前（`originalName`）がそのまま入る
  - 同じ名前のファイルを続けて送ると、別々の名前で保存され、どちらも残る
  - 50MBまでのファイルは保存される。50MBを超えるファイルは断られ、保存されない

## 画面の配信

- 画面のフォルダーのファイルは、その名前のパスで返る
- `/`と、ファイルのないパスには、`index.html`が返る
- `/api`、`/assets`、`/mcp`のルートは、同じパスにある画面のファイルより優先される
- `/apps/`のパスは拡張Webアプリとして扱われ、画面の`index.html`は返らない
- 画面のフォルダーがなければ、画面は配信されない
- `index.html`と、名前にハッシュのないファイルは、キャッシュされずに毎回確かめられる（`no-cache`）
  - ハッシュと同じ形を名前に含むページのパスに返る`index.html`も、キャッシュされない
- 名前にハッシュの付いたファイルは、長くキャッシュされる（`public, max-age=31536000, immutable`）

## 拡張Webアプリ

- 設定フォルダーの`apps/{名前}/`のファイルは、`/apps/{名前}/`の下のパスで返る。画面を配信していなくても返る
  - `/apps/{名前}/`には、そのアプリの`index.html`が返る
  - 拡張子のないパスでファイルがなければ、そのアプリの`index.html`が返る
  - 拡張子のあるパスでファイルがなければ、404になる
  - アプリのフォルダーがなければ、どのパスも404になる
- 末尾の`/`がない`/apps/{名前}`は、クエリを保ったまま`/apps/{名前}/`へリダイレクトされる
- 消したアプリのフォルダーは、起動し直さなくても配信されなくなる
- `..`をアプリの名前やパスにしても、`apps/`の外のファイルは返らない
- アプリのファイルは、名前にハッシュがあってもなくても、キャッシュされずに毎回確かめられる

## ファイルの同期

- 同期の状態（`GET /api/sync/status`）は、Markdownフォルダーの場所（`markdownRoot`）、それがあること（`rootAvailable`）、取り込みが有効なこと（`importEnabled`）を返す
- 走査（`POST /api/sync/scan`）は、取り込んだ数（`imported`）を返す
- フォルダーから消えたファイルは消えたものとして記録され、ページはユーザーが決めるまで残る
  - 消えたファイルは、確かめるページ（`/api/sync/pending-deletes`）に、そのページのidとともに並ぶ
- フォルダーを監視し、外で変えたファイルを、走査を待たずに取り込む
  - 外で編集したファイルはページに取り込まれ、検索で見つかる
  - フォルダーに置いた新しいファイルは、ページになる
  - シンボリックリンクを通して設定したフォルダーでも、リンクの先で編集したファイルが取り込まれる
  - 監視を始めたときに`docs/`も`journals/`もなかったフォルダーでも、あとでできた`docs/`のファイルが取り込まれる
  - ほかの端末が書いた`settings/properties.json`も取り込まれる。`settings/`がなければ作って見る
  - Markdownフォルダーの設定を変えると、すぐに新しいフォルダーを監視する
  - 空のファイルが先に作られても、ページは中身が書かれてから作られる
  - 読み込みに失敗したファイルは、新しい変更を待たずに取り込み直す。4回失敗すると諦め、走査に任せる。一度取り込めたファイルは、あとでまた失敗しても4回試す

## MCP

- `/mcp`で、Streamable HTTPのMCPサーバーとして答える
  - 接続したクライアントに、ツールを持つことと、`PetaJournal`で始まる説明を伝える
- RESTのルートとMCPのツールは、同じ操作を持つ。ツールはそれぞれ1つのルートの操作である
  - 画面やブラウザーのためだけのルートは、ツールを持たない。ヘルス、イベントのストリーム、名前でのページの検索、OSで開く操作、フォルダーの選択、問い合わせへの答え、消えたファイルの確認ページとそのフォームである
- ツールは、項目のASTを除いて、同じ操作のRESTのルートと同じJSONで答える
  - `fields`で`ast`を頼んでも、ASTは返らない。利用者の`ast`という名前のプロパティは消えない
  - 日付は既定でミリ秒の数で、引数`dateFormat`に`iso`を渡すとUTCのISO 8601の文字列になる。知らない形式は断られ、何も変わらない
- 項目を変えるツールの引数は、項目のIDと、RESTのルートが本文で受ける欄である（例: `update_item`は`itemId`が必須で、`body`、`collapsed`、`level`、`dateFormat`を受ける）
- 不正なIDは、invalid paramsとして断られる。ない項目やページは、`Item not found`、`Page not found`で断られる
- 項目のツール
  - `create_item`、`get_item`、`update_item`、`delete_item`で、項目を作り、読み、変え、消す。`update_item`で渡さない欄は変わらない。`delete_item`は`deleted: true`を返す
  - `move_items`は、項目をページの間で動かす。`levelDelta`で階層をずらし、なければそのままにする。空になったページには空の項目が1つ残る
  - `cycle_task`はタスクの状態を進め、`get_items_by_type`は型で一覧する。`get_items_by_type`は`fields`と`withoutProperty`を受ける
  - `set_transclusion`は元の項目の本文を映し、`remove_transclusion`で解くと本文は空になる。`get_transcluded_items`はトランスクルードされている項目を返す
  - `upsert_property`、`delete_property`で、項目のプロパティを設定し、消す
  - `paste_text`、`split_item`、`merge_items`、`shift_item_levels`、`apply_batch`、`delete_items`、`stash_item`で、エディターの操作をする。スタッシュは`# Stash`の見出しの下に項目を入れる
- Markdownのツール
  - `export_tree_markdown`、`export_subtree_markdown`、`export_page_markdown`は、木のMarkdownを返し、ASTを持たない
  - `import_tree_markdown`は、木のMarkdownをFlat Markdownにする。`parse_flat_markdown`は、Flat Markdownを項目に分ける
- ページのツール
  - `get_or_create_doc`は、ページを作るか見つける。作ったときは`created: true`で空の項目を1つ持ち、2回目は`created: false`で同じページを返す
  - `get_page_by_id`はページを返し、`fields`を受ける。`update_page`は改名し、`delete_page`は消して`deleted: true`を返す
  - `get_recent_pages`は、最後に触ったページを先に返す。`type`が空ならすべての型、`maxResults`が0なら10件である
  - `get_backlinks`は、ウィキリンクで指すページと項目を返す
  - `get_child_pages`、`upsert_page_property`、`delete_page_property`、`demote_page`で、子ページ、ページのプロパティ、フォルダーのページを下ろす操作を扱う
- ジャーナルのツール
  - `get_or_create_journal`は、名前でジャーナルを作るか見つける。ジャーナルの名前でなければ`Not a journal filename`で断られる
  - `get_most_recent_journal_before`は、日付の前の直近の日を返す。なければ`found: false`で、日付でなければ`Invalid date`で断られる
  - `get_most_recent_weekly_journal_before`は、週の前の直近の週を返す。`move_items_to_journal`は、項目をその日のジャーナルへ移す
- 検索のツール（`search_page_names`、`search_combined`、`search_by_tag`）は、`maxResults`の数まで返す。0なら既定の数である
- 設定のツール
  - `get_config`、`update_config`で、設定を読み書きする。答えには、設定から作るリンクの形（`wikilinkHref`、`hashtagHref`、`pagesBase`など）も入る
  - `get_property_definitions`、`add_property_definition`、`delete_property_definition`で、プロパティ定義を読み、足し、消す
- `get_sync_status`と`scan_folder`は、RESTの同期の状態と走査と同じJSONで答える
- `upload_asset`は、PCのファイルの絶対パスを受け、そのファイルを新しい名前でアセットに写す。元のファイルは残り、答えはRESTのアップロードと同じ形である
  - ファイルのないパスとフォルダーは`No file at`で、絶対でないパスは断られる
  - ちょうど50MBのファイルは写し、それを超えるファイルは断られる
- 項目を変えるツールとページを改名・削除するツールは、そのページの変更をイベントのストリームで知らせる。知らせの`source`は`mcp`である
