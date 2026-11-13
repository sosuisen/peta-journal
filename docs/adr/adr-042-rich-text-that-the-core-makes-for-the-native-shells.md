# ADR 042: Rich Text That the Core Makes for the Native Shells

## Status

Accepted

## Context

HTML とネイティブの OS では、文字を描くモデルが違う。HTML の DOM は入れ子の木であり、AST の入れ子をそのまま要素の入れ子に写せる。ネイティブのリッチテキストは、平らな文字列の範囲に見た目を付けるモデルを基本とする。macOS と iOS の `NSAttributedString` と `AttributedString`、Android の `Spanned` と `AnnotatedString` がそうである。Windows の `RichTextBlock` は文字の入れ子を持てるが、段落の並びより上の構造（リスト、引用、表）は持たない。

ADR 041 で、core は本文を解析して、ビューモデルの行（`Row.ast`）に型のある AST（`flatmarkdown_ast::Node`）を入れることにした。ネイティブのシェルは、Swift、Kotlin、C# に生成した `Node` の型から、OS の部品で項目を描く。HTML を経由する道で起きた「種類の書き落としに、コンパイラが気づかない」問題は、これで消えた。

4 つのシェルを AST から描くように切り替えたところ、どのシェルにも同じ変換ができた（2026-11-14）。シェルは AST をそのまま描かない。入れ子の木を平らにして、「表示する部品」に直してから描く。平らにすることは、2 種類ある。

1 つ目は、文字の入れ子を、見た目付きの文字の並びにすることである。4 つのシェルのすべてがこれをする。AST では、`**太字の中に *斜体* がある**` は次の木である。

```
strong
├─ text "太字の中に "
├─ emph
│  └─ text "斜体"
└─ text " がある"
```

ネイティブの文字表示は、この木をそのまま受け取らない。SwiftUI の `AttributedString`、Compose の `AnnotatedString`、macOS の `NSAttributedString` は、「文字と、その見た目」の並びを受け取る。Windows の Run も、見た目を付けた文字の並びとして書く。そのため、シェルは木を次の並びに直す。

```
「太字の中に 」太字 ／ 「斜体」太字＋斜体 ／ 「 がある」太字
```

リンク、脚注の `[1]`、ハッシュタグも、この段階で、見た目とリンク先の付いた文字になる。リストの印、脚注の番号、アラートの名前、リンクの行き先もここで決まる。

2 つ目は、表と画像を、文字から外に出すことである。これをするのは、SwiftUI と Compose のシェル（iOS と Android）だけである。SwiftUI の `Text` と Compose の `Text` は、文字の中に表や画像を入れられない。そのため、表、コードブロック、画像を、独立した部品として文字から分け、縦に並べる。それらを中に持つリストや引用も、項目ごとに分ける。たとえば、リストの 2 つ目の項目の中に画像があると、次のようになる。

```
AST（入れ子）                    表示する部品（縦に並べる）
list                            [文]  • 1つ目
├─ item: 1つ目                  [画像] picture.png
└─ item: 2つ目                  [文]  …続きの文
   └─ paragraph
      ├─ 画像
      └─ 続きの文
```

iOS と Android は、この部品を `ShownBlock` と呼ぶ。部品の種類は、文、コードブロック、表、画像、リスト、引用である。

macOS の `NSTextView` は、文字の中に表（`NSTextTable`）と画像（`NSTextAttachment`）を入れられる。Windows の `RichTextBlock` は、画像は入れられる（`InlineUIContainer`）が、表は入れられない。そのため、この 2 つのシェルは 2 つ目をせず、1 つのテキストに書く。Windows は、表を行の文字で出す。

どのシェルも、同じ決まりを自分の言語で書いている。行数は、iOS が約 520 行、Android が 391 行、macOS が 360 行、Windows が 417 行である。この重複には、4 つの問題がある。

- 決まりを変えるたびに、4 つの言語で同じ変更が要る。
- テストも 4 か所に要る。
- AST の種類が増えると、4 つのシェルのビルドが止まる。
- シェルによって、できることがずれる。画像を絵として出すのは iOS と Android だけで、macOS と Windows は代わりの文字を出す。リンクも、iOS と Android はアプリの中で開き、macOS と Windows はブラウザで開く。2026-11-14 に、ユーザーが次の 2 つを決めた。
  - 画像は、どのシェルでも絵として出す。
  - リンクは、アプリの中でそのページへ移る。macOS に限り、Shift-クリックでブラウザで開く。

  macOS は、この ADR を書く間に、Alternative 2 の作業（PR #464）で、この 2 つの形になった。

2 つ目の分け方は、描く力の弱い道具（SwiftUI と Compose）に合わせれば、1 つに決まる。分けた部品は、macOS では順につなげば 1 つのテキストに戻せる。Windows では、表の部品だけを別の部品（`Grid` など）で描く。したがって、1 つ目と 2 つ目は、どのシェルでも同じ形にできる。

