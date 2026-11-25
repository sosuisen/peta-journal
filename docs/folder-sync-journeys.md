# Markdown フォルダ同期のユーザージャーニー

PC A と PC B にそれぞれジャーナルサーバーと SQLite があり、`markdownDir` は同じ Dropbox フォルダを指している、という前提で、ページの作成と削除がどう伝わるかを 1 ステップずつ追う。

関係するコードは、`journal/journal-core/src/infrastructure/export/` にある。フォルダの監視は `journal/journal-server/src/infrastructure/watch.rs`、確認ページは `journal/journal-server/src/presentation/rest/sync.rs` と `infrastructure/pending_delete_prompt.rs` にある。

規則を決めた理由は [ADR 006](adr/adr-006-item-level-lww-for-folder-sync.md)、利用者向けのまとめは [folder-sync.md](folder-sync.md) にある。

## 共通の仕組み

| 部品 | 役割 |
|---|---|
| `export_queue` テーブル | DB が変わったページの ID を積む。5 秒ごとに `MarkdownExportService::process_export_queue()` が取り出して書き出す |
| `file_sync_state` テーブル | ファイルごとに「最後に読み書きした内容のハッシュ」「size / mtime」「前回の同期の日時」「`synced` か `missing` か」を持つ。自分が書いたファイルと外から変わったファイルを見分ける根拠 |
| `MarkdownFolderWatcher` | フォルダを `notify` クレートで監視。イベントはファイルごとに 2 秒デバウンスしてから `MarkdownImportService::sync_file()` に渡す |
| `MarkdownReconcileService` | 起動 10 秒後と 60 秒ごとのフルスキャン。Watcher の取りこぼしと、サーバー停止中の変化を拾う保険 |
| `MarkdownImportService` | ファイルを DB にマージする入口。ハッシュで自分の書き込みを見分け、競合のバックアップを作る。Dropbox の conflicted copy も取り込む |
| `PageMatcher` | ファイルがどのページのものかを決める(frontmatter の id → パス → 新規)。リネームやコピーもここ |
| `ItemMerger` | ファイルのアイテムを DB のアイテムに id でマージする。アイテムごとに `modified` の新しい方を採る |
| `PendingDeletePrompt` + `sync.rs` | 消えたファイルの確認ページ(`/api/sync/pending-deletes`)をブラウザで開く |

ファイルの中身(Flat Markdown v0.2):

```
---
id: <ページ HMTID>
created: <ISO 日時>
modified: <ISO 日時>
---

<!-- @ <アイテム HMTID> idx="a0" modified="<ISO 日時>" -->
本文

<!-- @@ <アイテム HMTID> idx="a1" collapsed="true" transclude="<ID>" modified="<ISO 日時>" plan="2026-09-27" -->
次のアイテム …
```

- 1アイテム = メタデータ行1行 + 本文(Markdownのブロック1つ)
- メタデータ行は本文の**上**に置く。`@`の数がlevel(`@` = 1、`@@` = 2)
- プロパティは`key="value"`。順は`idx`、`collapsed`、`transclude`、`modified`、残りはキーのアルファベット順。`idx`は常に書く。`collapsed="true"`は折りたたみ時だけ、`transclude`はトランスクルージョン時だけ
- `modified`はアイテムの更新日時。項目ごとの Last-Writer-Wins に使う(ADR 006)。アトラスのファイルには書かない
- frontmatter には、`id`、`created`、`modified` のあとに、ページのタグとプロパティが続く
- 本文が空のアイテムはメタデータ行だけになる

ページ ID・アイテム ID・`idx` がファイルに入っているので、両 PC は同じ ID と同じ `idx` を持つ。

---

## ジャーニー 1: PC A でページを作る

### ステップ 1: PC A でユーザーがページ「Ideas」を作る(0 秒)

`POST /api/pages/doc {title: "Ideas"}` → `PageService::get_or_create_doc()`。

PC A の DB:

| テーブル | 行 |
|---|---|
| `pages` | id = **A-page**、dir_path = `""`、name = `Ideas`、type = `doc` |
| `items` | id = **A-item**、page_id = A-page、body = `""`、idx = `a0` |
| `export_queue` | page_id = A-page |

ファイルはまだ無い。

### ステップ 2: PC A の export スケジューラが動く(最大 5 秒後)

`MarkdownExportService::process_export_queue()` → `export_page(A-page)`:

