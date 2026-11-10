# ADR 020: HTML Renderer as the flatmarkdown-ast2html Crate

## Status

Accepted

## Context

Flat MarkdownのASTをHTMLにするコードが、2つある。1つはRustで、`flatmarkdown`クレートの`html.rs`にある`node_to_html`である。もう1つはTypeScriptで、npmパッケージの`flatmarkdown-ast2html`である。どちらも同じ仕事をするが、別々に書かれている。

2つの出力は、すでにずれている。Rustの側は、画像を`embed`ノードとして読む。TypeScriptの側は`image`ノードを読み、`embed`の処理を持たない。ハッシュタグの属性、alertのタグ、表の`<thead>`と`<tbody>`の有無も違う。ASTの仕様が変わるたびに、2つを同じように直す必要があるが、それを確かめる仕組みはない。

2026年11月、Rustの`node_to_html`を正本にすると決めた。HTMLを作る実装を1つにし、TypeScriptの側をいずれなくすためである。

しかし、今の`node_to_html`は、ASTだけを持っている側からは使えない。`node_to_html`は公開されていない関数で、外から呼べるのは`body_to_html`だけである。`body_to_html`はMarkdownのテキストを受け取り、`comrak`で解析してからHTMLにする。そのため、`node_to_html`を使うには、`flatmarkdown`クレートの全体と、パーサーである`comrak`に依存することになる。

ASTだけを持っている側は、実際にある。PetaJournalのサーバーは、項目ごとのASTをDBに保存し、APIではASTを返す。HTMLは保存していない。付箋紙アプリとtodoアプリは、このASTを受け取り、TypeScriptの`flatmarkdown-ast2html`でHTMLにしている。これらのアプリは、Markdownを解析し直す必要がない。

`node_to_html`そのものは、パーサーに依存していない。使っているのは`flatmarkdown-ast`クレートの`Node`型だけである。`html.rs`の中でパーサーに触れるのは、`body_to_html`と`body_to_html_with`の2つの関数だけである。ADR 012は、同じ理由でASTの型を`flatmarkdown-ast`クレートに分けた。ASTを読むだけの側が、`comrak`に依存しないようにするためである。

## Decision

HTMLを作る機能を、`flatmarkdown`クレートからなくす。ASTをHTMLにするコードは、`flatmarkdown-ast2html`という名前の独立したクレートに移し、crates.ioに公開する。名前は、同じ仕事をするnpmパッケージ`flatmarkdown-ast2html`にそろえる。`flatmarkdown-ast`クレートが、npmの`flatmarkdown-ast`に名前をそろえたのと同じである。

クレートは、`flatmarkdown`のリポジトリ（`petajournal/flatmarkdown`）に置く。ワークスペースの3つ目のメンバーとして、`flatmarkdown-ast2html/`のフォルダを足す。ASTを作るコード、ASTを読む型、ASTをHTMLにするコード、ASTの仕様（`SPEC_AST.md`）が、1つのリポジトリに並ぶ。仕様を変えるときは、1つのPRでこれらをそろえて直す。

関数の名前の「ast」は、常に`flatmarkdown-ast`の型を指す。JSONの文字列は指さない。3つのクレートの関数を、次の形にそろえる。

- `flatmarkdown`の`body_to_ast`と`body_to_ast_with`は、`Node`を返す。今はJSONの文字列を返している。
- `flatmarkdown`の`page_to_ast`と`page_to_ast_with`は、`Page`を返す。今はJSONの文字列を返している。
- `flatmarkdown-ast2html`は、`ast_to_html(ast: &Node) -> String`の1つだけを公開する。今の`node_to_html`の名前を変えたものである。

こうすると、MarkdownからHTMLを作る側は、`ast_to_html(&body_to_ast(markdown))`と書ける。JSONの文字列は経由しない。

JSONが要る側のために、`flatmarkdown-ast`に`Node::to_json`と`Page::to_json`を足す。JSONを読む`parse`と`parse_page`の対である。ASTをDBに保存する側は、`body_to_ast_with(markdown, &opts).to_json()`と書く。JSONを持っている側は、`flatmarkdown_ast::parse`で`Node`にしてから`ast_to_html`に渡す。

`flatmarkdown`クレートからは、`html.rs`を消す。公開していた`body_to_html`と`body_to_html_with`も消す。`flatmarkdown`は、Markdownを解析してASTを作るだけのクレートになる。

これらは破壊的変更である。今はまだ破壊的変更をしてよい段階なので、互換のための関数は残さない。

`flatmarkdown`と`flatmarkdown-ast2html`は、互いに依存しない。どちらも`flatmarkdown-ast`だけに依存する。

