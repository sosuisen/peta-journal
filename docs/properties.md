# プロパティの仕様

アイテムとページに付く「プロパティ」の仕様をまとめる。サーバーは `journal-server-rust`、フロントエンドは `frontend-rust` を前提にする。

## 1. プロパティ定義

キーごとに1つの定義を持つ。定義はSQLiteの `property_definitions` テーブルにある。

| 列 | 意味 |
|---|---|
| `key` | キー。英数字と `_` と `-` だけを使える |
| `value_type` | 型。`text` / `number` / `checkbox` / `date` / `datetime` / `list` / `tags` |
| `is_system` | システム管理キーなら1。ユーザーは定義を追加・変更・削除できない |
| `for_page` | ページに付けられるキーなら1 |

ユーザーが新しいキーを定義するときに選べる型は `text` / `number` / `checkbox` / `date` / `datetime` の5つである。`list` と `tags` は内部用で、`tag` だけが使う。

キーの型は最初に定義されたときに固定される。既存キーに別の型で値を送っても定義は変わらない。

### システム管理キー

| キー | 型 | ページで使えるか | 誰が書くか |
|---|---|---|---|
| `tag` | tags | 使える | アイテムではサーバーが本文の `#hashtag` から作る。ページではユーザーが編集する |
| `begin` | datetime | 使えない | サーバー。タスクの開始時刻 |
| `end` | datetime | 使えない | サーバー。タスクの終了時刻。あればタスクは完了扱い |
| `plan` | date | 使えない | フロントエンド。日次ジャーナルでタスク化したときの日付 |
| `due` | date | 使えない | タスクの期限。今はMCPから付けるだけで、読む処理はない |
| `cancelled` | date | 使えない | タスクを中止した日。今はMCPから付けるだけで、読む処理はない |
| `zoom` | number | 使えない | フロントエンド。Shift+ホイールの拡大率 |
| `id` | text | 使えない | ページファイルのfrontmatter |
| `created` | datetime | 使える | ページの作成日時。`pages.created_date` そのもので、ページのプロパティとして編集できる |
| `modified` | datetime | 使える | ページの更新日時。`pages.modified_date` そのもので、ページのプロパティとして編集できる |

システム管理キーはサーバー起動時に `migrate_definition_flags` が必ずフラグを立てる。

ユーザー定義キーは `for_page` が既定で1になる。

## 2. 値の保存

スカラー値は `properties` テーブルに入る。主キーは `(target_type, target_id, key)` で、1対象につき1キー1値である。`target_type` は `item` か `page` である。

| 型 | 保存先 | 読み出したときの文字列 |
|---|---|---|
| `number` | `value_number` | 整数なら `"2"`、小数なら `"1.5"`。数値でない入力は400エラー |
| `checkbox` | `value_checkbox` | `"true"` のときだけ1。読み出しは `"true"` / `"false"` |
| `text` / `date` / `datetime` | `value_text` | そのまま。日付の形式チェックはない |

リスト値は `property_list_items` テーブルに1値1行で入る。`sort_order` で順序を保つ。

## 3. REST API

### 定義

- `GET /api/config/property-definitions`
  - `propertyDefinitions`: `{ key: 型 }` のマップ。古いクライアント向け。
  - `definitions`: `[{ key, valueType, system, page }]` の配列。
- `POST /api/config/property-definitions`
  - ボディは `{ key, valueType, page? }`。`page` は省略すると `true`。
  - システム管理キーと同名、定義済み、型が5種以外、キーに使えない文字がある、のどれかなら400。
  - 成功すると `GET` と同じ形で全定義を返す。

### アイテム

- `PUT /api/items/{id}/properties/{key}`: ボディは `{ valueType, value }`。`valueType` 省略時は `text`。未定義のキーは、その型でユーザー定義キーとして作られる。
- `DELETE /api/items/{id}/properties/{key}`

どちらも更新後のアイテムを `{ item }` で返す。

### ページ

- `PUT /api/pages/{id}/properties/{key}`: ボディは `{ valueType, value }`。
  - `created` と `modified` はRFC 3339の日時（例 `2026-01-02T03:04:05Z`）を受け取り、`pages` テーブルの日付を書き換える。`properties` テーブルには入らない。形式が違えば400。
  - `tag` はカンマ区切りの文字列を受け取り、リストとして保存する。
  - `for_page` が0のキー（`zoom` など）は400。
  - 未定義のキーは、その型でユーザー定義キーとして作られる。
