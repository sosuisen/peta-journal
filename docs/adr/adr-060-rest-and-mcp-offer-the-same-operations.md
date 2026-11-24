# ADR 060: REST and MCP Offer the Same Operations

## Status

Accepted

## Context

ジャーナルサーバー（`journal-server`）は、2つのAPIを持つ。
1つはRESTで、Web版、Emacs版、付箋紙などのクライアントが使う。
もう1つはMCPで、AIのクライアントが使う。
どちらも、ADR 029で決めた同じ入口（`journal-core`の`Journal`）を呼ぶ。

### 操作の食い違い

RESTとMCPでは、できる操作がそろっていない。
項目では、貼り付け（`/api/items/{id}/paste`）、分割（`split`）、スタッシュ（`stash`）、バッチ（`/api/items/batch`）、結合（`merge`）、字下げ（`shift-level`）、まとめての削除（`/api/items/delete`）、Flat Markdownの解析（`/api/items/parse`）が、RESTにしかない。
ページ、検索、ジャーナル、アトラス、設定、同期の領域は、まだ並べて比べていない。
逆に、MCPにしかない操作は、いまのところ見つけていない。

### 答えのJSONの食い違い

MCPの道具の一部は、ページと項目を、RESTと違う形で返す。
入口の答えはRESTの形なので、MCPはその答えを書き直している（`presentation/mcp.rs`の`page_record_json`と`item_record_json`）。
違いは次の4つである。

- 日付（`createdDate`、`modifiedDate`）：MCPはISO 8601の文字列（`2026-11-11T10:00:00Z`）で返し、RESTはミリ秒の数で返す。
- プロパティ（`properties`）：RESTの答えにはあり、MCPの答えにはない。
- 親のないページの`parentId`：MCPは`null`で返し、RESTは空の文字列で返す。
- トランスクルージョンの木（`transcludedTrees`）：RESTの`GET /api/pages/{id}`は返し、MCPの`get_page_by_id`は返さない。

この形で返す道具は、`get_page_by_id`、`get_or_create_doc`、`update_page`、`get_recent_pages`、`get_backlinks`、`get_or_create_journal`、`get_most_recent_journal_before`、`create_item`、`update_item`、`move_items`、`get_items_by_type`である。
ほかの項目の道具（`get_item`、`cycle_task`など）は、すでにRESTの形（`ItemDto`）で返す。

### 片方だけを消した事故

2026-11-11に、AIが`POST /api/items/export-subtree`（PR #328）と`POST /api/items/export-page`（PR #330）を消した。
リポジトリの中に、このルートを呼ぶクライアントがなかったためである。
しかし、MCPには同じ操作の道具（`export_subtree_markdown`、`export_page_markdown`）があった。
リポジトリの外のクライアントは、RESTからこの操作を使えなくなった。
PR #331で、2つのルートを戻した。
このとき開発者は、RESTとMCPの方針を`docs/plans.md`に書いた。
この方針は、ADRとして残していなかった。

## Decision

RESTとMCPは、同じ操作を持つ。
RESTは、答えのJSONの形を決める唯一の正（SSOT）である。
MCPは、RESTと同じ形で答えを返す。
ジャーナルサーバーのAPIを足す、変える、消すときは、まずこのADRを読む。

### 同じ操作を持つ

RESTにある操作は、MCPにもなくてはならない。
MCPにある操作は、RESTにもなくてはならない。
操作を足すときは、RESTとMCPの両方に足す。
操作を消すときは、RESTとMCPの両方から消す。
リポジトリの中に呼ぶクライアントがないことは、片方だけを消す理由にならない。
APIはリポジトリの外のクライアントも使うためである。

今、RESTにしかない操作は、MCPに道具を足して埋める。
アセットのアップロード（`POST /api/assets/upload`）も、MCPに足す。
AIのクライアントが、ページに画像を足す場面があるためである。

MCPの道具（`upload_asset`）は、ファイルの中身ではなく、ファイルの絶対パス（`filePath`）を受ける。
サーバーがそのファイルを読み、アセットに写す。
RESTは`multipart`で中身を受けるが、MCPの引数はJSONである。
中身をBase64で受けると、画像1枚で数万トークンを使うためである。
サーバーとAIのクライアントが、同じPCで動くことを前提にする。
答えの形は、RESTと同じである。

### 例外

次のルートは、画面やブラウザーのためだけにある。
MCPには足さない。

- `GET /api/events`：SSEで変更を知らせる。
- `GET /api/health`：サーバーが動いているかを答える。
- `POST /api/config/browse-folder`：PCのフォルダーを選ぶダイアログを出す。
- `POST /api/pages/{id}/open-folder`：PCのOSで、フォルダーを開く。
- `GET /assets/{file_name}`：アセットのファイルを、ブラウザーへそのまま返す。
- `POST /api/confirm/{request_id}`：拡張関数がクライアントに出した質問に答える。MCPのクライアントは、この質問を受け取らない。
- `GET /api/sync/pending-deletes`、`POST /api/sync/pending-deletes`：消えたファイルの扱いを選ぶHTMLの画面と、そのフォームの送信先である。ブラウザーのためだけにある。

