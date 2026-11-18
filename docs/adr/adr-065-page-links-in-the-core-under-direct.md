# ADR 065: Page Links and the Copied HTML in client-core, under `client_core::direct`

## Status

Proposed

## Context

### コピーのHTML

項目をコピーすると、シェルはクリップボードにMarkdownとHTMLの2つを書く（ADR 027）。
HTMLは、外のアプリに書式つきで貼り付けるためのものである。
PetaJournalの中への貼り付けには使わない。
以下では、これを「コピーのHTML」と呼ぶ。
コピーのHTMLでは、PetaJournalの中へのリンク（ウィキリンク、ハッシュタグ、`.md`への相対リンク）が、アプリを開く絶対URLになる。
貼り付けた先のアプリから、元のページを開けるようにするためである。

### 今の依存

`client-page-links`は、ページへのリンクの行き先と、コピーのHTMLを作るクレートである（ADR 027、ADR 037）。
中身は、`AppLinks`（`src/links.rs`）と`copied_html`（`src/copied_html.rs`）の2つで、合わせて約300行である。
`page-urls`（ADR 061）、`flatmarkdown-ast`、`flatmarkdown-ast2html`、`percent-encoding`に依存する。
`flatmarkdown`には、テストでASTを作るためだけに依存する（`[dev-dependencies]`）。

`client-page-links`を使うのは、次の2つである。

- `client-native-bridge`：`copied_html`（`src/html.rs`）と、`page_address`・`asset_address`（`src/links.rs`）で使う。
  `client-shell-http`の`ffi.rs`が、`client_native_bridge::copied_html`を呼ぶ。
- outliner-web：`src/effects/ui.rs`で、`copied_html`と`AppLinks`を使う。

`page-urls`を直接使うシェルは、次の3つである。

- outliner-web：`components/item_html.rs`、`components/item_view.rs`、`components/page_properties.rs`。
- outliner-emacs：`src/shell.rs`。
- `client-shell-embedded`：`src/lib.rs`と`src/screen.rs`で、`journal_core::page_urls`の再公開から使う。

`client-core`も、`page-urls`に直接依存する。
`rich_text/links.rs`、`view.rs`、`screens/navigate.rs`、`app.rs`、`ops/wiki.rs`で、ページのURLを作り、読む。

そのため、`client-native-bridge`は、`flatmarkdown-ast2html`に2つの経路で依存する。

```
client-native-bridge ─→ client-page-links ─→ page-urls ─→ flatmarkdown-ast2html
        └─────────────→ client-core ───────→ page-urls ─→ flatmarkdown-ast2html
```

outliner-webも、`client-page-links`、`page-urls`、`client-core`の3つを通して、同じクレートに届く。
Cargoは同じ版のクレートを1回だけビルドするので、バイナリの中のコードは1つである。
版のずれは、ADR 020の版の固定（`=0.9.0`）で防いでいる。
壊れてはいないが、同じものに別々の経路で依存しており、依存の図が読みにくい。

### `client-page-links`をコアに入れなかった理由

`client-page-links`の`src/lib.rs`は、コアに入れない理由を2つ挙げている。

1. コアはHTMLを作らない（ADR 025、ADR 026）。
2. コアは、アプリが配信されている場所（アプリのURL）を知らない。

ADR 027のAlternative 3（コアがHTMLを作る）も、同じ理由で退けた。
加えて、HTMLを使わないEmacsとSwiftUIのシェルでも、コピーのたびにHTMLを作ることになる点を挙げた。

今は、次の点が変わっている。

- `client-core`は、`page-urls`を通して、すでに`flatmarkdown-ast2html`に依存する（ADR 061）。
- `client-core`は、ネイティブのシェルのために、RichTextのリンク先のURLを作る（ADR 042、`rich_text/links.rs`）。
  リンクの行き先を決めることは、すでにコアの仕事である。
- ネイティブ4アプリも、`client-native-bridge`の`copied_html`で、HTMLをクリップボードに書く。
  コピーのHTMLを書かないのは、Emacsだけである。
  ただし、Emacsも、項目をMarkdownのまま表示しながら、コピーのHTMLをクリップボードに書く余地はある。
  表示の形と、外のアプリに渡す形は、別のものだからである。

### Cruxの外で呼ばれる関数

