# ADR 046: Property Definitions Synced by a JSON File

## Status

Accepted

## Context

プロパティの定義（キー、型、ページで使えるか）は、SQLite の `property_definitions` にしかない。
ファイルには出ない。
そのため、別の PC やモバイルの端末には、ユーザーが定義したプロパティとその型が届かない。
定義のない端末は、ページのファイルから取り込んだキーを `text` として定義する（2026-11-14 の修正）。

端末の間で届くのは、Markdown フォルダーのファイルだけである。
PC では Dropbox のアプリがフォルダーを同期し、サーバーが監視と走査で取り込む。
Android と iOS では、シェルが Dropbox の SDK で送受信し、core が走査で取り込む（ADR 030、ADR 034）。
モバイルの同期が運ぶのは、`.md` で終わるファイルと `assets/` の下だけである（`MarkdownFolder.keeps`）。

定義をファイルに置くと、定義のファイルとページのファイルが、決まった順では届かない。
ページのファイルが先に届けば、そのキーはまだ定義がない。
定義が先に届けば、そのキーの値はまだない。
届いた順で判定すると、結果が端末ごとに変わる。
猶予期間を置いて待つ方法もあるが、どれだけ待てばよいかは決められない。

アトラスの `index_atlas.md` の同期でも、同じ問題があった。
アトラスは、待つのではなく、2 つの決まりで順序の問題を消している。
まだ知らないページのカードは、消さずに残す。
アトラスのファイルを取り込んだときは、アトラスを作り直さない（`docs/properties.md`）。

今の DB には、もう 1 つの問題がある。
値を入れる列が型で決まる。
`number` は `value_number` に、`checkbox` は `value_checkbox` に、ほかは `value_text` に入る。
読むときも、型の列だけを読む。
そのため、型が変わると読む列も変わり、値が見えなくなる。

例 1：ページのファイルが先に届き、定義があとから届く。

- Mac で `score` を `number` と定義し、ページ Report に `12` を入れる。
- iPhone に `docs/Report.md`（`score: 12`）が先に届くと、`score` を `text` として定義する。
  値は `value_text` に `"12"` として入る。
- 次に `settings/properties.json` が届き、`score` は `number` になる。
  `number` は `value_number` を読むが、そこは空である。
  よって、画面から値が消える。
- iPhone が次に `Report.md` を書くと、ファイルからも `score` が消え、その版が届いた Mac の値も消える。

例 2：型が `number` から `text` に変わる。

- Mac の DB では、`score` の値は `value_number` に `12` として入り、`value_text` は空である。
- iPhone で後から `score` を `text` と定義すると、Mac でも `text` になる。
  `text` は `value_text` を読むので、値が消える。

例 3：新しい型に合わない値。

- iPhone で先に `score` を `text` と定義し、ページ Memo に `abc` と入れていた。
- そこへ Mac の、より新しい `number` の定義が届く。
  `abc` は数ではないので、今の列の持ち方では入れる場所がない。

ほかにも、今の持ち方は元の文字列を残さない。
`"1.50"` は `1.5` になり、`checkbox` のキーに来た `"yes"` は、黙って偽になる。

## Decision

定義を Markdown フォルダーの `settings/properties.json` に置き、端末の間で同期する。
届く順で結果が変わる判定はしない。
値は、ファイルに書かれた文字列のまま持ち、型はその読み方とする。

### ファイルの形式

JSON で、キーごとに型、ページで使えるか、定義した日時を書く。
消した定義は、消した日時だけを書いて残す。

```json
{
  "definitions": {
    "owner": { "type": "text", "page": true, "modified": "2026-11-15T01:00:00Z" },
    "score": { "type": "number", "page": false, "modified": "2026-11-15T02:00:00Z" },
    "old": { "deleted": "2026-11-14T09:00:00Z" }
  }
}
```

システム管理キーは書かない。どの端末でも、起動時に同じ定義を作るためである。