ADR 041 の Alternative 2 は、「AST とは別に、表示用の型を作る」案を採らなかった。理由は、同じ本文の木を 2 つの型と 2 つの仕様で持つことになり、AST を表示に使える形に直せば足りる、と考えたからである。しかし、シェルが描くのに要るのは木ではなく、平らにした部品であった。どのシェルもその変換を書いたことで、それがはっきりした。

AST が要らなくなるわけではない。Web 版（`outliner-web`）は、`Row.ast` を `flatmarkdown-ast2html` で HTML にして描く。REST の todo と memoview、コピーの HTML も、AST から作る。

## Decision

core が、AST から「表示する部品」を作り、ネイティブのシェルに渡す。この部品の型を `RichText` と呼ぶ。シェルは、`RichText` を OS の部品で描くだけにする。AST を読んで平らにする処理は、シェルから消す。ADR 041 のうち、「ネイティブのシェルは `Row.ast` から描く」という部分を、この ADR で置き換える。AST を core が本文から作ること、Web 版が AST から描くことは変えない。

`RichText` の型は、`client-core` に置く。iOS と Android の `ShownBlock` を元にし、次の形にする。

- `RichText` は、部品（`RichBlock`）の並びである。
- `RichBlock` は、文（段落の並び）、コードブロック（行のまま）、表（行と、セルごとの段落）、画像、リスト、引用のいずれかである。リストと引用は、中に独立した部品を持つときだけ分ける。持たなければ、文の段落になる。
- 段落（`RichParagraph`）は、見出しのレベル、リストの深さと印、引用の深さ、区切り線かどうか、文字の並び（`RichRun` の並び）を持つ。
- `RichRun` は、文字列、見た目、リンク先を持つ。見た目は、太字、斜体、取り消し線、下線、マーカー、コード、上付き、下付き、隠し（スポイラー）の組み合わせである。
- 画像は、Markdown フォルダーの下のパス（`assets/picture.png`）、代わりの文字、ページが求める幅と高さを持つ。Web の画像は絵にせず、そのアドレスへのリンクにする。

リンク先も、core が決めて型で渡す。種類は、アプリの中のページ（`/pages/notes` のような URL）、Web のアドレス、assets のファイルの 3 つである。ウィキリンク、ハッシュタグ、`.md` への相対リンクは、アプリの中のページになる。ハッシュタグのページは、設定のタグのフォルダーの下にある。それ以外のリンクは、どこにも行かない文字にする。

core は、行の本文の表示を、トレイト（`BodyView`）を通して作る。実装は 2 つ置く。

- AST の実装は `Node` を返す。Web 版が使う。
- RichText の実装は `RichText` を返す。ネイティブの 4 つのシェルが使う。

core とビューモデルは、このトレイトを型引数に取る。Web 版は AST の実装を使い、`client-native-bridge` は RichText の実装を使う。ネイティブの境界には、AST を通さない。core は、本文が変わったときだけ、行の本文の表示を作り直す。

facet の型の生成が、型引数を持つビューモデルを書き出せることを、最初に確かめる。書き出せないときは、core の起動時に実装を渡し、行の本文を `Node` と `RichText` の enum にする。

各シェルは、`RichText` を次のように描く。

- iOS（SwiftUI）と Android（Compose）は、部品ごとにビューを作り、縦に並べる。
- macOS（AppKit）は、部品を順につないで 1 つの `NSAttributedString` にする。表は `NSTextTable` に、画像は `NSTextAttachment` にする。
- Windows（WinUI）は、文と画像を `RichTextBlock` に書く。画像は `InlineUIContainer` に入れ、表だけを別の部品で描く。

リンクと画像は、すべてのシェルで同じ決まりにする。

- ページへのリンクをクリックまたはタッチすると、アプリの中でそのページへ移る。macOS では、Shift-クリックで、journal サーバーの Web 版をブラウザで開く。
- Web のアドレスは、ブラウザで開く。assets のファイルは、OS の既定のアプリで開く。
- 画像は、どのシェルでも絵として出す。iOS と Android はアプリ内フォルダーから、macOS と Windows は journal サーバーの `/assets/` から読む。

移行は、ADR 041 と同じく段階に分ける。

1. facet の生成を確かめる。トレイトと 2 つの実装を作り、Rust でテストする。
2. シェルを 1 つずつ切り替える。順は、iOS、macOS、Android、Windows とする。
3. Android と Windows は、ユーザーが実機で確かめてから先へ進む。

## Alternatives

### Alternative 1: 今のまま、各シェルが AST を平らにする

ADR 041 のままである。core は変わらない。しかし、同じ決まりを 4 つの言語で持ち続ける。画像とリンクの扱いをそろえるにも、4 か所を直すことになる。Context の問題が残るので、採らない。

### Alternative 2: 同じ言語のシェルの間でだけ、変換を共有する

Swift のパッケージ（`peta-shown`）を作り、iOS と macOS で共有する。この案は、この ADR を書く間に作り、2026-11-14 に main に入った（PR #464）。core と境界の型は変わらない。しかし、まとまるのは 4 つのうち 2 つだけで、Kotlin と C# には同じ決まりが残る。リンクの URL を作る処理を、各アプリの core から注入する仕組みも要る。採らない。

### Alternative 3: 行に `ast` と `rich_text` の両方を入れる