シェルは、`Event`を送り、コアが返す`Effect`とビューモデルを扱う。
ただし、シェルが`Event`を通さずに、コアの関数を直接呼ぶ場所もある。
`page_urls::page_url`、`client_core::drag::edge_step`、`client_core::util::utf`などである。
今は、どれがそういう関数かが、名前からは分からない。

ADR 064は、シェルが使ってよいコアの中身を許可リストに書くこと（S1）を試すと決めた。
許可リストには、`client_core::Event`、`client_core::view`、`client_core::effects`と、「シェルが直接呼ぶと決めた関数」を書く。
シェルが直接呼ぶ関数が1つのモジュールにまとまっていれば、許可リストにはそのモジュールを書けばよい。

## Decision

`client-page-links`を`client-core`に取り込み、クレートを無くす。
シェルが`Event`を通さずに呼ぶリンクの関数は、`client_core::direct`から再公開する。

### `client-page-links`の取り込み

`client-page-links`の`src/links.rs`と`src/copied_html.rs`を、`client-core`のモジュールに移す。
コードは変えない。
`client-page-links`のテストも、`client-core`のテストに移す。
`client-core`は`flatmarkdown`にすでに通常の依存を持つので、テストのための依存は増えない。
`client-core`には、`percent-encoding`の依存が新しく入る。

`client-core`の`update`と`view`は、`copied_html`を呼ばない。
HTMLを作るのは、今と同じく、シェルが`copied_html`を呼んだときだけである。
アプリのURLも、今と同じく、シェルが`AppLinks`の`app_url`として渡す。
そのため、ADR 027の決定（コピーのHTMLはシェルが作る）は変わらない。
変わるのは、その関数を置くクレートだけである。

### `client_core::direct`

`client-core`に、`direct`モジュールを作る。
`direct`は、シェルが`Event`を通さずに直接呼ぶものを集める場所である。
名前は、ADR 064の「シェルが直接呼ぶと決めた関数」にそろえた。

`direct`は、次のものを公開する。

```rust
// client-core/src/direct.rs
pub use page_urls;                          // the crate, as journal-core re-exports it (ADR 061)
pub use crate::page_links::{copied_html, AppLinks};
```

`page-urls`は、クレートごと再公開する。
journal-coreが`page-urls`を再公開する形（ADR 061）と同じである。
シェルは、`client_core::direct::page_urls::page_url`のように呼ぶ。

`client-core`の中のコードは、`page_urls`を今のまま直接使う。
`direct`は、シェルのための入口であり、コアの中の決まりではないためである。

### シェルの依存

次のクレートから、直接の依存を外す。

| クレート | 外す依存 | 代わりに使うもの |
|---|---|---|
| `client-native-bridge` | `client-page-links` | `client_core::direct::{copied_html, AppLinks}` |
| outliner-web | `client-page-links`、`page-urls` | `client_core::direct` |
| outliner-emacs | `page-urls` | `client_core::direct::page_urls` |
| `client-shell-embedded` | （`journal_core::page_urls`の利用） | `client_core::direct::page_urls` |

`client-shell-embedded`は、journal-coreにも依存し続ける。
ただし、URLはシェルの関心なので、ほかのシェルと同じく`client_core::direct`から使う。

outliner-webは、`flatmarkdown-ast`と`flatmarkdown-ast2html`への直接の依存を残す。
項目の表示で、`ast_to_html_with`を自分で呼ぶためである（ADR 026、`components/item_html.rs`）。
表示のHTMLは、シェルが作るものである。
そのため、`direct`には入れない。

### `direct`に入れないもの

ほかにもシェルが直接呼ぶ関数がある。
`client_core::drag`、`client_core::util::utf`、`client_core::policy`などである。
これらは、この決定では移さない。
ADR 064のS1を試すときに、1つずつ、`direct`に移すか、コアに任せる形に直すかを決める。

## Alternatives

### Alternative 1: 今のままにする

`client-page-links`を残し、依存も変えない。

長所は、何も変えなくてよいことである。
Cargoが同じ版を1回だけビルドするので、バイナリの大きさは変わらない。

短所は、同じクレートに別々の経路で依存する形が残ることである。
Cruxの外で呼ばれる関数が、名前から分からないことも残る。

