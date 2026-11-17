# ADR 061: Page URLs in One Crate

## Status

Accepted

## Context

PetaJournalのページは、アプリの中のURLで開く。
ふつうのページは`/pages/<ページ名>`、タグのページは`/pages/<タグ用ディレクトリ><タグ名>`である。
ページの中の項目へ移るときは、後ろに`#<項目のid>`を付ける。

このURLを作る知識と読む知識が、多くの場所に分かれている。
2026-11-16に調べた。
テスト、README、使い方の例、コメントは除いている。

### URLを作る場所（Rust）

- `client-page-links/src/links.rs`の`wikilink_path`と`hashtag_path`。
  Webのシェル、「HTMLとしてコピー」（ADR 027）、付箋紙の`PageAddress`が使う。
- `client-core/src/rich_text/links.rs`。
  ネイティブのシェルのための`RichLink::Page`で、上と同じURLを作る。
- `client-core/src/screens/navigate.rs`。ページへ移るときのURLである。
- `client-core/src/view.rs`。ビューモデルに出すページのURLである。
- `client-core/src/ops/wiki.rs`。タグのページの名前を、その場で作る。
- Webのシェルの`components/item_view.rs`。`/pages/Untitled-…`を作る。

### URLを作る場所（Rustのほか）

- Android：`shell/Shell.kt`の定数`PAGES`。コアに渡すアプリの中のURLである。
- iOS：`PetaBorder/Shell.swift`の定数`pages`。コアに渡すアプリの中のURLである。
- Emacs：`lisp/peta-journal.el`。
- todo：`web-app/todo/src/links.ts`と`tasks.ts`の`pagePath`。
- memoview：`Services/PageLinks.cs`。

### URLを読む場所

- `client-core/src/app.rs`の`Route::parse`。
  `pages/`、`docs/`、`journals/`を外して、ページ名にする。
  どのシェルも、最後はここを通る。
- `client-core/src/rich_text/links.rs`。`/pages/`で始まるリンクを、アプリの中のページと見なす。
- `client-shell-embedded/src/lib.rs`。`Route::parse`とは別に、`/pages/`を外してページ名を取る。

### サーバーはURLを知らない

`journal-server`の`presentation/frontend.rs`は、`/api/`以外のどのパスにも`index.html`を返す。
`/pages/`という名前は、コメントにしか出てこない。
journal-coreにも、この値はない。
そのため、Rustを呼べないクライアント（todo、memoview、Emacs）は、URLの知識をサーバーから受け取れず、それぞれが自分で持っている。

### ずれが出ている

分かれた知識は、すでにずれている。

- todoの`pagePath`は、ジャーナルを`/journals/`、ドキュメントを`/docs/`で作る。ほかはすべて`/pages/`で作る。
  `/journals/`と`/docs/`は、Angular版の最初のURLである。
  Angular版は途中で`/pages/`に統一し、古い2つを後方互換のために残した。
  2026-10-16にRustへ移したとき、この読み方が`Route::parse`にそのまま移った。
- 符号化が違う。
  memoviewの`PageLinks.cs`は、`#`と`?`まで符号化する。
  `client-page-links`は、`#`を残す。
  todoの`pagePath`は、符号化しない。

### 描画のライブラリの形も違う

項目の本文の中のwikilinkとハッシュタグは、flatmarkdownの描画のライブラリがリンクにする。
hrefの決め方が、ライブラリごとに違う。

- Rustの`flatmarkdown-ast2html`は、テンプレートで決める（ADR 051）。`/pages/{}`の`{}`に名前が入る。
- `flatmarkdown-ast2react`は、前置き（`hrefPrefix`）で決める。todoが使う。
- wasm版の`flatmarkdown-ast2html`には、設定がない。hrefはページ名のままである。
  memoviewは、hrefを名前として読み、C#でURLを作る。

### 開発者が決めていたこと

- URLの知識は、1か所にあるべきである（2026-11-16）。
- 少なくとも`pages`の値は、journal-coreで管理する（2026-11-17）。
  AndroidとiOSは、サーバーのURLにアクセスしない。
  それでも、コアに渡すアプリの中のURL（ページを開く、履歴、前回の画面）で`/pages/`を使うためである。