3つのクレートの版は、常に同じにする。ADR 012は、`flatmarkdown`と`flatmarkdown-ast`の版を常に同じにすると決めた。この決まりを、3つ目のクレートにも広げる。1つしか変えていないときも、3つの版を上げて公開する。版は、今までどおり一番上の`Cargo.toml`に1回だけ書く。

HTMLのテストは、新しいクレートに集める。`Node`を入力にするテストを足す。Markdownを入力にする今のHTMLのテストも、新しいクレートの`tests/`へ移す。このテストは、ワークスペースの中の`flatmarkdown`を、開発時の依存としてパスで参照する。

使っている側は、次の版に上げるときに直す。PetaJournalのjournal-server、Web版のコピー（`shell-leptos`）、`fm-leptos`のテスト、そして`flatmarkdown-java`である。

このADRが決めるのは、ここまでである。次のことは、このADRでは決めない。

- 新しいクレートをwasmにして、JavaScriptから呼べるようにすること
- 付箋紙アプリとtodoアプリを、新しいクレートに切り替えること
- npmパッケージの`flatmarkdown-ast2html`を廃止すること

## Alternatives

### Alternative 1: `flatmarkdown-ast`クレートの中に置く

ASTをHTMLにする関数を、`flatmarkdown-ast`のモジュールとして公開する。

クレートは2つのままで済む。版の決まりも変えなくてよい。しかし、型を持つクレートに、HTMLを書く責務が混ざる。ASTを読むだけの側（年表を描く`timeline`など）は、使わないレンダラーも一緒に受け取る。HTMLの出力を直しただけでも、型のクレートが変わったように見える。そのため採らなかった。

### Alternative 2: `flatmarkdown`の中で公開するだけにする

クレートは分けず、`node_to_html`を`pub`にする。

変更は最も小さい。しかし、ASTだけを持っている側が、パーサーである`comrak`に依存することは変わらない。wasmにするときも、`comrak`と絵文字の表が入る。機能フラグでパーサーを外すことはできるが、使う側が毎回指定する必要がある。ADR 012が、型を`flatmarkdown`に入れなかったのと同じ理由である。そのため採らなかった。

### Alternative 3: TypeScriptの`flatmarkdown-ast2html`を正本にする

RustのHTML出力をTypeScriptの側に合わせ、TypeScriptの側を直し続ける。

付箋紙アプリとtodoアプリは、今のまま使える。しかし、Rustの側の実装はなくせない。Web版の画面は、コピーのときにRustでHTMLを作っている。2つの実装が残るので、ずれる余地も残る。そのため採らなかった。

### Alternative 4: 2つの実装を残し、同じ出力になることをテストで確かめる

同じASTを両方に渡し、HTMLが一致することを確かめるテストを足す。

どちらの側も、今の使い方を変えなくてよい。ずれは、テストが見つける。しかし、ASTの仕様を変えるたびに、2つの言語で同じ変更を書くことは変わらない。テストも、2つのリポジトリをまたぐものになる。そのため採らなかった。

### Alternative 5: 新しいリポジトリを作る

`flatmarkdown-ast2html`だけのリポジトリを作る。

リポジトリごとの役割は単純になる。しかし、ASTの仕様を変えるときに、変更が2つのリポジトリに分かれる。3つの版をそろえる決まりも、守りにくくなる。ADR 012のAlternative 4と同じ理由である。そのため採らなかった。

### Alternative 6: 新しいクレートの版を別にする

`flatmarkdown-ast2html`を0.1.0から始め、独立して版を上げる。

HTMLの出力だけを直したときに、ほかの2つの版を上げずに済む。しかし、どの版のASTを、どの版のレンダラーで読めるかを、別に管理する必要がある。ADR 012のAlternative 6と同じ理由である。そのため採らなかった。

### Alternative 7: `flatmarkdown`に`body_to_html`を残す

`body_to_html`と`body_to_html_with`を残し、中身は新しいクレートを呼ぶだけにする。

`flatmarkdown`を使っている側は、HTMLについては何も変えなくてよい。しかし、`flatmarkdown`が新しいクレートに依存し、解析とHTMLの2つの仕事を持ち続ける。HTMLを作る入り口も、2つのクレートに分かれる。使っている側は2か所だけで、今はまだ破壊的変更をしてよい段階である。そのため採らなかった。

### Alternative 8: `body_to_ast`はJSONのままにし、型を返す関数を別に足す

`body_to_ast`と`page_to_ast`は変えない。`Node`を返す`body_to_node`を足し、新しいクレートの関数は`node_to_html`にする。

今の使う側とテストを、ほとんど直さずに済む。しかし、「ast」がJSONを指し、型は「node」と呼ぶことになる。ADR 014で、ASTの正体は型になった。名前がそれと合わない。`page_to_ast`には、型を返す対の関数がないままになる。今はまだ破壊的変更をしてよい段階である。そのため採らなかった。