### Alternative 2: `client-page-links`を残し、`client-core`が再公開する

`client-core`が`client-page-links`に依存し、`client_core::direct`から再公開する。
シェルは`client-core`だけに依存する。

長所は、コードを移さなくてよいことである。

短所は、クレートの数が減らないことである。
`client-page-links`は、`client-core`のほかに使う者がいなくなる。
別のクレートにしておく理由が無い。

### Alternative 3: コアがHTMLを作る（ADR 027のAlternative 3）

コアが`copied_html`を呼び、`UiOp::CopyMarkdown`にHTMLを入れる。

長所は、どのシェルも同じHTMLを受け取ることである。

短所は、ADR 027の決定を覆すことである。
コアにアプリのURLを渡す仕組みも要る。
今のEmacsのシェルでは、使わないHTMLをコピーのたびに作る。
outliner-webの表示用の`ast2html`は残るので、依存の重なりも消えない。

### Alternative 4: 別の名前にする

`client_core::for_shells`や`client_core::outside_crux`も考えた。

`for_shells`は、だれのためのものかが分かる。
ただ、`view`や`effects`もシェルのためのものなので、区別が弱い。
`outside_crux`は意味が正確だが、長い。
`direct`は短く、ADR 064の言い方にそろう。

### Alternative 5: 直接呼ばれる関数をすべて`direct`に移す

`drag`、`util::utf`、`policy`なども、この決定で`direct`に移す。

長所は、一度で整理が終わることである。

短所は、関数ごとに、`direct`に置くか、コアに任せる形に直すかを決める必要があることである。
それはADR 064のS1の試みでする判断であり、この決定の範囲を超える。

## Confidence Level

確信は高い。
移すコードは約300行で、振る舞いは変わらない。
同じ形（ライブラリが小さなクレートを再公開する形）を、journal-coreと`page-urls`ですでに使っている。

次の場合は見直す。

- `client-core`を使わないRustのクライアントが、コピーのHTMLやリンクを必要としたとき。
  そのときは、`client-page-links`のようなクレートに戻す。
- ADR 064のS1の試みで、`direct`という分け方が許可リストに合わないと分かったとき。
- コアがコピーのHTMLを作る形（Alternative 3）に移ると決めたとき。
  この決定のあとは、`copied_html`がコアの中にあり、6つのシェルのうちEmacs以外の5つがHTMLを書く。
  そのため、コアが`UiOp::CopyMarkdown`にHTMLを入れる余地は、ADR 027のときより大きい。
  Emacsも、Markdownで表示しながら、コピーのHTMLをクリップボードに書く余地がある。
  Emacsがそうすれば、使わないHTMLを作るシェルは無くなる。
  残る課題は、コアにアプリのURLを渡す仕組みである。
  たとえば、EmacsもコピーのHTMLを書くことになったときや、ADR 064のS1で`copied_html`をシェルが直接呼ぶことが問題とされたときに、検討する。

## Consequences

### 良い結果

- `client-native-bridge`の依存は、`client-core`だけになる。
  `flatmarkdown-ast2html`に届く経路は、`client-core`を通る1本になる。
- クレートが1つ減る。
- シェルが`Event`を通さずに呼ぶリンクの関数が、`client_core::direct`という名前で分かる。
- ADR 064のS1の許可リストに、`client_core::direct`と書ける。

### 悪い結果

- `client-core`に、`update`と`view`から呼ばれない関数（`copied_html`）が入る。
  `direct`の外から見ると、コアがHTMLを作るように見えるかもしれない。
- `client-core`に、`percent-encoding`の依存が入る。
- outliner-webは、表示のために`flatmarkdown-ast2html`に直接依存し続ける。
  `client-core`を通る経路と合わせて、2つの経路が残る。

### 中立的な結果

- ADR 037のクレートの表から、`client-page-links`が無くなる。
- ADR 061の「`client-core`と`client-page-links`は、`page-urls`に直接依存する」という依存の向きは、`client-core`だけになる。
- `docs/architecture.svg`では、`client-native-bridge`の`ast2html`のチップが無くなる。
  outliner-webの`ast2html`のチップは、表示のために残る。
- `client-page-links`の`src/lib.rs`にあった、コアに入れない理由の説明は無くなる。
  その理由がもう当てはまらないことは、このADRのContextに書いた。