- journal-serverは、URLを作る機能の一部を、journal-coreに任せる（2026-11-17）。

`client-core`はwasmにもビルドするので、journal-coreに依存できない。
journal-coreは、rusqliteとtantivyを使うためである。

## Decision

ページのURLを作る規則と読む規則を、1つのクレート`page-urls`に置く。
ほかのコードは、URLを自分で組み立てず、自分で読み解かない。

### クレート`page-urls`

`journal/page-urls`に、新しいクレートを作る。
`journal/hmtid`と同じ並びである。
依存は`flatmarkdown-ast2html`だけで、wasmにもビルドできる。

このクレートは、次のものを持つ。

- 描画のライブラリに渡すテンプレートを返す関数。テンプレートそのものは、flatmarkdownの関数に前置きを渡して作る。
  wikilinkのテンプレートは`/pages/{}`、ハッシュタグのテンプレートは`/pages/<タグ用ディレクトリ>{}`である。
  形は、ADR 051の`HtmlOptions`と同じである。
- ページのURLを作る関数。wikilinkのテンプレートにページ名を入れて、`/pages/<ページ名>`を作る。
- 項目のURLを作る関数。ページのURLの後ろに`#<項目のid>`を付ける。
- タグのページの名前を作る関数。タグ用ディレクトリとタグ名をつなぐ。
- 画像の相対パスを解く基準（`/pages/`）。
- URLを読む関数。今の`Route::parse`のうち、パスからページ名と項目のidを取り出す部分である。

週やジャーナルの名前の決まりは、このクレートに入れない。
決定を小さく保つためである。

### テンプレートを作るのも、名前を入れるのも、flatmarkdownだけである

知識を、次の2つに分ける。

| | 知っていること | 知らないこと |
|---|---|---|
| flatmarkdown | テンプレートの書式（`{}`）、テンプレートの作り方、名前の入れ方 | PetaJournalのページが`/pages/`の下にあること |
| `page-urls` | ページは`/pages/`の下にある。タグのページは、さらにタグ用ディレクトリの下にある。URLの読み方 | `{}`という書式、名前の入れ方 |

`page-urls`のコードには、`{}`という文字を書かない。
`page-urls`は、前置き（`/pages/`）をflatmarkdownの関数に渡して、テンプレートを受け取る。
ページのURLを作るときは、そのテンプレートとページ名を、flatmarkdownの関数に渡す。
`page-urls`にとってテンプレートは、flatmarkdownから受け取って、flatmarkdownへ返すだけの、中身を見ない文字列である。

描画のライブラリが本文の中のリンクを作るときも、名前を入れる同じ関数を通る。
そのため、本文の中の`[[ページ名]]`のリンクと、サーバーが返す`url`は、必ず同じURLになる。

### flatmarkdownが公開する2つの関数

名前を入れる処理は、今は`flatmarkdown-ast2html`の内部にある（`src/lib.rs`の`href`）。
テンプレートの`{}`をすべて名前に置き換え、そのあとでHTMLのエスケープをかける。

```rust
const NAME: &str = "{}";

fn href(template: &str, name: &str) -> String {
    escape_html(&template.replace(NAME, name))
}
```

`ast2html`は、wikilinkとハッシュタグの`<a href="…">`を書くときに、この関数を呼ぶ。
wikilinkでは`name`にノードの`url`が入り、ハッシュタグではタグ名が入る。

flatmarkdownの側で、公開の関数を2つ足す。
ここでは仮に`href_template_from_prefix`と`fill_href`と呼ぶ。名前は、flatmarkdownの側で決める。

- `href_template_from_prefix`は、前置きからテンプレートを作る。`/pages/`から`/pages/{}`ができる。
- `fill_href`は、テンプレートに名前を入れて、URLを返す。HTMLのエスケープはかけない。

今の内部の関数`href`は、`fill_href`の結果にHTMLのエスケープをかける形に変える。