キーは辞書順に並べ、同じ定義からは、どの端末でも同じバイト列を書く。

### ぶつかったとき

キーごとに、新しい方を採る。
定義の日時は `modified`、消した定義の日時は `deleted` である。

2 台で別のキーを足せば、両方が残る。

同じキーを別の型で定義すれば、後に定義した方の型になる。

消した定義より後に定義し直せば、定義が残る。

日時が同じなら、次の順で決める。どの端末でも同じ結果にするためである。

1. 消した方を採る。
2. `text` より、`text` 以外の型を採る。
   `text` は、型の分からない値に与える型である。
   ほかの型は、だれかが選ばなければ付かない。
3. 型の名前の辞書順で、後の方を採る。
4. 型も同じなら、`page` が真の方を採る。
   偽にするほうが影響が大きいため。

### 消した定義の記録

端末は、ユーザーが定義を消したときと、定義を消したことを `settings/properties.json` から受け取ったときに、その端末の DB の表 `deleted_property_definitions(key, deleted_date)` に、キーと消した日時を入れる。
項目の `deleted_items(id, deleted_date)`（ADR 006）と同じ形である。
端末は、`settings/properties.json` を書くとき、この表の行を、消した定義（`deleted` だけを持つ）として書く。
端末は、届いた `settings/properties.json` の定義の `modified` を、この表の日時と比べて、新しい方を採る。
この表がないと、消すより前に書かれた `settings/properties.json` が届いたときに、端末はそのキーを知らないキーとして受け取り、消した定義が生き返る。
端末は、次の 2 つのときに、この表からそのキーの行を消す。

- その端末のユーザーが、同じキーを定義し直したとき。
- 届いた `settings/properties.json` に、この表の日時より新しい、同じキーの定義があったとき。

この表の行は、ほかには消さない。いつ消してよいかを、端末は知ることができないためである。

### 取り込みと書き出し

例として、iPhone が、Mac の書いた `settings/properties.json` を受け取る場合を書く。

1. iPhone は、Mac の書いた `settings/properties.json` の定義と、iPhone の DB の定義（`property_definitions` の本物の定義と `deleted_property_definitions`）を、キーごとに合わせる。
2. 合わせた結果が、Mac の書いた `settings/properties.json` と違えば、iPhone は `settings/properties.json` を書き直す。
   iPhone の DB にしかない定義を、Mac に届けるためである。
3. iPhone が書き直した `settings/properties.json` が Mac に届くと、Mac も 1 と同じく合わせる。
   合わせた結果は、iPhone の書いたファイルと同じになるので、Mac は書き直さない。

書く中身は、合わせた結果だけで決まる。
そのため、Mac と iPhone が `settings/properties.json` を送り合い続けることはない。

端末は、その端末のユーザーが定義を足したときと消したときにも、`settings/properties.json` を書く。

Dropbox が `settings/properties.json` の conflicted copy を作ったら、端末は、それを Mac の書いたファイルと同じく、もう 1 つの版として合わせてから消す。

### 値の持ち方

スカラーの値は、型にかかわらず、ファイルに書かれた文字列のまま `properties` の `value` に入れる。
`properties` の値の列は、これ 1 つにする。今、`value_number` と `value_checkbox` を読むのは、値を文字列に戻して返す 1 か所だけで、数で比べる検索も並べ替えもないためである。
型は、REST と MCP から書くときのチェックと、画面での見せ方にだけ使う。
型が変わっても、DB の値は変えない。

DB の版を上げるとき、今ある `number` と `checkbox` の値を、書き出しと同じ形の文字列にして `value_text` に移す（`12.0` は `"12"`、`1` は `"true"`）。
そのあと、`value_number` と `value_checkbox` の列を消し、`value_text` の名前を `value` に変える。
リストの表 `property_list_items` の要素の列も `value` なので、2 つの表で名前がそろう。

