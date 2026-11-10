# ADR 023: npm flatmarkdown-ast2html Built from the Rust Crate

## Status

Accepted

## Context

ADR 020で、ASTからHTMLを作る正本を、Rustのクレート`flatmarkdown-ast2html`と決めた。そのADRは、wasmにするか、付箋紙アプリを切り替えるか、npmのパッケージをどうするかを決めなかった。このADRは、その3つを決める。

npmにも、同じ名前のパッケージ`flatmarkdown-ast2html`がある。これはTypeScriptで書いた別の実装で、リポジトリ`petajournal/flatmarkdown-ast2html`にある。このパッケージを使うのは、付箋紙アプリ（`ext-desktop-app/stickies`）だけである。todoアプリは`flatmarkdown-ast2react`を使い、WebフロントエンドはRustのクレートを直接リンクしている。

付箋紙アプリは、C#とWebView2で作られている。ビルドのときにunpkgから約4KBの.jsを1つ取り、`Stickies.exe`に埋め込む。実行時には、その.jsを文書の`<script>`に書き込み、`FmAst2Html.renderToHtml(ast)`を呼ぶ。

TypeScriptの実装は、正本と同じHTMLを出さない。付箋紙アプリが使う0.1.3は、ハッシュタグを扱えない。最新の0.1.7でも、Rustが`embed`とするノードを`image`として扱うなどの違いが残る。実装が2つあるかぎり、ASTの仕様を変えるたびに両方を直す必要がある。

付箋紙アプリが正本のHTMLを得る方法として、4つを考えた。サーバーがHTMLを返す、ネイティブのDLLを呼ぶ、wasmにする、npmのバージョンを上げるだけにする、の4つである。

wasmについては、試作して測った。wasmは169KB（gzipで56KB）である。wasmをbase64で埋め込んだ.jsは、1つで234KBになり、今の.jsの約60倍である。ヘッドレスのEdge 157では、同期での初期化が約6ms、300件の変換が約11msだった。試作の.jsは、今と同じ`FmAst2Html.renderToHtml(ast)`の形で呼べる。

## Decision

npmのパッケージ`flatmarkdown-ast2html`を、Rustのクレートをwasmにしたものに作り替える。TypeScriptの実装は削除する。

wasmのコードは、リポジトリ`petajournal/flatmarkdown`に置く。ワークスペースの4つ目のメンバーとして、フォルダ`flatmarkdown-ast2html-wasm/`を作る。クレートの名前も`flatmarkdown-ast2html-wasm`とする。このクレートは、wasm-bindgenの薄い層であり、crates.ioには公開しない。npmのパッケージの名前は、`flatmarkdown-ast2html`のままとする。

リポジトリ`petajournal/flatmarkdown-ast2html`は、アーカイブする。READMEに、移動先を書く。

npmのバージョンは、クレートのバージョンと同じにする。最初のバージョンは0.6.0であり、0.1.7から飛ぶ。

配るファイルは、.jsを1つだけとする。wasmは、base64でその.jsに埋め込む。`<script>`で読み込むと、グローバルの`FmAst2Html`ができる。

`FmAst2Html.renderToHtml(ast)`は、JSONの文字列かオブジェクトを受け取り、HTMLの文字列を同期で返す。ASTとして読めない入力には、例外を投げる。

次のものは、なくす。これは破壊的な変更である。

- オプションの引数（wikilinkとハッシュタグのタグ名や属性の指定）
- ESモジュールとCommonJSのビルド
- TypeScriptの型定義
- ASTの型の再エクスポート

付箋紙アプリは、`FmAst2HtmlVersion`を0.6.0にする。バージョンを変えたときは、ビルドが.jsを取り直すようにする。CSSは、Rustが出すHTMLに合わせて直す。

npmへの公開は、人間が行う。手順は、flatmarkdownのREADMEの「Release」の節に足す。

## Alternatives

### Alternative 1: サーバーがHTMLを返す

journal-serverが、`ItemDto`にHTMLを足して返す。付箋紙アプリは、.jsを持たずに済む。しかし、サーバーが特定の画面のための表示を作ることになる。表示は各クライアントが決める、という今の分担が崩れる。そのため選ばなかった。

### Alternative 2: ネイティブのDLLをP/Invokeで呼ぶ

RustのクレートをDLLにして、C#から直接呼ぶ。WebView2の中でwasmを動かす必要がない。しかし、CPUの種類ごとにDLLを作り、配る必要がある。HTMLを作る場所がC#に移るので、付箋紙アプリの書き換えも大きい。そのため選ばなかった。

### Alternative 3: TypeScriptのまま、0.1.7に上げる

付箋紙アプリの`FmAst2HtmlVersion`を上げるだけで済む。.jsは小さいままである。しかし、正本と違うHTMLを出す実装が残る。ASTの仕様を変えるたびに、2つの実装を直す作業も残る。そのため選ばなかった。

### Alternative 4: npmのリポジトリを残し、中身をwasmのビルドに入れ替える

