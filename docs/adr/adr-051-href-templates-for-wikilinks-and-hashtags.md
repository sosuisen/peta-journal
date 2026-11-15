# ADR 051: Href Templates for Wikilinks and Hashtags in flatmarkdown-ast2html

## Status

Accepted

## Context

`flatmarkdown-ast2html`の`ast_to_html`は、ASTだけを受け取り、出力を変える手段を持たない。wikilinkは`<a href="<ページ名>" data-wikilink="true">`、hashtagは`<a href="<タグ名>" class="hashtag">`になる。`href`には、ページ名やタグ名だけが入る。

ADR 026は、Webのシェルがこの出力から項目を描くと決めた。そのとき、`ast_to_html`は変えず、オプションも足さないことにした。足りないことはWebのシェルが補う。その1つがリンクの`href`である。描画のあと、`item_view.rs`の`point_links`が、DOMの`href`に`/pages/`とタグ用ディレクトリを付ける。ADR 026は、`ast_to_html`にリンクのオプションを足すかどうかを、先送りにした。

この補い方には、無理がある。タグ用ディレクトリは、設定で変えられる。変わると、書き換えがもう一度走る。そのときの`href`には、もう接頭辞が付いている。二重に付けないために、元の名前を`data-page`という属性に取っておいている。この属性は、後から書き換えるという方法のためだけにある。flatmarkdown側を変えずにこの属性を消す方法は、ない。HTMLの文字列は設定で変わらないので、leptosはDOMを入れ直さない。描画の前にASTを書き換えると、表示の文字か`class="hashtag"`の目印が変わってしまう。

同じ問題は、PetaJournalの外にもある。sosuisen.netは、`fm2html`で記事をHTMLにする。hashtagを`/tags/<タグ>`のページへつなぐため、描画したHTMLの文字列を書き換えている。

一方、hrefをそのままの名前で使うアプリもある。memoviewは、wasm版の`renderToHtml`で描く。クリックを横取りし、`href`をタグ名やページ名として読み、C#側に渡す。C#側は、クリックのたびにサーバーからタグ用ディレクトリを聞き、URLを組み立てる。「HTMLとしてコピー」（ADR 027）は、描画の前にASTのリンクを普通の`Link`に置き換え、URLをパーセント符号化する。どちらも、今の出力のままで困っていない。

## Decision

`flatmarkdown-ast2html`に、wikilinkとhashtagの`href`をテンプレートで決める設定を足す。設定は`HtmlOptions`という構造体で、`wikilink_href`と`hashtag_href`の2つの文字列を持つ。テンプレートの中の`{}`は、すべて置き換える。wikilinkならノードの`url`、hashtagならタグ名が入る。たとえば`/pages/tags/{}`は、`<a href="/pages/tags/Rust" class="hashtag">#Rust</a>`になる。テンプレートの後ろにも文字を置けるので、`/tags/{}.html`のような形も作れる。

設定を受け取る関数として、`ast_to_html_with(node, options)`を足す。`ast_to_html`は残し、既定の設定で`ast_to_html_with`を呼ぶ。既定のテンプレートは、どちらも`{}`である。既定の設定では、今と同じHTMLになる。そのため、既存の呼び出しは変えなくてよい。

`href`は、テンプレートに名前を入れたあとで、まとめてHTMLのエスケープをかける。表示の文字（`#Rust`やwikilinkの中身）と、目印の`data-wikilink`と`class="hashtag"`は変えない。

パーセント符号化はしない。タグ名には、URLで特別な意味を持つ文字が入らない。日本語などは、リンクを開くときにブラウザが符号化する。wikilinkのページ名には`?`や`%`が入りうるが、今のWebのシェルも符号化していないので、この決定で変わることはない。どの文字を符号化するかはアプリごとに違うので、符号化が要るアプリが出たら、そのときに設定を足す。

wasm版（npmの`flatmarkdown-ast2html`）には、設定を足さない。wasm版を使うmemoviewは、`href`を名前として読むので、設定を使わない。既定の設定では、Rust版とwasm版は同じHTMLを出す（ADR 023）。wasm版で設定が要るアプリが出たら、そのときに足す。

3つのクレートは、そろって`0.9.0`に上げる（ADR 020）。公開APIが増えるが、既存のAPIは変わらない。

Webのシェルは、`item_html`で`ast_to_html_with`を使う。wikilinkには`/pages/{}`、hashtagには`/pages/<タグ用ディレクトリ>{}`を渡す。タグ用ディレクトリが変わったら、HTMLを作り直す。描画のあとに`href`を書き換える`point_links`と、`data-page`の属性は消す。ADR 026が決めた補い方のうち、リンクの`href`の項目は、この決定で置き換える。画像の大きさとコードブロックの色付けは、ADR 026のまま残す。クリックの横取りは変えない。目印で選び、`href`をURLとして開くので、今のまま動く。