1. `markdown_root()` でフォルダの実在を確認。Dropbox が未マウントなどで無ければキューに戻して待つ(フォルダを作り直さない)
2. Markdown を組み立てる(上の形式)
3. パスは `docs/Ideas.md`(`{type}s/{dirPath}{name}.md`)。内容の SHA-256 を計算
4. `file_sync_state` を page_id で引き、**別パス**の行があればその古いファイルを消す(ページのリネーム・移動の後始末)。今回は無し
5. `file_sync_state` にこのパスの行があり、ハッシュが同じで、ファイルも存在すればスキップ。今回は行が無いので書く
   - ディスクのファイルが、記録した内容とも、これから書く内容とも違うことがある。別のPCから届いて、まだ取り込んでいないファイルである。このときは書かずに、キューへ戻す。取り込みが先に動き、マージするか、`.conflict`のバックアップを作る。
   - 5分たっても取り込まれなければ、ディスクのファイルを`.conflict`として残してから書く。
   - サイズと更新日時が記録と同じなら、中身は読まない。
6. `docs/.Ideas.md.tmp` に書いてから `docs/Ideas.md` へアトミックにリネーム。途中状態のファイルは誰にも見えない
7. `file_sync_state` に記録: `(docs/Ideas.md, A-page, ハッシュ, size, mtime, synced)`。前回の同期の日時には、ページとアイテムの `modified` の最大値を入れる

`status` は「Dropbox にアップロードされたか」ではなく「この PC のフォルダと DB が同じ内容か」を表す。

### ステップ 3: PC A 自身の Watcher が反応する(直後)

1. `.Ideas.md.tmp` のイベント → 先頭が `.` なので無視
2. `Ideas.md` の CREATE → `pending` マップ(インメモリ)に入れる
3. 最後のイベントから 2 秒経ったら `sync_file()`(500 ミリ秒ごとに確かめる)
4. 読んでハッシュを計算 → `file_sync_state` の値と**一致** → `UNCHANGED`。DB は触らない

ここで「自分で書いて自分で読み込む」ループが止まる。mtime ではなく内容ハッシュで判定しているので、Dropbox が後で mtime を書き換えても影響しない。

2 秒待つ理由: 1 回の保存で複数イベントが来る、書き途中を読まない、Dropbox の一括到着をまとめる。`--import-debounce-ms` で変更可。`pending` はインメモリだが、消えてもフルスキャンが `file_sync_state` との差で拾うので問題ない。

### ステップ 4: Dropbox が A から B へ運ぶ

PetaJournal は関与しない。Dropbox B は `.dropbox.cache/` に一時ファイルを作ってから `docs/Ideas.md` へ移動する。

PC B の Watcher: `.dropbox.cache` 配下は無視(`.` で始まるフォルダやファイルは無視する)、`docs/Ideas.md` の CREATE → `pending` へ。

### ステップ 5: PC B が取り込む(ページの照合)

`sync_file()`:

1. 無視対象でない、`docs/…/x.md` の形 → OK
2. ファイルを読み、ハッシュを計算
3. `file_sync_state` に行が無い → `import_content()` へ

`import_content()`:

1. 内容が空なら「まだ書き途中」として `ERROR` を返し、次回に回す
2. CRLF → LF に正規化してパース。フロントマターの `id: A-page` と `modified` を読む
3. **ID で照合**: B の `pages` に A-page → 無い
4. **パスで照合**: `(doc, "", Ideas)` → 無い
5. **新規ページ**を作る。**ID は A-page をそのまま使う**。created / modified はファイルの値

### ステップ 6: PC B が取り込む(アイテムのマージと仕上げ)

`ItemMerger::merge()`。このページは B ではまだファイルを持ったことが無いので、ファイルの中身をそのまま採る(`MergeMode::WholeFile`)。ファイルを持ったことのあるページなら、アイテムごとに決める(`MergeMode::ItemByItem`。下の補足を参照):

1. **ID**: ファイルのメタデータ行 `<!-- @ A-item idx="a0" -->` の ID は B の DB のどこにも無い → そのまま使う。
   (このページには無いが**他のページにはある** ID は「別ファイルからコピーされた」とみなして新 ID を振り、後で書き戻す。そうしないとアイテムがページ間を行き来する)
2. **idx**: メタデータ行の `idx="a0"` をそのまま採用。無ければ前後の間の値を計算。行の順序と `idx` が食い違っていれば行の順序を正とし、食い違った分だけ計算し直して書き戻す
3. **保存**: 既存が無いので保存。AST 生成、ハッシュタグ → tag プロパティ、検索インデックス
4. **プロパティ**: メタデータ行の `key="value"` のスカラー値を upsert。ファイルに無いものは削除
5. **削除**: ファイルに無い既存アイテムは削除(上書き前に実行し、`idx` の衝突を避ける)
6. `recalculate_parent_ids_in()`