例外を増やすときは、このADRを改める。

### 答えのJSONの形

答えのJSONの形は、RESTが決める。
MCPの道具は、同じ操作のRESTのルートと同じ形で返す。
MCPのために、RESTと違う形へ書き直さない。

ただし、MCPの道具は、項目の`ast`を返さない。
ADR 041で、MCPは項目に`ast`を付けないと決めたためである。
AIには本文があれば足り、`ast`は答えを長くするだけである。
違いは、この1つだけである。

日付の既定は、ミリ秒の数である。
RESTは、クエリの`dateFormat=iso`を受けたとき、日付をUTCのISO 8601の文字列（`2026-11-11T10:00:00Z`）で返す。
MCPの道具は、同じ名前の引数`dateFormat`を受け、同じ形で返す。
AIのクライアントには、数より文字列の方が読みやすいためである。
引数がなければ、MCPもミリ秒の数で返す。
既定の答えが、RESTとMCPで同じになるためである。

### テストで守る

RESTのルートとMCPの道具の対応表を、`journal-server`のテストに持つ。
テストは、サーバーに登録されたルートと道具を、この表と比べる。
対応する相手のない操作があれば、テストは落ちる。
例外のルートは、表の中で例外として書く。
こうすれば、片方だけに足したり、片方だけから消したりすると、PRの前に気づける。

### 対象の外

組み込みのAPI（`client-api-embedded`）は、このADRの対象の外である。
組み込みのAPIは、AndroidとiOSのシェルの都合で操作を選ぶためである。

## Alternatives

### Alternative 1: 呼ばれていないルートを消す

リポジトリの中のクライアントが呼ばないルートや道具を消す。

- 長所：コードが減る。
- 短所：リポジトリの外のクライアントが壊れる。RESTとMCPで、できることがずれていく。

PR #328とPR #330の事故は、この考え方から起きた。
そのため採らなかった。

### Alternative 2: MCPはAIに向く操作だけを持つ

MCPは、RESTの操作のうち、AIが使いそうなものだけを持つ。

- 長所：MCPの道具の数が少なく、AIが道具を選びやすい。
- 短所：どの操作がAIに向くかを、その都度決める必要がある。決めた結果が、テストで確かめられない。

足すか消すかの判断が人ごとにぶれるので、採らなかった。

### Alternative 3: MCPを唯一の正にする

答えの形をMCPが決め、RESTがそれに合わせる。

- 長所：AIのクライアントに読みやすい形を、そのまま正にできる。
- 短所：RESTを使うクライアントが多い。Web版、Emacs版、付箋紙、todoの答えの読み方を、すべて変える必要がある。

変える量が多いので、採らなかった。
読みやすさは、`dateFormat`の引数で補う。

## Confidence Level

確信度は高い。
RESTとMCPは同じ入口（`Journal`）を呼ぶので、同じ操作を持たせる費用は小さい。
答えの形を1つにすれば、MCPの書き直しのコードも消せる。

次のことが起きたら、この決定を見直す。

- RESTかMCPの片方だけに要る操作が、例外の一覧に収まらないほど増えたとき。
- MCPの道具が多すぎて、AIのクライアントが道具を選び間違えることが増えたとき。
- RESTの形が、AIのクライアントにとって、`dateFormat`のほかにも読みにくいと分かったとき。

## Consequences

良い結果は、次のとおりである。

- RESTとMCPのどちらから使っても、同じことができる。
- 片方だけを足したり消したりすると、対応表のテストが落ちる。
- MCPの`page_record_json`と`item_record_json`の書き直しが要らなくなる。
- MCPの答えに、プロパティとトランスクルージョンの木が入る。

悪い結果は、次のとおりである。

- MCPの答えが変わる。
  日付は既定でミリ秒の数になり、親のないページの`parentId`は空の文字列になる。
  MCPを使う側を、先に確かめて直す必要がある。
  `peta-journal`、`kcg-lecture`、`report`などのスキルと、日付を文字列として読んでいるところである。
- RESTにしかない操作の分だけ、MCPの道具が増える。
  MCPのクライアントに渡す道具の説明が長くなる。
- アセットのアップロードの引数は、RESTとMCPで違う。
  別のPCのクライアントは、MCPでアップロードできない。
  MCPのクライアントは、サーバーが読めるPCのファイルを、アセットへ写せる。

中立的な結果は、次のとおりである。

- `docs/plans.md`の「APIの方針：RESTとMCP」の節は、このADRに移した。plansには、この節のタスクを置かない。
- 対応表のテストは、まだない。
  RESTにしかない操作をMCPに足し終えるまでは、足りない操作を、表の中で一時的に印を付けて通す。
- 答えの形をそろえる作業は、項目の操作を入口へ下ろし終えてから行う。
  途中でそろえると、下ろす作業と答えの変更が1つのPRに混ざるためである。