例 1 から例 3 は、次のようになる。
例 1 では、定義が届いたあとも `value` の `"12"` は変わらず、`number` として読む。
例 2 では、同じ `"12"` を `text` として読む。
例 3 では、`value` の `"abc"` は残る。
画面には元の文字列を見せる。
あとで `text` の定義が勝てば、普通の値に戻る。

REST や MCP から型に合わない値を書くと、今と同じく 400 で断る。
型に合わない値が端末の DB に入るのは、次の 2 つのときだけである。

- 端末が、ページのファイルから、型に合わない値を取り込んだとき。
- 端末が、届いた `settings/properties.json` でキーの型を変え、端末の DB の値がその型に合わなくなったとき（例 3）。

システム管理キー（`atlas_bounds` など）は、どの端末の DB でも型が変わらず、atlas などが型どおりの値を読む。
そのため、端末は、ページのファイルから来たシステム管理キーの値が型に合わなければ、今と同じく読み飛ばす。

### 数で比べる検索を足すとき

数を文字列で持つと、文字列のまま比べれば `"10"` が `"9"` より小さくなる。
`"1.5"` と `"1.50"` も、文字列としては別の値である。
SQLite の `CAST('abc' AS REAL)` は、エラーにならずに `0.0` を返す。
今は数で比べる検索がないので、どれも困らない。
`rating > 3` のような検索や、数での並べ替えを足すときは、`value` から作る生成列（`GENERATED ALWAYS AS (...)`）を `properties` に足し、そこに索引を張る。
生成列は、数の形の値だけを数にし、ほかは `NULL` にする。
正本は `value` の 1 つのままで、生成列は SQLite が必ず `value` から作り直す。
コードが写しを正本に合わせる必要はない。

### リストのキー

`list` 型のキーの名前は `_list` で終わらせる（`member_list`、`author_list` など）。
名前が `_list` で終わるキーは、`list` 型にする。
システム管理キーの `tags` だけは例外である。
journal-core は、キーを `PropertyKey` の型で持つ。
`PropertyKey` は、スカラーのキー（`ScalarKey`）か、リストのキー（`ListKey`）である。
`ListKey` は、`tags` か、名前が `_list` で終わるユーザー定義のキー（`UserListKey`）である。
キーの文字列から `PropertyKey` を作るときに名前を確かめるので、この決まりに合わないキーは、journal-core の中で作れない。
名前でスカラーかリストかが決まるので、定義が届く前でも、どの端末でも同じ判断になる。
スカラーとリストの間で型が変わることはない。
REST と MCP では、この決まりに合わない定義と値を 400 で断る。
今の DB に、ユーザー定義の `list` のキーはないので、名前を変えるマイグレーションは要らない。

リストの値は、今と同じく `property_list_items` に、要素ごとに 1 行で入れる。
Obsidian では、`contains(composers, [[Paul McCartney]])` のような要素での検索と、要素ごとのリンクが、リストの主な使いみちである。
要素を 1 行ずつ持てば、`(key, value)` の索引で要素を引ける。

### ページのファイルは書き直さない

iPhone は、Mac の書いた `settings/properties.json` を取り込んでも、`docs/` と `journals/` のページのファイルを書き直さず、iPhone の DB のページの `modified` も進めない。
アトラスと同じである。
iPhone がページのファイルを書き直すと、Mac がそのあとに書いた、より新しいページのファイルとぶつかるためである。

### 仮の定義

端末は、DB の `property_definitions` に仮の定義を持つことができる。
仮の定義は、その端末の DB の `properties` と `property_list_items` にある値を読むためだけのものである。

端末は、仮の定義を本物の定義と区別する。
仮の定義は、端末の DB の `property_definitions` で、`provisional` の列が 1 の行である。
本物の定義は、`provisional` が 0 の行である。

仮の定義は `settings/properties.json` ファイルに書かない。