仕上げ:

7. 新規ページなので modified はファイルの値のまま
8. `file_sync_state` に `(docs/Ideas.md, A-page, ハッシュ, synced)` を記録 — A と同じハッシュ
9. **export キューには入れない**。B からファイルを書き戻さないので Dropbox に何も流れない
10. SSE `pagesUpdated {pageId, source: "file"}` → サイドバー・最近のページが更新。開いているアウトラインに該当ページがあれば、編集中でなければアイテムを差し替える

両 PC の DB に同じ ID のページ・アイテム、Dropbox に 1 つのファイル、両 PC の `file_sync_state` に同じハッシュ、で収束。

### 補足: 既にあるページのファイルが変わった場合(アイテムごとの Last-Writer-Wins)

両方にあるアイテムは、`modified` の新しい方を採る。日付が同じで中身が違うアイテムは、手で編集されたものとみなし、ファイルの中身を採って `modified` を今にし、すぐに書き戻す。

片方にだけあるアイテム:

- DB にだけあるアイテムは、前回の同期より後に変えたか、まだ書き出していなければ残す。それ以外は、相手が消したとみなして消す
- ファイルにだけあるアイテムは足す。ただし、ユーザーがこの PC で、そのアイテムの `modified` より後に消したもの(`deleted_items` テーブル)は足さない

この PC で見ていないファイルの変更が負けたときは、ファイルを `name.conflict-yyyyMMdd-HHmmss.md` として残す。アトラスは今までどおりページ単位で、DB が新しければファイルをバックアップして DB から書き直す。

### 補足: 書き戻し(write back)は 1 往復で収束する

取り込んだ側が「ファイルに ID が無い」「`idx` が無い/順序と食い違う」「他ページの ID を含む」「DB にファイルより新しい変更がある」と判断したときだけ `queue_export` し、正規化したファイルを書く。相手はそれを受け取っても書き換える理由が無いので、書き戻しは 1 回で止まる。

### 補足: 同じページを両 PC で別々に作った場合(今日のジャーナルなど)

Dropbox は同じパスの 2 ファイルを見て片方を正、もう片方を「conflicted copy」にする(こちらはもう1つの版として合流して消す。`docs/folder-sync-future.md`の案2)。正になった方を相手が取り込むと、ID では見つからず**パスで**見つかり、ID が違う。このとき**古い方の ID が勝つ**(HMTID は時刻順)。負けた側は `PageRepository::rekey_in()` でローカルのページ ID を付け替える(items / export_queue / file_sync_state / page properties / 検索インデックスをまとめて移す)。両 PC で同じ判定をするので必ず同じ ID に収束する。

### 補足: フォルダのindexページ

ページを`d/`に作ると、`d/index`のページもDBにできる(`ensure_ancestor_pages`)。

- アプリで作ったときは、indexページもすぐにexportのキューに入れる。ファイルがすぐにでき、相手のPCはそれを取り込む。自分のindexページを別に作らない。
- 取り込んだファイルのために作ったときは、すぐには書かない。送ってきたPCのindexファイルが、あとから届くことがあるため。届けば、パスで照合して、古いidが勝つ。届かなければ、60秒ごとのスキャンが「ファイルの無いページ」として書く。

### 補足: ページを移動した場合(新しいファイルが先に届く)

PC Aでページを`d/`から`e/`へ移動すると、Dropboxは「`docs/e/c.md`ができた」と「`docs/d/c.md`が消えた」の2つをPC Bへ運ぶ。届く順は決まっていない。

- 古いファイルが先に消えたとき: 新しいファイルのidで見つかるページのファイルが、もう無い。そのため移動として取り込む。
- 新しいファイルが先に届いたとき: 同じidのページのファイルが、まだ残っている。すぐにはコピーと決めず、取り込みを保留する(`Outcome::Waiting`)。DBには入れない。
  - 古いファイルが消えたら、保留したファイルをすぐに取り込み直す。今度は移動として取り込む。
  - 60秒ごとのフルスキャンでも取り込み直す。保留はメモリーにだけあり、再起動で消える。消えてもスキャンが取り込み直す。
  - 5分たっても古いファイルが残っていれば、本当のコピーと決める。新しいidのページを作り、ファイルに書き戻す。
- 移動として取り込んだら、そのページの古いパスの`file_sync_state`の行を消す。消すのは、ファイルがもう無い行だけである。残すと「消えたファイル」になり、移動しただけのページが削除の確認ページに出てしまう。
- スキャンも、ファイルが無い行のうち、ページが別のパスに移り、そのファイルがある行を消す。以前の版の取り込みで残った行を片づけるためである。