- `DELETE /api/pages/{id}/properties/{key}`: `created` と `modified` は消せない（400）。

どちらも更新後のページを `{ page }` で返す。`page.properties` にスカラー値とリスト値が入る。ページ一覧を返すAPI（recent、children、backlinks）では `properties` は空である。

### ファイルへの書き出し

アイテムまたはページのプロパティだけを変えたときも、ページの `modified_date` を更新し、Markdownファイルを書き出す。これは `ItemService::set_property` / `delete_property` と `PageService::set_property` / `delete_property` が行う。MCPの `upsert_property` ツールも同じ経路を通る。

## 4. Markdownファイルでの表現

### アイテム

メタデータ行に `key="value"` として書く。`idx`、`collapsed`、`transclude` の後に、残りのキーをアルファベット順で並べる。リスト値はカンマ区切りで1つの値にする。値の `\` と `"` はエスケープする。

```
<!-- @@ <id> idx="..." plan="2026-03-01" tag="a,b" zoom="1.5" -->
```

### ページ

frontmatterに書く。`id` / `created` / `modified` の後にスカラー値を `key: value` で、リスト値をYAMLのリストで書く。

```
---
id: <id>
created: 2026-10-16T00:00:00Z
modified: 2026-10-16T00:00:00Z
owner: "me: now"
tag:
  - a
  - b
---
```

値が空、または `"` `\` `: ` ` #` を含む、または `:` で終わる、または端に空白がある、またはYAMLで意味を持つ記号で始まるときは `"` で囲む。`2026-01-02T03:04:05Z` のような日時はそのまま書く。`"` は `\"` にする。`outline_parser` はこの形式を読み戻せる。

## 5. フロントエンドのUI

### Settings画面

- 全定義を一覧にする。システム管理キーは読み取り専用で表示する。
- キーと型を入力してユーザー定義キーを追加できる。

### ページのプロパティ

- ページがプロパティを持つときは、タイトルと本文の間に常にプロパティUIを出す。
- ページがプロパティを持たないときは、「…」メニューの「プロパティの編集」でUIを出す。
- `created` と `modified` の行は常に先頭にある。すべてのページが持つので、この2行だけではUIは自動では開かない。「X」は付かない。
- `modified` はサーバーが保存のたびに動かすので、UIからは変更できない。表示だけである。
- `datetime` 型の値は現地時刻で分まで表示し、分まで入力する。サーバーへは秒を0にしたUTC（`YYYY-MM-DDTHH:MM:SSZ`）で送る。
- 「+」で足した行が `datetime` 型のときは、現在の日時が初期値として入っている。Enterでそのまま確定できる。
- 値は、変更して入力欄を離れたとき、Enterを押したとき、入力欄の外をクリックしてフォーカスが外れたときに確定する。足したばかりの行は、値が空のままならフォーカスが外れても確定しない。その行の「X」を押すと、確定せずに行が消える。
- 1プロパティ1行。「+」で行を足し、「X」で消す。
- キーは `for_page` が1の定義から選ぶ。

## 6. 今後の課題

### ページプロパティのファイルからの取り込み

今はDBからファイルへの一方向だけである。ユーザーがMarkdownファイルのfrontmatterを手で編集しても、`id` / `created` / `modified` 以外のキーはDBに取り込まれない。

取り込みを作るときの注意点を残しておく。

- `outline_parser::parse_frontmatter` はすでに全キーをマップに読んでいる。リストはカンマ区切りの1文字列になる。
- `import_service` の `PageMatcher` が `id` と `modified` を使う。残りのキーをここで `PropertyRepository` に書けばよい。
- キーの型は定義から引く。未定義のキーをどう扱うか（`text` として定義するか、無視するか）を決める必要がある。
- `for_page` が0のキーがfrontmatterにあった場合は無視する。
- 取り込みで `modified_date` を進めると再エクスポートが走り、ファイルが書き換わる。衝突判定（DBが新しいときはバックアップ）との整合を確認する。