ファイルから同じキーの定義が届けば、仮の定義はいつでもそれに置き換わる。

仮の定義ができるのは、次の 2 つのときである。

- 定義のないキーの値が届いたとき。
  端末は、名前が `_list` で終わるキーには `list` の、ほかのキーには `text` の仮の定義を作る。
- 定義を消したことが届いたとき。
  下の「消したことが届いたとき」に書く。

DB の版を上げるとき、端末は、今あるユーザー定義をすべて本物の定義とし、日時を 0 とする。
今の DB では、ユーザーが定義したものと、2026-11-14 の修正のあと取り込みで作ったものを区別できない。
しかし、取り込みで作ったものは `text` なので、同じ日時の決まりの 2 により、ほかの端末のユーザーが選んだ型に負ける。
どの端末で新しく定義しても、日時が 0 より新しいので、そちらが勝つ。

### 消したことが届いたとき

例として、Mac のユーザーが `score` の定義を消し、iPhone がそれを受け取る場合を書く。
iPhone の DB では、ページ Report が `score: 12` を持っているとする。

#### Mac が消すとき

1. Mac のユーザーが、Mac の Settings で `score` の定義を消す。
2. Mac は、Mac の DB のすべてのページと項目から `score` の値を消す。
   値を失ったページ Report の `modified` を進め、Mac は `docs/Report.md` を書き直す。
   今と同じ処理である。
3. Mac は、Mac の DB の `property_definitions` から `score` の行を消し、消した日時を、Mac の DB の `deleted_property_definitions` に入れる。
4. Mac は、`settings/properties.json` に、`score` を消した定義（`deleted` 付き）として書く。

#### iPhone が、Mac の書いた `settings/properties.json` を受け取るとき

5. iPhone は、Mac の書いた `settings/properties.json` の `score` の `deleted` と、iPhone の DB の `score` の定義の `modified` を比べる。
   `deleted` の方が新しければ、6 に進む。
   古ければ、iPhone は iPhone の DB の `score` の定義を残す。
   iPhone の DB の `score` が仮の定義のときと、iPhone の DB に `score` の定義がないときは、比べずに 6 に進む。
6. iPhone は、消した日時を、iPhone の DB の `deleted_property_definitions` に入れる。
7. iPhone は、iPhone の DB に `score` の値がまだあるかを見る。
   - ある場合：iPhone は、iPhone の DB の `property_definitions` の `score` の行を残し、仮の定義にする。
     型は変えない。
     iPhone は、iPhone の DB の `score` の値を消さない。
     `docs/Report.md` も書き直さない。
   - ない場合：iPhone は、iPhone の DB の `property_definitions` から `score` の行を消す。
8. iPhone は、`settings/properties.json` を書き直さない。iPhone の DB の `score` は、消した定義として Mac の書いたファイルと同じだからである。

#### iPhone が、Mac の書いた `docs/Report.md` を受け取るとき

9. iPhone は、Mac の書いた `docs/Report.md` を、今と同じく取り込む（ADR 006）。
   Mac の書いた版の方が新しいので、iPhone の DB の Report から `score` の値が消える。
10. iPhone は、iPhone の DB に `score` の値が 1 つも残っていないかを見る。
    残っていなければ、iPhone は、iPhone の DB の `property_definitions` から、`score` の仮の定義の行を消す。
    ほかの仮の定義も同じである。
    端末は、ページのファイルを取り込んで、端末の DB から仮の定義のキーの値がなくなったら、その仮の定義の行を消す。
    仮の定義は、値を読むためだけのものだからである。

#### 補足

- 9 が 7 より先に起きることもある。
  そのとき、iPhone が `settings/properties.json` を受け取る 7 の時点では、iPhone の DB に `score` の値がない。
  iPhone は、7 で `score` の行を消す。
  どちらの順でも、最後は同じになる。