```rust
/// The template of the hrefs that start with `prefix`.
pub fn href_template_from_prefix(prefix: &str) -> String {
    format!("{prefix}{NAME}")
}

/// The URL that `template` makes for `name`. It is not escaped for HTML.
pub fn fill_href(template: &str, name: &str) -> String {
    template.replace(NAME, name)
}

fn href(template: &str, name: &str) -> String {
    escape_html(&fill_href(template, name))
}
```

`{}`という文字が出てくるのは、flatmarkdownのこの定数`NAME`だけである。

HTMLのエスケープを分けるのは、`page-urls`が作るのはHTMLの属性ではなく、URLそのものであるためである。
エスケープをかけたままだと、`&`を含むページ名のURLが`&amp;`になる。

`href_template_from_prefix`の入力と出力は、次のとおりである。

| 前置き | 結果 |
|---|---|
| `/pages/` | `/pages/{}` |
| `/pages/tags/` | `/pages/tags/{}` |

`fill_href`の入力と出力は、次のとおりである。

| テンプレート | 名前 | 結果 |
|---|---|---|
| `/pages/{}` | `Projects/Foo` | `/pages/Projects/Foo` |
| `/pages/tags/{}` | `Rust` | `/pages/tags/Rust` |
| `/pages/{}` | `2026-11-18` | `/pages/2026-11-18` |
| `{}`（既定） | `Other Page` | `Other Page` |
| `/pages/{}` | `What?` | `/pages/What?`（今は符号化しない） |

### `page-urls`の関数

`page-urls`は、flatmarkdownの2つの関数を呼ぶだけである。
関数の名前は、実装のときに決める。

```rust
/// The base that every page lives under. `pagesBase` of the config.
pub const PAGES_BASE: &str = "/pages/";

pub fn wikilink_href() -> String {
    href_template_from_prefix(PAGES_BASE)                            // "/pages/{}"
}

pub fn hashtag_href(tags_dir: &str) -> String {
    href_template_from_prefix(&format!("{PAGES_BASE}{tags_dir}"))    // "/pages/tags/{}"
}

pub fn page_url(page_name: &str) -> String {
    fill_href(&wikilink_href(), page_name)               // "/pages/Projects/Foo"
}

pub fn item_url(page_name: &str, item_id: &str) -> String {
    format!("{}#{item_id}", page_url(page_name))         // "/pages/Projects/Foo#abc123"
}
```

呼ぶ場所は、次のとおりである。

- 本文の中のリンク：`ast2html`が、`HtmlOptions`のテンプレートとノードの名前で、内部から`fill_href`を通る。
  テンプレートは、Rustのクライアントなら`page-urls`の`wikilink_href`と`hashtag_href`から、ほかのクライアントなら`GET /api/config`から来る。
- サーバーが返す`url`：journal-coreが、`page-urls`の`page_url`と`item_url`を呼ぶ。
- ページへ移るときのURL、ビューモデルのURL：`client-core`が、`page-urls`の`page_url`を呼ぶ。

flatmarkdownが符号化を始めるときは、`fill_href`の中で、名前を符号化してから置き換える。
`page-urls`と`ast2html`の呼び出しは、変えなくてよい。

URLの形が変わったときに直すのは、`page-urls`の中と、flatmarkdownのテンプレートを作る関数だけである。
たとえば、後ろにも文字を置く形（`/tags/{}.html`）が要るようになったら、flatmarkdownに、その形のテンプレートを作る関数を足す。
`page-urls`は、その関数を呼ぶように変える。
`fill_href`、描画のライブラリ、todoなどのクライアントは、変えなくてよい。
後ろ置きのための引数は、今は用意しない。使う場面が、まだないためである。

`flatmarkdown-ast2react`は、TypeScriptで書かれているので、Rustの`fill_href`を呼べない。
同じ置き換えを、TypeScriptで持つことになる。
Rust版とTypeScript版をどうそろえるかは、flatmarkdownの側で決める。

`page-urls`が自分でつなぐ形にすると、同じURLを作る処理が、flatmarkdownと`page-urls`の2か所になる。
片方だけが符号化の規則を変えると、同じページへの2つのリンクが、違うページを開く。
これを避けるためである。

### 依存の向き

