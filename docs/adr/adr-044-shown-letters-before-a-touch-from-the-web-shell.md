# ADR 044: Shown Letters Before a Touch from the Web Shell

## Status

Accepted

## Context

ADR 043は、表示した文字に触れた位置を、どのシェルも`ShownTextTouched`でcoreに送ると決めた。送るのは、表示の文字のうち、触れた位置より前にある文字の数である。数え方は`RichParagraph.start`に合わせる。coreは、表示の文字をMarkdownと照らし合わせ（`markdown_place`）、その位置をMarkdownの位置に直す。

iOS、Android、macOS、Windowsは、coreが作った`RichText`から表示を作る。そのため、段落の`start`に、段落の中で触れた文字の数を足せば、coreと同じ数え方になる。リストの印や区切り線のように、シェルが足す飾りは数えない。

Web版は、`RichText`を使わない。coreが渡すASTから、`flatmarkdown-ast2html`でHTMLを作って表示する（ADR 026、ADR 041）。そのHTMLの文字は、coreの「表示の文字」と並びが合わない。

- タグの間に改行がある。
- 詰めて書いたリストでは、`<li>`の中に、段落の文字と入れ子のリストが一緒に入る。要素の単位では、段落の`start`を付けられない。
- アラートの題、脚注、HTMLブロックなどでも、並びがずれるおそれがある。

2026-11-14に、Web版の直し方として3つを比べた。HTMLのブロックに`start`を付ける案（`ast2html`に設定を足す1a、ページに入れたあとで付ける1b）、Web版も`RichText`から表示を作る案、Web版はこのままにする案である。1bを選んだが、上の並びのずれが見つかった。

一方、coreの`markdown_place`は、段落の数を前提にしていない。表示の文字の列とMarkdownの文字の列を、編集距離で照らし合わせる。表示の側にしかない文字があっても、飛ばすように作られている（`SHOWN_LEFT_OUT`）。

## Decision

Web版は、表示した文字に触れた位置を、文字の数ではなく文字の列でcoreに送る。送るのは、ページの上でクリックより前にある、項目の要素の文字である（`Event::ShownTextTouchedAfter { item_id, before }`）。DOMのテキストノードの文字を、そのままつなげたものを送る。

coreは、その文字の列を`markdown_place`でMarkdownと照らし合わせ、カーソルの位置を決める。Markdownにない文字（タグの間の改行など）は飛ばす。決めた位置は、ADR 043と同じく書記素の境に寄せる。

`flatmarkdown-ast2html`は変えない。Web版は、HTMLと`RichText`の並びを揃えない。

`RichText`から表示を作るシェル（iOS、Android、macOS、Windows）は、ADR 043のとおり`ShownTextTouched`で文字の数を送る。数で送れるシェルは数で送る。これは、照らし合わせる文字の列を、coreが自分で作れるからである。文字の列で送るのは、`RichText`を描かないシェルだけとする。

ADR 043のDecisionのうち「どのシェルも`ShownTextTouched`で送る」は、Web版については、このADRで置き換える。「シェルは、表示の文字の位置をMarkdownの位置として使わない。Markdownの位置に直すのはcoreである」は、そのまま守る。

## Alternatives

### Alternative 1: HTMLのブロックに`start`を付け、ast2htmlに設定を足す（1a）

`ast_to_html`に、段落やコードブロックの要素へ`data-start`を付ける設定を足す。既定では付けないので、ほかの利用者の出力は変わらない。Web版は、クリックした要素の`data-start`に、要素の中で触れた文字の数を足して`ShownTextTouched`で送る。

長所は、全シェルが同じイベントで送れることである。短所は、「表示の文字」の数え方が、flatmarkdownとcoreの`RichText`の2か所に書かれることである。flatmarkdownのリリースも要る。さらに、詰めたリストの`<li>`のように、要素の単位では`start`を付けられない所が残る。そのため選ばなかった。

### Alternative 2: ページに入れたHTMLへ、Web版が`start`を割り当てる（1b）

`ast2html`は変えない。ページに入れたあとで、ブロックの要素を順にたどり、`RichText`の段落の`start`を順に割り当てる。

長所は、数え方がcoreの1か所に残ることである。短所は、HTMLのブロックと`RichText`の段落が1対1に並ぶ必要があることである。詰めたリスト、アラートの題、脚注などで並びがずれる。ずれるたびに、割り当ての規則をWeb版に足すことになる。そのため、一度選んだあとで取りやめた。

### Alternative 3: Web版も`RichText`から表示を作る

Web版も、ネイティブのシェルと同じく`RichText`を描く。数え方は確実に揃い、`ShownTextTouched`で送れる。

短所は、Web版の表示をほぼ作り直すことである。見た目も変わる。この問題のためだけには、変更が大きすぎる。そのため選ばなかった。

### Alternative 4: Web版はこのままにする

Web版は、DOMで数えた位置を、そのままMarkdownの位置として使い続ける。変更はない。

短所は、リンクや強調の記号の分だけ、カーソルがずれる不具合が残ることである。ADR 043の「シェルは、表示の文字の位置をMarkdownの位置として使わない」にも反する。そのため選ばなかった。

## Confidence Level

確信度は中程度から高い。

coreの照らし合わせは、iOSとAndroidで、表示の文字とMarkdownを並べるのに既に使っている。Web版のE2Eのテストでも、強調、リンク、リストの項目で、正しい位置に置かれることを確かめた。ただし、DOMの文字の列で試したのは、まだこの数例だけである。

次のときは、見直す。

- DOMにしかない文字が多く、照らし合わせがずれる例が見つかったとき。例えば、脚注の番号や、長いHTMLブロックである。飛ばす費用（`SHOWN_LEFT_OUT`）の調整で直らなければ、別の形を考える。
- Web版が`RichText`から表示を作るようになったとき。そのときは、ほかのシェルと同じく`ShownTextTouched`で数を送ればよく、このADRは要らなくなる。
- 長い項目で、照らし合わせが遅くなったとき。照らし合わせは、文字の列の長さとMarkdownの長さの積に比例する。クリックのたびに1回なので、今は問題にならない。

## Consequences

良い結果：

- Web版で、リンクや強調の記号の分だけカーソルがずれる不具合が直る。
- 表示の文字をMarkdownの位置に直す処理は、coreの`markdown_place`の1か所に残る。Web版に、Markdownの解析や数え方を持たせずに済む。
- `flatmarkdown-ast2html`は変わらない。flatmarkdownのリリースは要らない。HTMLと`RichText`の並びを揃える規則も要らない。

悪い結果：

- 触れた位置を伝えるイベントが2つになる。`ShownTextTouched`（数）と`ShownTextTouchedAfter`（文字の列）である。新しいシェルを作るときは、どちらを使うかを選ぶ必要がある。`RichText`を描くなら前者、そうでなければ後者である。
- Web版は、`RichText`を描くシェルより、照らし合わせに頼る部分が多い。DOMにしかない文字が多い項目では、位置がずれるおそれがある。
- Web版は、クリックのたびに、項目の要素の文字をクリックの位置までつなげて送る。長い項目では、送る文字が多くなる。

中立的な結果：

- 文字の脇のクリックは、今までどおり項目の先頭に置く。リンクのクリックは、編集に入らずにリンクを開く。
- テキストの選択のドラッグは、coreが置いたカーソルの位置から始まる。
- 同じ照らし合わせを、ほかの描き方をするシェルでも使える。例えば、Emacsが表示した文字に触れた位置を送るようになったときである。