- 7 で iPhone が iPhone の DB の値を消さないのは、iPhone のユーザーが、Mac で消したことを知らずに、後から `score` の値を書いていることがあるからである。
  そのときは、iPhone の DB の Report の方が新しいので、9 で Mac の書いた `docs/Report.md` を取り込んでも、`score` の値は残る。
  iPhone が書いた `docs/Report.md` が Mac に届くと、Mac の DB にも `score` の値が入り、Mac は `score` の仮の定義を作る。
- 仮の定義を `property_definitions` の行として残すのは、iPhone の DB の値の表（`properties` と `property_list_items`）の `key` が、外部キーとして `property_definitions(key)` を指しているからでもある。
  値のあるキーの行は消せない。

### ページのファイルを取り込むときの `for_page`

Mac は、iPhone の書いた `docs/Report.md` を取り込むとき、frontmatter のユーザー定義のキーを、Mac の DB の `for_page` にかかわらず、すべて Mac の DB のページ Report の値として入れる。
今は、Mac は、Mac の DB で `for_page` が 0 のキーを読み飛ばす（`replace_page_properties`）。
読み飛ばすと、iPhone が Mac の書いた `settings/properties.json` を受け取る前に、iPhone のユーザーがそのキーをページに付けたとき、Mac の DB にその値が入らない。
Mac が次に `docs/Report.md` を書くと、その値は Mac の書いたファイルからも消え、iPhone の DB からも消える。
`for_page` は、ページのプロパティの UI で選べるキーを決めるためだけに使う。
システム管理キー（`zoom` など）は、どの端末の DB でも同じ定義なので、Mac は今と同じく読み飛ばす。

### モバイルのシェル

Android と iOS のシェルが、`settings/properties.json` もアップロード・ダウンロードするようにする。

### PC のサーバーと core の走査

PC のジャーナルサーバーは、Markdown フォルダーの監視で、`settings/properties.json` の変更も受け取る。
core の走査（`MarkdownReconcileService`）も、`docs/` と `journals/` に加えて、`settings/properties.json` を見る。
今は、走査は `docs/` と `journals/` しか見ず、`path_mapper` は `.md` のファイルしか扱わないためである。
Android と iOS のアプリは、ダウンロードのあとの core の走査で、`settings/properties.json` を取り込む。

## Alternatives

### Alternative 1: 定義を `.md` のファイルに置く

Markdown の表などで、`.md` のファイルに定義を書く。
モバイルの同期は `.md` を運ぶので、Kotlin と Swift を変えずに済む。
しかし、中身はページではなく設定である。
表は自分で解析することになり、壊れやすい。
ほかの Markdown のツールでは、ノートとして見えてしまう。
JSON なら、journal-core がすでに使っている `serde_json` で読み書きできる。
採らない。

### Alternative 2: YAML のファイル

YAML は人が書きやすい。
しかし、journal-core には YAML を読む依存がない。
よく使われていた `serde_yaml` は、もう保守されていない。
採らない。

### Alternative 3: ファイルごとに新しい方を採る

新しいファイルが、定義をすべて決める。
作りは簡単である。
しかし、2 台がオフラインで別々にキーを足すと、片方の定義が消える。
採らない。

### Alternative 4: 足すだけにして、消すことは端末ごとに行う

ファイルの定義を足すだけで、消したことは伝えない。
消した日時を持たずに済む。
しかし、消した定義は、別の端末が書いたファイルから戻ってくる。
採らない。

### Alternative 5: 猶予期間を置いて判定する

定義やページのファイルが届いてから一定の時間待ち、そのあとで型や値の有無を判定する。
今の値の持ち方のままで済む。
しかし、どれだけ待てばよいかは決められない。
モバイルの端末は、数日同期しないこともある。
待っても、届く順で結果が変わることは消えない。
採らない。

### Alternative 6: 型が変わったら、合わない値を消す