journal-coreは、`page-urls`に依存し、再公開する。
journal-serverは、journal-coreの再公開から使う。
`client-core`と`client-page-links`は、`page-urls`に直接依存する。
`page-urls`は、`flatmarkdown-ast2html`に依存する。

```
journal-server ──→ journal-core ──→ rusqlite, tantivy
                        │（再公開する）
                        ▼
                   page-urls ──→ flatmarkdown-ast2html ──→ flatmarkdown-ast
                     ▲    ▲
           client-core    client-page-links
```

`flatmarkdown-ast2html`の依存は、`flatmarkdown-ast`だけである。
パーサー（comrak）には依存しない。
journal-coreと`client-page-links`は、すでにflatmarkdownのクレートに依存しているので、新しい重い依存は入らない。

Rustのクライアントは、コンパイル時に同じ規則を使う。
そのため、サーバーとRustのクライアントの間で、URLの形がずれない。

### URLの形は`/pages/`だけにする

作るURLは、`/pages/`の形だけである。
読むURLも、`/pages/`の形だけである。
`docs/`と`journals/`の読み方は消す。
`/today`と`/settings`は、今のまま残す。
タグのページも、今と同じく`/pages/`の下に置く。
`/tags/`という新しいURLは作らない。

### Rustを呼べないクライアントには、サーバーが渡す

todo、memoview、Emacsは、Rustのクレートを呼べない。
これらには、サーバーがRESTで渡す。
ADR 060により、MCPの答えにも同じものが入る。

渡すものは2種類ある。

1つめは、出来上がったURLである。
ページの答えに`url`を足す。値は`/pages/<ページ名>`である。
項目の答えにも`url`を足す。値は`/pages/<ページ名>#<項目のid>`である。
クライアントは、ページや項目へのリンクを、この値から作る。
todoの`pagePath`は消す。

2つめは、項目の本文を描くための値である。
`GET /api/config`の答えに、次の3つを足す。

- `wikilinkHref`：wikilinkのテンプレート。例は`/pages/{}`である。
- `hashtagHref`：ハッシュタグのテンプレート。例は`/pages/tags/{}`である。
- `pagesBase`：画像の相対パスを解く基準。値は`/pages/`である。

`hashtagHref`は、タグ用ディレクトリを含む。
開発者が設定でタグ用ディレクトリを変えると、この値も変わる。
クライアントは、今の設定の読み直しで、新しい値を受け取る。

サーバーが渡すのは、パスだけである。
オリジン（`http://localhost:8087`など）は入れない。
サーバーは、自分がどのアドレスで開かれているかを知らないためである。
クライアントが、自分の知っているサーバーのアドレスを前に付ける。

### クライアントはテンプレートを解釈しない

クライアントは、`wikilinkHref`と`hashtagHref`を、中身を見ずに描画のライブラリへ渡す。
`{}`に名前を入れるのは、描画のライブラリだけである。
テンプレートだけを渡してクライアントが組み立てると、テンプレートの書式と符号化の知識が、クライアントに残るためである。

そのために、flatmarkdownの側で次の3つを行う。

- `flatmarkdown-ast2html`に、テンプレートを作る関数と、テンプレートに名前を入れる関数を公開する。`page-urls`が呼ぶためである。
- `flatmarkdown-ast2react`の`hrefPrefix`を、テンプレートに置き換える。
  前置き`/pages/`は、テンプレート`/pages/{}`と同じ結果になるので、テンプレートで代用できる。
  ハッシュタグの既定値も、Rustの`ast2html`にそろえる。
- wasm版の`flatmarkdown-ast2html`に、Rust版と同じ設定を足す。
  ADR 051は、wasm版で設定が要るアプリが出たら足すと決めていた。memoviewが、そのアプリに当たる。

あとの2つが終わるまで、todoとmemoviewの本文の中のリンクは、今のまま残す。
終わったら、todoの`links.ts`の組み立てと、memoviewの`PageLinks.cs`を消す。

### シェルはURLを作らない

AndroidとiOSのシェルは、URLを自分で作らない。
ページ名をコアに渡すか、コアの関数でURLを作る。
`Shell.kt`の定数`PAGES`と、`Shell.swift`の定数`pages`は消す。
Webのシェルの`item_view.rs`と、`client-shell-embedded`の`/pages/`を外す処理も、`page-urls`の関数に替える。