## Confidence Level

クレートを分けること自体の確信度は高い。`node_to_html`が使うのは`Node`型だけで、パーサーには触れていない。動かすコードは約300行である。ADR 012で、同じリポジトリに同じ形でクレートを足した実績もある。

関数の戻り値を変えることと、`body_to_html`を消すことの確信度も高い。`page_to_ast`は、すでに`Page`を組み立ててからJSONにしている。使っている側は、journal-serverの2か所、Web版のコピーの1か所、`fm-leptos`のテスト、`flatmarkdown-java`だけである。どれも1行で直せる。直す量が多いのは、`flatmarkdown`自身のテストである。JSONの文字列を受け取る形で書かれているので、JSONにする補助関数を通すように直す。

この決定は、後から変えることもできる。crates.ioに出した版は消せないが、`flatmarkdown`にHTMLの関数を戻すことは、いつでもできる。

確信度が低いのは、`ast_to_html`の形である。この関数は、出力を変える手段を持たない。TypeScriptの`flatmarkdown-ast2html`には、wikiリンクとハッシュタグのタグ名やURLの接頭辞を変えるオプションがある。付箋紙アプリとtodoアプリがこのオプションに頼っているかは、まだ調べていない。頼っているなら、切り替えのときに関数の形を見直す。

次のことが起きたら、この決定を見直す。

- ASTだけを持っている側が、新しいクレートを使わないと分かったとき。使う側がMarkdownから始める側だけなら、分けた意味が薄い。
- wasmにしたときの大きさが、TypeScriptの実装（約4KB）に比べて受け入れられないほど大きいと分かったとき。大きさは、まだ測っていない。
- 3つのクレートの版をそろえて公開する手間が、変更の頻度に見合わなくなったとき。

## Consequences

### 良い結果

ASTをHTMLにするRustのコードを、パーサーなしで使える。ASTだけを持っている側は、`flatmarkdown-ast2html`と`flatmarkdown-ast`に依存するだけで済む。`comrak`と絵文字の表は入らない。

`flatmarkdown`の仕事が1つになる。Markdownを解析してASTを作るだけである。HTMLの出力を直しても、パーサーのクレートのコードは変わらない。

「ast」という言葉の意味が1つになる。関数の名前の「ast」は、常に型を指す。`ast_to_html(&body_to_ast(markdown))`と、そのままつなげる。型を作ってからJSONにし、すぐ読み直す無駄は、使う側にも生まれない。

HTMLを作る実装を1つにする道が開く。新しいクレートをwasmにすれば、付箋紙アプリやtodoアプリも同じコードを使える。そうなれば、TypeScriptの`flatmarkdown-ast2html`をなくせる。

### 悪い結果

次の版は、破壊的変更を含む。`body_to_ast`と`page_to_ast`の戻り値が変わり、`body_to_html`がなくなる。使っている側は、版を上げるときに必ずコードを直す。直すまでは、古い版に固定しておく必要がある。

`flatmarkdown`自身のテストを、広く書き直す。テストはJSONの文字列を受け取る形で書かれている。変更は機械的だが、行数は多い。レビューでは、テストの主張が変わっていないことを確かめる必要がある。

公開するクレートが3つになる。公開の手順が1つ増える。1つしか変えていないときも、3つの版を上げる。

MarkdownからHTMLを作る側は、2つのクレートに依存し、2つの関数を呼ぶ。今は、1つのクレートの1つの関数で済んでいる。

npmパッケージの`flatmarkdown-ast2html`は、このADRのあとも残る。なくすまでの間、2つの実装のずれは続く。同じ名前のものがcrates.ioとnpmにあり、出力が違う状態になる。

### 中立的な結果

ADR 012の「2つのクレートの版は常に同じ」という決まりは、「3つのクレートの版は常に同じ」に変わる。`flatmarkdown`のワークスペースの`Cargo.toml`にあるコメントも、それに合わせて直す。

`flatmarkdown-java`は、`body_to_html`をJavaに公開している。次の版に上げるときに、Rustの側で2つの関数を呼ぶように直す。Javaの側の関数は変えなくてよい。

HTMLのテストは、`flatmarkdown`から`flatmarkdown-ast2html`へ移る。`flatmarkdown-ast2html`のテストは、開発時だけ`flatmarkdown`に依存する。公開するクレートの依存には入らない。

wasmにすること、付箋紙アプリとtodoアプリを切り替えること、npmパッケージをなくすことは、それぞれ別に決める。`ast_to_html`にオプションを足すかどうかも、切り替えのときに決める。