型に合う値だけを新しい型に直し、合わない値は消す。
今の列の持ち方のままで済む。
しかし、例 3 では、ユーザーが書いた値が消え、ページのファイルからも消える。
届く順によっては、型が戻ったあとも値は戻らない。
採らない。

### Alternative 7: `value_text` を正本にし、`value_number` と `value_checkbox` を写しとして残す

値を `value_text` に文字列で持ち、数と真偽値の列は、そこからコードが作る写しにする。
数で比べる検索に、すぐ索引を使える。
しかし、今はその列を使う検索がない。
写しを正本に合わせるコードが要り、合わせ忘れると 2 つが食い違う。
検索が要るときは、SQLite が作る生成列で足りる。
採らない。

### Alternative 8: ユーザー定義のリストも `properties` の `value` に入れる

リストを `"a,b"` の 1 つの文字列として持つ。
`text` と `list` の間で型が変わっても、DB の値を変えずに済む。
しかし、要素で探すには `LIKE` や `json_each` を使うことになり、索引が効かない。
`LIKE '%work%'` は `homework` にも当たる。
要素での検索は、Obsidian のリストの主な使いみちである。
名前でスカラーかリストかを決めれば、型がその間で変わることはない。
採らない。

### Alternative 9: 英語の複数形でリストと判断する

`authors` や `composers` のように、`s` で終わる名前をリストにする。
Obsidian でも、複数形の名前にリストを入れる慣習がある。
しかし、`status`、`address`、`news` は `s` で終わるのにスカラーである。
`people` や `data` は、`s` で終わらないのにリストにしたい名前である。
採らない。

## Confidence Level

中程度である。
キーごとに新しい方を採る決まりと、値を文字列のまま持つ決まりは、どちらもページの同期（ADR 006）とアトラスの同期で、うまく動いている考え方である。

次のときは見直す。

- 定義の型を、ユーザーが UI で変えられるようにするとき。
  今は、定義の型は最初に決まり、変わるのは同期のときだけである。
- 週番号やタグのフォルダーなど、ほかの設定も同期するとき。
  `settings/` に置く形を、ほかの設定と合わせて考え直す。
- 端末の時計が大きくずれていて、定義の新旧が逆になるとき。

## Consequences

- 別の端末で定義したキーと型が、PC にもモバイルにも届く。
  届く順にかかわらず、最後はどの端末も同じ定義と値になる。
- ページのファイルから来た値は、型が変わっても消えない。
  元の書き方（`"1.50"` など）も残る。
- ユーザー定義のキーでは、型に合わない値が、端末の DB に入りうる。
  画面に見せる処理は、それを考える必要がある。
  型に合わない値に警告の印を付けるのは、シェルごとの別のタスクとする。
- 数で比べる検索と並べ替えを足すときは、`properties` に生成列を足す。
  文字列のまま比べてはならない。
- ユーザーは、リストのキーに `_list` を付けなければならない。
  スカラーのキーには付けられない。
  Obsidian の `authors` のような名前は、`author_list` になる。
- DB の版が 1 つ上がる。
  `properties` の値の列は `value` の 1 つになる。
  `property_definitions` に、定義の日時と、仮の定義かどうかを足す。
  消した定義を残す表 `deleted_property_definitions` を足す。
- DB の版を上げた端末は、取り込みで作った `text` の定義も、本物の定義としてファイルに書く。
  そのキーをほかの端末で `text` 以外の型で定義していれば、そちらが勝つ。
- Android と iOS のシェルに、`settings/properties.json` を運ぶ変更が要る。
  Android はこの Mac でビルドできないので、実機での確認が要る。
  古い版のアプリは、このファイルを運ばない。
  古い版のアプリの core には仮の定義がないので、その端末では、今と同じく、知らないキーに `text` の本物の定義を作る。
- 定義を消したことが届いても、値はページのファイルが届くまで残る。
  その間、そのキーは仮の定義として、Settings に見える。
