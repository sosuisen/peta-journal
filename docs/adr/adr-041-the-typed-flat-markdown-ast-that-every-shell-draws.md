# ADR 041: The Typed Flat Markdown AST That Every Shell Draws

## Status

Proposed

## Context

Flat Markdown の AST は、項目の本文を表示するための型のつもりだった。しかし今、AST から表示を作る方法は、シェルごとに違う。

ADR 024 は、ビューモデルを型として生成し、境界に通すと決めた。ただし、項目の AST（`Item.ast`）は JSON の文字列のまま通した。シェルの側には、AST のノードの型がない。

ADR 025 は、そのためにシェルで AST を読まないと決めた。Rust の `ast_to_html` は、ノードの種類を1つの `match` で受ける。種類を書き落とせば、コンパイルが通らない。シェルの言語で JSON を読むと、この保証がなくなる。そこで、Rust が HTML を作り、シェルは OS の HTML の変換に任せることにした。

OS の HTML の変換は、どれも一部の種類を落とした。シェルは、それぞれ別の方法でこれを補った。

- macOS は、`NSAttributedString` の変換で消えるチェックボックス、区切り線、画像を、HTML の文字列を置き換えて補った。
- Windows には、HTML の変換がない。ADR 028 は、HTML を C# で読むことにした。
- Android の Compose は、コードブロック、表、画像を描けない。ADR 030 は、この3つだけ Kotlin で AST の JSON を読むことにした。ADR 025 が避けた方法である。
- iOS は、Android と同じ形にした（ADR 034）。同じ Swift で書く macOS とは、描き方が違う。

AST の形にも、型として渡すのに向かない部分がある。リストの種類、区切り、表の揃え、アラートの種類は、文字列で持っている。画像の `props` は、`serde_json::Value` の対応表である。未知の種類（`Unknown`）は、serde の手書きの処理に頼っている。どれも、facet で型を生成できない。

AST の JSON を使う場所を調べた（2026-11-14）。journal は、本文を保存するたびに解析し、DB の `items.ast` に入れる。これを、ハッシュタグの取り出し、バックリンクの確認、項目のコピーに使う。REST は、項目ごとに `ast` を送る。client の core は、これを運ぶだけである。MCP も、項目の記録に `ast` を付ける。しかし、AI には本文があれば足りる。core を使わないアプリも2つある。`web-app/todo` は REST の `ast` を `flatmarkdown-ast2react` で描く。`webview-app/memoview-win` は、同じ `ast` を WebView の中の `flatmarkdown-ast2html` で HTML にする。

AST は、本文と決まったオプションだけから作られる。オプションは、`./` と `../` で始まるリンクを wikilink にし、`#tag` をハッシュタグにするものである。journal しか知らない情報は使わない。このオプションの正本は、`journal-core` の `markdown_parser.rs` にある。flatmarkdown には、仕組み（`LinkResolution::On { prefixes }`）だけがあり、既定値は `Off` である。同じ値を、Java のラッパーと client のテストが手で写している。そのうち `client-native-bridge/src/tests/html.rs` の写しは、接頭辞が空で、すでにずれている。

AST を型として渡せるかを確かめた（2026-11-14）。

- **生成**：AST と同じ形の再帰する enum から型を生成した。Swift は `indirect enum` になり、ビルドが通った。Kotlin は `sealed interface`、C# は `sealed record` になった。この2つは、手元にコンパイラがなく、ビルドを試していない。
- **大きさ**：このリポジトリの `docs` の 2383 項目で、AST の JSON は本文の約2.1倍だった。bincode にすると、JSON の約 0.75 倍になる。この値は、書式の規則から計算した推定である。
- **時間**：同じ項目で、本文を解析して `Node` にするのに 1項目 5.2µs かかった。AST の JSON を読んで `Node` にするのには 9.8µs かかった。本文から解析する方が速い。今ある最大のページ（1063 項目、ADR 018）を解析すると、この Mac で約 5.5ms である。電話では測っていない。

## Decision

AST を、表示に使える型に直す。そのうえで、すべてのシェルが、生成された AST の型から直接描くようにする。AST を JSON の文字列で保存したり運んだりするのは、やめる。

**flatmarkdown：AST の型を直す**