「HTMLとしてコピー」（ADR 027）とmemoviewは、変えない。コピーはURLをパーセント符号化する必要があり、テンプレートでは置き換えられない。memoviewは、`href`を名前として読む。

## Alternatives

### Alternative 1: 今のまま、アプリが後から`href`を書き換える

`ast_to_html`は変えない。Webのシェルは`point_links`と`data-page`を、sosuisen.netはHTMLの文字列の書き換えを続ける。

- 長所: flatmarkdownの公開APIが増えない。どのアプリも変えなくてよい。
- 短所: 後から書き換えるための工夫が、アプリごとに残る。Webのシェルの`data-page`は、この方法のためだけにある。

ADR 026が先送りした問題が、2つのアプリで同じ形で現れた。そのため、選ばなかった。

### Alternative 2: 接頭辞だけを設定する

テンプレートではなく、名前の前に付ける文字列だけを渡す。`/pages/tags/`を渡すと、`/pages/tags/Rust`になる。

- 長所: `{}`の決まりが要らず、より単純である。
- 短所: 名前の後ろに文字を付けられない。`/tags/Rust.html`や`/search?tag=Rust&lang=ja`は作れない。

今のPetaJournalとsosuisen.netは接頭辞で足りる。それでも、後ろに文字が要るサイトはよくある。そのため、`{}`の決まりが1つ増えるだけのテンプレートを選んだ。

### Alternative 3: 名前からURLを作る関数を渡す

`Fn(&str) -> String`を設定として渡す。

- 長所: パーセント符号化も含め、どんなURLでも作れる。
- 短所: 関数はJSONで渡せず、wasm版に同じ設定を足せない。Rust版とwasm版で、できることが違ってしまう（ADR 023）。

今は符号化が要るアプリがないので、選ばなかった。

### Alternative 4: 名前を`data-`の属性に出す

`href`はそのままにして、`data-tag="Rust"`のように名前を別の属性にも出す。

- 長所: memoviewのように名前を読むアプリでは、`href`を名前として読む約束が要らなくなる。
- 短所: `href`は今のままなので、Ctrl+クリックや中クリックでは、行き先がずれたままである。Webのシェルの書き換えは消えない。

`href`を正しくするという目的に合わないので、選ばなかった。

## Confidence Level

確信度は高い。既定の設定では今と同じHTMLになるので、既存のアプリへの影響はない。Webのシェルとsosuisen.netの書き換えが消えることは、コードを読んで確かめた。

次のときは、見直す。

- パーセント符号化が要るアプリが出たとき。たとえば、ページ名に`?`や`%`を使うことが増えたときや、「HTMLとしてコピー」もテンプレートで描きたくなったときである。そのときは、テンプレートに符号化の設定を足すか、Alternative 3の関数を考え直す。
- wasm版を使うアプリが、リンク先を決めたくなったとき。そのときは、`HtmlOptions`をJSONで受け取る関数をwasm版に足す。
- テンプレートで決めたい部分が増えたとき。たとえば、画像の大きさや、wikilinkとhashtag以外のリンクである。設定が増えるようなら、`HtmlOptions`の形を見直す。

## Consequences

良い結果：

- Webのシェルから、`point_links`と`data-page`の属性が消える。リンクの`href`は、描画の時点で正しくなる。
- sosuisen.netの`fm2html`から、hashtagの`href`を書き換える処理が消える。
- PetaJournalの外のサイトでも、`/tags/{}.html`のような形でリンク先を決められる。

悪い結果：

- flatmarkdown-ast2htmlの公開APIが、`HtmlOptions`と`ast_to_html_with`の2つ増える。設定を足すたびに、すべての描画関数へ設定を渡す必要がある。
- タグ用ディレクトリを変えると、Webのシェルはすべての項目のHTMLを作り直す。今は`href`だけを書き換えている。めったに変えない設定なので、受け入れる。
- Rust版とwasm版で、使える設定が違う。既定のHTMLは同じだが、wasm版ではリンク先を決められない。

中立的な結果：

- 3つのクレートの版が`0.9.0`になる。PetaJournalとsosuisen.netは、版を上げてから、新しい関数に切り替える。
- ADR 026の決定のうち、リンクの`href`を描画のあとに書き換える項目は、このADRで置き換わる。画像の大きさとコードブロックの色付けは、ADR 026のまま残る。
- 「HTMLとしてコピー」（ADR 027）とmemoviewは、今のまま変わらない。