### 符号化

`page-urls`は、符号化の規則を持たない。
符号化するかどうかは、テンプレートに名前を入れるflatmarkdownの関数が決める。

今のflatmarkdownは、符号化しない（ADR 051）。
そのため、`%`、`?`、`#`を含むページ名では、本文の中のリンクも、サーバーが返す`url`も、正しいページを開かない。
たとえば`What?`というページのURLは`/pages/What?`になり、ブラウザーは`What`というページを開こうとする。
壊れ方は、本文の中のリンクと`url`で同じである。

flatmarkdownの関数が符号化するようになれば、本文の中のリンクと`url`が、同時に符号化される。
この作業は、このADRの外である。
`docs/plans.md`に、ADR 051を改めるタスクとして積んである。

`page-urls`のURLを読む関数は、flatmarkdownが作ったURLを、元のページ名に戻せなくてはならない。
`page-urls`のテストで、「作ったURLを読むと、元のページ名に戻る」ことを確かめる。
flatmarkdownが符号化を始めたときに、読む側が追いついていなければ、このテストが落ちる。

「HTMLとしてコピー」（ADR 027）の符号化（`client-page-links`の`OUTSIDE_A_URL`など）は、今のまま`client-page-links`に残す。
flatmarkdownが符号化するようになったら、置き換えるかを決める。

## Alternatives

### Alternative 1: journal-coreをfeatureで分ける

journal-coreの中にURLの規則を置き、DBと索引をfeatureで外せるようにする。
`client-core`は、featureなしのjournal-coreに依存する。

- 長所：クレートが増えない。
- 短所：journal-coreのDBと索引を使うモジュールすべてに、`cfg`を付ける必要がある。
  wasmのビルドがfeatureなしで通ることを、確かめ続ける必要がある。

journal-coreへの変更が広いので、採らなかった。

### Alternative 2: サーバーが実行時に渡すだけにする

コンパイル時の依存は足さない。
`client-core`も、サーバーや組み込みのjournal-coreから、実行時に値を受け取る。

- 長所：依存が増えない。どの言語のクライアントも、同じ道で値を取る。
- 短所：`client-core`は、値が届くまでURLを作れず、読めない。
  最初の画面を開く`Route::parse`が、設定の答えを待つことになる。

起動が複雑になるので、採らなかった。

### Alternative 3: 各言語の定数を、Rustから生成する

Rustのクレートから、TypeScript、C#、Emacs Lispの定数のファイルを作る。

- 長所：どのクライアントも、実行時にサーバーへ聞かずに済む。
- 短所：言語ごとに、生成の仕組みが要る。
  タグ用ディレクトリは設定で変わるので、結局は実行時の値と組み合わせる必要がある。

仕組みが重いので、採らなかった。

### Alternative 4: 定数を各クライアントに残し、テストで確かめる

- 長所：クライアントのコードを変えなくてよい。
- 短所：知識は分かれたままである。符号化のずれのような違いは、定数の比較では見つからない。

1か所にするという目的に合わないので、採らなかった。

### Alternative 5: サーバーはテンプレートだけを渡す

出来上がったURLは渡さず、クライアントがテンプレートに名前を入れてURLを作る。

- 長所：答えのJSONが小さい。
- 短所：`{}`の書式、入れる値、符号化の規則を、3つの言語のクライアントが知る必要がある。

知識がクライアントに残るので、採らなかった。

### Alternative 6: サーバーは前置きだけを渡す

サーバーは`/pages/`だけを返し、クライアントは前置きと名前をつなぐ。

- 長所：flatmarkdownを変えなくてよい。`ast2react`の`hrefPrefix`に、そのまま渡せる。
- 短所：Rustの`ast2html`（テンプレート）と形が2つになる。
  `/tags/{}.html`のように、後ろに文字を置く形を作れない。

形を1つにそろえたいので、採らなかった。

### Alternative 7: `docs/`と`journals/`を、読む側にだけ残す