`flatmarkdown-ast` の `Node` を、型として生成できる形に直す。

- リストの種類と区切り、表の揃え、アラートの種類は、文字列ではなく enum で持つ。
- 画像の `props` は、型のある値で持つ。`serde_json::Value` は使わない。
- `Unknown` の種類名は、ふつうのフィールドとして持つ。serde の手書きの処理は使わない。

facet の derive は、`facet` の feature の中に置く。derive は、`Node` から Swift（macOS、iOS）、Kotlin（Android）、C#（Windows）の型を生成するときにだけ要る。Rust で `Node` を使う側（client の core、Web、Emacs、journal、`ast2html`、`ast2react`、flatmarkdown の外の利用者）には要らない。PetaJournal は、型を生成するときだけこの feature を有効にする。

型が変わるので、flatmarkdown のすべてのクレートの版を 0.8 に上げる（ADR 020）。`SPEC_AST.md` も、新しい型に合わせて直す。`flatmarkdown-ast2html` と `flatmarkdown-ast2react` は、名前を変えずに新しい型に合わせる。

解析のオプションは、flatmarkdown に置く。flatmarkdown は、ページへのリンクを見分ける規則を、名前の付いた既定値として持つ。既定値は、`./` と `../` で始まり `.md` で終わる相対リンクを wikilink にし、`#tag` をハッシュタグにする。ほかのディレクトリ構造を使う利用者のために、リンクの接頭辞は今の `LinkResolution::On { prefixes }` のように引数で変えられるまま残す。PetaJournal（journal と client の core）と Java のラッパーは、この既定値を使う。値の写しは持たない。同じ本文から、いつも同じ AST ができるようにするためである。

**client の core：本文から AST を作る**

core は、本文を `flatmarkdown` で解析し、`Node` を作る。ビューモデルの項目は、AST を文字列ではなく `Node` で持つ。`Node` は、ビューモデルのほかの型と一緒に、Swift、Kotlin、C# の型として生成される。

各シェルは、生成された `Node` を `switch` や `when` で受け、OS の部品で描く。AST に種類が増えれば、型を作り直したときに、シェルのコンパイルがそれを知らせる。この決定は、ADR 025 の HTML を経由する描き方を置き換える。ADR 028 の C# で HTML を読む部分と、ADR 030 の `fromHtml` と Kotlin で JSON を読む部分も置き換える。

コピーの `CopyMarkdown`（ADR 027）も、journal の AST を受け取らない。core が Markdown を自分で解析する。

**journal：AST を保存も送信もしない**

journal は、DB の `items.ast` の列と、`ItemPatch` の `ast` をなくす。ハッシュタグとバックリンクは、本文を解析して取り出す。MCP は、項目の記録に `ast` を付けない。

REST は、core を使わない `web-app/todo` と `webview-app/memoview-win` のために、項目の AST を送り続ける。ただし、DB から読むのではなく、応答を作るときに本文から作った `Node` の JSON を送る。

**順序**

flatmarkdown、client、journal の順に変える。client は新しい flatmarkdown を使い、journal の `ast` を読まなくなる。その後で、journal から `ast` を外す。

## Alternatives

### Alternative 1: HTML を経由したまま、シェルごとの HTML の読み方を揃える

各シェルが Rust の HTML を自分で読む（ADR 025 の Alternative 2。Windows は ADR 028 でこれを採った）。OS の変換は使わないので、種類が消えることはない。しかし、HTML の読み手を4つの言語で書くことになる。HTML には型がないので、Rust が新しいタグを出しても、シェルのコンパイルは気づかない。ずれの原因がそのまま残るので、採らない。

### Alternative 2: AST とは別に、表示用の型（`RichText`）を作る

core が AST を表示用の型に変え、シェルはそれを描く。この型は、AST と同じく本文の木である。同じものを2つの型と2つの仕様で持つことになる。AST を表示に使える形に直せば足りるので、採らない。

### Alternative 3: `flatmarkdown-ast` は変えず、PetaJournal の側に同じ形の型を作る

公開クレートの型を変えずに済む。しかし、型が2つになり、写す処理も要る。`ast2html` と `ast2react` は、直さない古い型を使い続ける。AST が表示用の型であるという考えに合わないので、採らない。