トレイトを使わず、`Row` に 2 つの欄を持たせる。作りは簡単である。しかし、ネイティブの境界を、使わない AST が通り続ける。ネイティブ用に `Node` の型を生成し続けるので、AST の種類が増えると、シェルの生成物も変わる。採らない。

### Alternative 4: 実装を実行時に注入し、行の本文を enum にする

core の起動時に、AST か RichText の実装を渡す。行の本文は、`Node` と `RichText` の enum になる。型引数が要らないので、facet の生成で困らない。しかし、各シェルは、自分には届かない `Node` の場合も扱うことになる。`Node` の型も生成され続ける。Decision では、facet の生成が型引数を書き出せないときの代わりとして残す。

### Alternative 5: シェルが、AST を RichText に変える関数を呼ぶ

core は今のまま `Row.ast` を渡す。シェルは、描く前に、境界の関数で AST を `RichText` に変える。core を変えずに済む。しかし、AST が境界を 2 度通る。変換の結果を覚えておく処理も、各シェルに要る。採らない。

### Alternative 6: `RichText` を独立したクレートに置く

`client-core` の外に、`client-rich-text` のようなクレートを作る。ほかの道具からも使える。しかし、リンクの URL とタグのフォルダーは core が持っている。今は、core のほかに使う人がいない。採らない。

## Confidence Level

中くらいから高い。

確信がある点は、次のとおりである。4 つのシェルが、同じ変換を自分の言語で書いたことは、コードで確かめた。部品への分け方は、iOS と Android で同じであり、すでに動いている。macOS が部品を 1 つのテキストにつなげることは、`NSTextTable` と `NSTextAttachment` で描けるので、無理がない。リンクの URL を作る処理は、もう Rust にある（`client-page-links`）。

確かめていない点は、次のとおりである。

- facet の型の生成が、型引数を持つビューモデルを、Swift、Kotlin、C# に書き出せるか。書き出せないときは、Alternative 4（実行時の注入と enum）に切り替える。
- Windows の `RichTextBlock` で、画像を `InlineUIContainer` に入れ、表を別の部品で描く形が、編集との切り替えや選択とぶつからないか。
- macOS と Windows で、journal サーバーから読んだ画像の大きさを、付箋の幅に合わせて決められるか。

次のときは、この決定を見直す。

- あるシェルが、`RichText` の部品の分け方では描けないものを求めたとき。たとえば、文字の中に表を描けるシェルが、それを生かした見た目を求めたとき。
- ネイティブのシェルが、AST そのものを要るようになったとき。たとえば、シェルの側で本文を編集する部品が、構造を求めたとき。
- 新しいシェルが、HTML のように入れ子のモデルで描くとき。そのシェルは、AST の実装を使えばよい。両方を使い分ける必要が出たら、トレイトの形を見直す。

## Consequences

良い結果:

- AST を平らにする決まり（リストの印、脚注の番号、アラートの名前、リンクの行き先、画像の幅と高さ、部品の分け方）が、Rust の 1 か所になる。テストも Rust の 1 か所で済む。
- 4 つのシェルの、AST を読む処理（合わせて約 1700 行）が消える。シェルに残るのは、`RichText` を OS の部品で描く処理だけである。
- AST の種類が増えても、直すのは core の RichText の実装だけである。シェルは、`RichText` の部品が変わらない限り、何も変わらない。
- ネイティブの境界を AST が通らない。ネイティブのシェルには、`Node` の型を生成しない。
- 画像とリンクの扱いが、4 つのシェルでそろう。macOS と Windows も、画像を絵として出す。
- 変換は Rust で 1 回だけになり、シェルでの変換はなくなる。core は、ネイティブでは `RichText` だけを覚えておく。

悪い結果:

- core とビューモデルが型引数を取るので、core の型が少し複雑になる。facet の生成が型引数を書き出せないときは、`Node` と `RichText` の enum になる（Alternative 4）。その場合、シェルには届かない `Node` の場合が残る。
- `RichText` の部品の分け方は、SwiftUI と Compose に合わせて決まる。文字の中に表を描ける macOS は、その力を部品の形からは生かせない。
- 4 つのシェルを切り替える作業が要る。Android と Windows は、ユーザーが実機で確かめる。
- `RichText` は、PetaJournal の中の型である。flatmarkdown の外の利用者には、公開しない。

中立的な結果:

- Web 版は、今と同じく `Row.ast` から `flatmarkdown-ast2html` で描く。REST の todo と memoview、コピーの HTML も、AST から作る。
- Swift のパッケージ `peta-shown`（Alternative 2、PR #464）は、iOS と macOS を `RichText` に切り替えるときに消す。macOS で画像を journal サーバーから読む処理と、リンクの決まり（クリックはアプリの中、Shift-クリックはブラウザ）は、この ADR の形でも使う。
- ADR 041 の Status は Accepted のままとし、ネイティブのシェルが何から描くかについては、この ADR が置き換えると書き足す。ADR 041 の Alternative 2 は、この ADR で見直した。
- `docs/architecture.svg` を、この形に合わせて直す。