リポジトリ`petajournal/flatmarkdown-ast2html`を使い続ける。npmの公開の設定を作り直さずに済む。しかし、wasmの層とRustのクレートが別のリポジトリになる。クレートを変えるたびに、2つのリポジトリでバージョンを合わせる必要がある。そのため選ばなかった。

### Alternative 5: wasmを.jsとは別のファイルで配る

base64にしないので、全体が約3割小さくなる。しかし、付箋紙アプリは、文書を文字列で渡している。別のファイルを読むには、非同期の読み込みと、ファイルを返す仕組みが要る。呼び出しの形も、今と同じにはできない。そのため選ばなかった。

### Alternative 6: ESモジュール、CommonJS、型定義も配る

npmのパッケージとして、普通の形になる。しかし、それを使う利用者が今はいない。ビルドとテストの対象だけが増える。そのため選ばなかった。

### Alternative 7: npmのバージョンを、クレートと別に付ける

0.1.7の次を0.2.0とする。npmだけを見れば、番号が自然に続く。しかし、どのクレートから作ったかが、番号から分からなくなる。対応の表を保つ必要もある。そのため選ばなかった。

### Alternative 8: npmに公開せず、GitHub Releaseに.jsを置く

付箋紙アプリは、ビルドのときにURLから.jsを1つ取るだけである。取得元は、npmでなくてもよい。npmの公開の手順と、破壊的な更新がなくなる。しかし、npmのパッケージは、TypeScriptの古い実装のまま残る。正本と違うHTMLを出すパッケージが、公開され続ける。そのため選ばなかった。

## Confidence Level

確信度は中程度である。

wasmにするという方向には、高い確信がある。試作で、wasmの大きさ、同期での初期化、300件の変換を測り、付箋紙アプリで問題にならないことを確かめた。呼び出しの形も、今と同じに保てる。

npmのパッケージとして公開するという形には、中程度の確信しかない。このパッケージを使うのは、今のところ付箋紙アプリだけである。利用者が1つのものに対して、npmへの公開と破壊的な更新は重すぎるかもしれない。それでもこの形を選ぶのは、正本と違うHTMLを出す実装を、公開したまま残さないためである。

まだ確かめていないことが2つある。1つは、実際のWebView2での動作である。試作はヘッドレスのEdgeで測ったもので、`NavigateToString`で渡した文書の中では試していない。もう1つは、`NavigateToString`の大きさの上限（約2MB）である。.jsが約4KBから234KBになるので、付箋の本文に使える余地が約1割減る。

次のときに、この決定を見直す。

- WebView2で、wasmの同期での初期化ができない、または目に見えて遅いと分かったとき。
- 付箋の内容が`NavigateToString`の上限に届くようになったとき。この場合は、.jsを文書に埋め込むのをやめる。WebView2の`SetVirtualHostNameToFolderMapping`でフォルダをホスト名に割り当て、`<script src>`で読み込む形を検討する。
- 付箋紙アプリがこのパッケージを使わなくなったとき。利用者がいなくなれば、npmでの公開をやめる。
- ESモジュールや型定義を求める利用者が現れたとき。

## Consequences

### 良い結果

付箋紙アプリが出すHTMLは、Webフロントエンドの「HTMLとしてコピー」が出すHTMLと同じになる。どちらも、Rustのクレート`flatmarkdown-ast2html`が作るためである。

ASTからHTMLを作る実装は、1つになる。ASTの仕様にノードを足したときに直す場所は、Rustのクレートだけである。TypeScriptの実装を追従させる作業はなくなる。

付箋紙アプリで、ハッシュタグと埋め込み（`embed`）が正しく表示されるようになる。今の0.1.3は、どちらも扱えない。

npmのパッケージのバージョンから、中身のクレートのバージョンが分かる。

### 悪い結果

.jsは、約4KBから234KBになる。`Stickies.exe`がその分大きくなり、`NavigateToString`の上限までの余地が約1割減る。

npmのパッケージは、破壊的に変わる。0.1.xを使う外部の利用者がいれば、0.1.7に留まるか、書き換える必要がある。

wikilinkとハッシュタグのタグ名や属性を、呼び出し側から変えられなくなる。変えたいときは、Rustのクレートに機能を足す必要がある。

flatmarkdownのリポジトリのビルドに、wasmの道具（wasm-pack、wasm-opt）と、.jsを1つにまとめる手順が加わる。リリースの手順には、crates.ioに加えて、npmへの公開が加わる。

デバッグがしにくくなる。wasmの中は、ブラウザーの開発者ツールで追いにくい。

### 中立的な結果

flatmarkdownのワークスペースのメンバーは、4つになる。4つ目の`flatmarkdown-ast2html-wasm`は、crates.ioには公開しない。

付箋紙アプリの`INSTALL.md`は、この変更に合わせて直す。「ソースからビルドする」手順と、「バージョンを変えたらファイルを消す」手順が変わる。

todoアプリは変わらない。`flatmarkdown-ast2react`は、TypeScriptの別の実装のまま残る。この実装をどうするかは、このADRでは決めない。