### Alternative 4: journal の AST の JSON を、core が `Node` に読む

解析するのは journal だけになる。しかし、AST の JSON を読むのは、本文を解析するより遅かった（1項目 9.8µs と 5.2µs）。DB の列と REST の大きさも残る。解析のオプションを flatmarkdown に置けば、同じ AST ができない心配はなくなる。なので、journal に解析をまとめる利点はない。採らない。

### Alternative 5: ビューモデルの AST を、JSON のまま手書きで読む

ADR 025 の Alternative 1 である。シェルの言語では、種類の書き落としにコンパイラが気づかない。採らない。

## Confidence Level

確信度は中程度から高い。

AST を型として渡すことは、確かめた。再帰する enum から Swift の型が生成でき、ビルドも通った。本文を解析する時間は、AST の JSON を読む時間より短かった。AST の JSON を使う場所も、journal、REST、MCP、コピー、todo、memoview のすべてを調べた。

確かめていないことが3つある。

- Kotlin と C# の型は、生成はできたが、ビルドを試していない。今のビューモデルと同じ形（sealed の型）なので、通ると見ている。
- 電話の上での解析の時間は、測っていない。
- facet の版は `=0.46.5` に固定している。公開クレートでこの固定を緩められるかは、試していない。

次のときは、この決定を見直す。

- Kotlin か C# で、再帰する `Node` の型がビルドできないか、読み書きが正しくないとき。
- 電話で、項目の多いページを開くと、解析のせいで目に見えて遅くなるとき。
- facet の版の固定のせいで、flatmarkdown を公開しにくくなるとき。
- シェルが描けない種類が AST に増え、各シェルで同じ手当てを繰り返すようになったとき。

## Consequences

**良い結果**

- どのシェルも、同じ AST の型から描く。HTML を読む処理は、すべてなくなる。macOS の `NSAttributedString` の変換と文字列の置き換え、Windows の AngleSharp、iOS の `ShownParagraphs`、Android の `fromHtml` と JSON の読み取りである。
- AST に種類が増えると、型を作り直したときに、どのシェルのコンパイルもそれを知らせる。ADR 025 が守ろうとした「すべての種類を表示する」保証を、シェルの側でも持てる。
- iOS と macOS は、同じ Swift の `Node` を受け取る。AST から表示を決める部分を、2つで共有できるようになる。
- 境界を通る量が減る。bincode の AST は、JSON の約 0.75 倍である。
- journal の DB から `ast` の列がなくなる。MCP の応答も小さくなる。
- 解析のオプションの正本が flatmarkdown の1か所になる。client のテストにある写しと、すでに起きているずれ（`client-native-bridge/src/tests/html.rs`）がなくなる。

**悪い結果**

- flatmarkdown の型を変えるので、版を 0.8 に上げる。`SPEC_AST.md`、`ast2html`、`ast2react`、npm のパッケージも直す。flatmarkdown の外の利用者には、破壊的な変更になる。
- 描く処理を、Swift、Kotlin、C# のシェルで書き直す。今の HTML から描く処理は、捨てることになる。macOS と Windows は、とくに書き直す量が多い。
- AST の種類が増えるたびに、すべてのシェルで描く処理を足す必要がある。今は Rust の `ast_to_html` を1か所直せば済んでいる。コンパイラが書き落としを知らせるが、手間は増える。
- journal の DB から列を消すので、移行が要る。
- client の core と journal の両方が、本文を解析する。同じ本文を2回解析するが、どちらも速い（1項目 5.2µs）。

**中立的な結果**

- REST は、todo と memoview のために AST を送り続ける。ただし、DB から読むのではなく、応答を作るときに本文から作る。
- Web のアウトライナーと Emacs は Rust の `Node` を直接使うので、型の生成とは関係がない。Web は、これまでどおり `ast2html` で HTML を作る。
- ADR 025 は、この ADR で置き換えられる。ADR 028 と ADR 030 は、HTML と AST の JSON を読む部分だけが置き換えられる。ADR 027 は、コピーの AST を journal から受け取る部分だけが置き換えられる。
- 作業は、flatmarkdown、client、journal の順に進める。