- 長所：古い形のブックマークやリンクが、引き続き開ける。
- 短所：読む規則が3つ残る。作る規則と読む規則が、1対1にならない。

URLの形を1つにするので、採らなかった。

### Alternative 8: `page-urls`が、自分で名前をつないで符号化する

`page-urls`は、flatmarkdownに依存しない。
ページのURLは、`/pages/`と、符号化したページ名を自分でつないで作る。
flatmarkdownとの一致は、両方の結果を比べるテストで守る。

- 長所：flatmarkdownに公開の関数を足さなくてよい。サーバーが返す`url`を、すぐに符号化できる。
- 短所：同じURLを作る処理と符号化の規則が、flatmarkdownと`page-urls`の2か所になる。
  flatmarkdownが符号化を始めるまで、本文の中のリンクと`url`で、開くページが違う。

知識が二重になるので、採らなかった。

## Confidence Level

確信度は中くらいである。
URLの規則を1つのクレートに置き、Rustのクライアントがそれを使う部分には、確信が高い。
`hmtid`が、すでに同じ形で動いているためである。

一方で、次のことはまだ試していない。

- `ast2react`とwasm版の`ast2html`が、同じテンプレートを受け取れるようになること。
- memoviewが、hrefを名前ではなくURLとして開く形に切り替えられること。
- flatmarkdownが符号化を始めたあとのURLを、どのシェルも元のページ名に読み戻せること。

次のことが起きたら、この決定を見直す。

- ページの種類ごとに違うURL（`/tags/`など）が要るようになったとき。
- flatmarkdownの側の3つの作業が、長く進まないとき。
- flatmarkdownの符号化の規則が、PetaJournalのURLに合わないと分かったとき。
- 答えの`url`が、クライアントの求める形（オリジン付き、別のアプリのURLスキームなど）に合わないと分かったとき。

## Consequences

良い結果は、次のとおりである。

- `/pages/`という値と、URLを読む規則が、`page-urls`だけにある。
  テンプレートを作る処理と、名前を入れる処理は、flatmarkdownだけにある。
- 本文の中のリンクと、サーバーが返す`url`が、同じ関数から出る。同じページには、必ず同じURLが付く。
- サーバーがURLを知る。RESTとMCPの答えに、ページと項目のURLが入る。
- todo、memoview、Emacs、Android、iOSから、`/pages/`の直書きが消える。
- todoのリンクが、ほかのクライアントと同じ`/pages/`の形になる。
- 3つの描画のライブラリが、同じ形のテンプレートを受け取る。

悪い結果は、次のとおりである。

- `/journals/`と`/docs/`の形のブックマークやリンクは、開けなくなる。
- flatmarkdownの側に、3つの作業が要る。
  `ast2react`の`hrefPrefix`を消すのは、`ast2react`の破壊的な変更である。
  終わるまで、todoとmemoviewには直書きが残る。
- ページと項目の答えが、`url`の分だけ大きくなる。
  `url`は、ページ名と項目のidから作れる値である。
- `%`、`?`、`#`を含むページ名のURLは、まだ壊れる。
  本文の中のリンクも、サーバーが返す`url`も、同じように壊れる。
  ADR 051を改めるタスクが終わるまで、この制限が残る。
- クレートが1つ増える。journal-core、`client-core`、`client-page-links`の3つが、これに依存する。
- `page-urls`が、flatmarkdownの版に縛られる。
  flatmarkdownの3つのクレートの版をそろえる決まり（ADR 020）に、`page-urls`も従う。

中立的な結果は、次のとおりである。

- `GET /api/config`の答えに、設定ではない値（`wikilinkHref`、`hashtagHref`、`pagesBase`）が入る。
  `hashtagHref`は、設定の`tagsDir`から作る値である。
- ADR 051のWebのシェルの切り替えでは、テンプレートを`page-urls`から取る。
- 「HTMLとしてコピー」（ADR 027）の動きは変えない。URLを作る関数の置き場所だけが、`page-urls`に変わる。
- `client-page-links`には、コピーするHTMLを作る処理が残る。
- 週やジャーナルの名前のための小さなクレートは、別に決める。