---

## ジャーニー 2: PC A でページを削除する

### ステップ 7: PC A でユーザーが「Ideas」を削除する(0 秒)

`DELETE /api/pages/{A-page}` → `PageService::delete_page()`:

1. ルートの index、子を持つ index なら拒否
2. 検索インデックスからページとアイテムを削除
3. `items`、`pages` から削除
4. `export_queue` からも消える(`ON DELETE CASCADE`。直後の export でファイルが作り直されるのを防ぐ)
5. コミットのあと `MarkdownExportService::delete_page_file(page)`: `docs/Ideas.md` を削除し、`file_sync_state` の page_id = A-page の行を全部消す。フォルダが無ければファイルは触らず行だけ消す

### ステップ 8: PC A 自身の Watcher が削除イベントを受ける(直後)

1. 削除のイベント → `pending` へ。2 秒後 `sync_file()` → ファイルが無いので `Missing`
2. `file_sync_state` に行があり `synced` なら `missing` にするが、ステップ 7 で**先に消してある**ので該当なし → 何もしない

「アプリで消した」と「外から消された」の区別は、`file_sync_state` の行が残っているかどうかで付ける。フルスキャンも行を起点に探すので同じ。

### ステップ 9: PC B で `docs/Ideas.md` が消え、確認ページが開く

Dropbox B が削除。PC B の Watcher:

1. 削除のイベント → 2 秒後 `sync_file()` → `Missing`
2. `file_sync_state` に行があり `synced` → `mark_missing()`: status = `missing`、`missing_since` = 今。**DB のページとアイテムは触らない**
3. `PendingDeletePrompt::notify_new_missing()` → 前回開いてから 5 分(`--import-delete-prompt-cooldown`)経っていれば、ブラウザで `http://localhost:8087/api/sync/pending-deletes` を開く(`open` クレート)

確認ページ(サーバーが返す素の HTML。フロントエンドは要らない):

- `missing` の行を集めて表にする: チェックボックス / パス / アイテム数 / 見えなくなった日時
- 注意書き: 選択型同期・同期未完了・オンラインのみのファイルもここに出る。本当に消したものだけチェックする
- ボタン: **Delete selected**(confirm 付き)/ **Write selected back to the folder**

### ステップ 10: PC B のユーザーが確認ページで操作する

**(a) チェックして Delete selected**
`POST` を `Origin`/`Referer` が localhost のときだけ受け付け、`PageService::delete_page()`(ステップ 7 と同じ)。拒否されたページは理由付きで残す。A と B が揃う。B から Dropbox へは何も流れない。

**(b) チェックして Write selected back**
その場で `export_page(A-page)` を呼び、`docs/Ideas.md` を書き直す。行が `synced` に戻る。Dropbox が A に運び、A では新規ページとして取り込まれて復活する。

**(c) 何もしない**
`missing` のまま DB にページは残る。

- B のアプリで編集すると export が走り、(b) と同じく A に復活する。「消したい」と「まだ使いたい」が衝突したときは編集を優先する
- ファイルが戻ってくれば(選択型同期を戻した等)Watcher/スキャンが `synced` に戻し、候補から自動で消える
- **放置は許さない**: 60 秒ごとのフルスキャンは `missing` が 1 件でも残っていれば `notify_new_missing()` を呼ぶ。クールダウンにより **5 分ごとに確認ページが開き続ける**。Delete か Write back を選ぶか、ファイルが戻るまで止まらない

---

## 設定

ジャーナルサーバーのフラグ(または環境変数)で変える。一覧は [BUILD.md](../BUILD.md) の「Rust server」にある。

| フラグ | 既定 | 意味 |
|---|---|---|
| `--export-interval` | 5 | export キューを処理する間隔(秒) |
| `--import-enabled` | true | フォルダ → DB を有効にする |
| `--import-debounce-ms` | 2000 | 最後のファイルイベントから読むまでの待ち(ms) |
| `--import-scan-interval` | 60 | フルスキャンの間隔(秒)。初回は起動 10 秒後 |
| `--import-delete-prompt-cooldown` | 300 | 確認ページを開く間隔の下限(秒) |
| `--import-copy-wait` | 300 | 同じidのファイルがほかに残るとき、コピーと決めるまで待つ時間(秒) |
| `--export-overwrite-wait` | 300 | まだ取り込んでいないファイルを、バックアップして上書きするまで待つ時間(秒) |
