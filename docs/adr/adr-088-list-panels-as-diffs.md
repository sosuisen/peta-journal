# ADR 088: List Panels as Diffs

## Status

Proposed

## Context

### 差分で送るのは、アウトラインの行だけである

ADR 076で、コアの`Shown::changes()`（`client-core/src/changes/mod.rs`）は、`Changes<B> { rows, panels }`を返すと決めた。
`rows`は、アウトラインの行の差分である。
要素の型は`ListChange<Row<B>>`で、`Insert { at, item }`、`Remove { at }`、`Move { from, to }`、`Update { at, item }`、`Reset { items }`の5つを持つ。
差分は`list_changes`が、行の`item.id`を鍵にして計算する。
`panels`は、変わったパネルを丸ごと送る。
ADR 076は、「サイドバーのページ、検索の結果、バックリンク、アトラスのカードは、当面はパネルの中で丸ごと送る。シェルが行ごとの扱いを必要としたときに、リストに移す。」と書いた。

ADR 085で、サイドバーは`Panel::IndexPages`、`Panel::Headings`、`Panel::RecentPages`の3つのパネルに分かれた。
今、次の4つのパネルが、要素の並びを丸ごと持っている。

| パネル | 並びのフィールド | 要素の型 | 要素を区別できる値 |
|---|---|---|---|
| `Panel::Headings` | `HeadingsView.headings` | `Heading` | `id`（見出しの項目のid） |
| `Panel::RecentPages` | `RecentPagesView.pages` | `SidebarPage` | `page.id` |
| `Panel::IndexPages` | `IndexPagesView.pages` | `SidebarPage` | `page.id` |
| `Panel::Search` | `SearchView.results` | `SearchEntry` | ない（`position`は並びの位置） |

### 丸ごと送ると、シェルの扱い方がアウトラインと違う

アウトラインの行は、どのシェルも差分を当てて持つ。
一方、上の4つの並びは、パネルが変わるたびに丸ごと差し替える。
同じ「並び」なのに、シェルは2通りの扱い方を持つことになる。
開発者は、並びを差分で考えるほうに揃えたい（2026-11-26）。
速さの差はほとんどないが、シェルを書く人の考え方が、アウトラインと同じ1つになるためである。

並びの中身にも、差分が向いている。

- 見出しは、見出しの項目を1文字直すたびに変わる。変わるのは、ふつう1つの見出しだけである。
- 検索の結果は、1文字打つたびに絞り込まれる。前の結果と共通する部分が多い。
- フォルダのページは、ルートのフォルダでは1000ページを超えることがある。`get_child_pages`には上限がない。1ページの名前が変わっても、全体を送り直している。

### 境界の型のジェネリックは、1種類の型引数でしか使えない

シェルの型は、`facet_generate`が、コアの`facet::Facet`の型から生成する（ADR 078、ADR 080）。
`facet_generate`は、ジェネリックな型を、型引数を埋めた1つの型として生成する。
TypeScriptの今の`ListChange`は、`{ kind: "Insert"; at: uint32; item: Row }`のように、`T`が`Row`に置き換わっている。
そのため、同じジェネリックな型を2種類の型引数で使うと、名前がぶつかる。
`facet_generate`は、`unsupported generic type: ListChange<Page>, the type may have already been used with different parameters`で止まる。

この制限は、ADR 085でも`Loadable<T>`で当たった。
2026-11-26に、最新の0.22.4でも残っていることを確かめた。
`facet_generate`のテストにも「TODO: Eventually we should support generics used more than once」とあり、直す予定は決まっていない。
したがって、`ListChange<Heading>`や`ListChange<SidebarPage>`を、`ListChange<Row<B>>`と並べて境界に出すことはできない。

同じ試しで、`macro_rules!`で作った、名前の違う同じ形の型は生成できることを確かめた。
マクロの型は、型引数を持つもの（`RowChange<B>`）も生成できた。

## Decision

### 並びのパネルを差分で送る

`Panel::Headings`、`Panel::RecentPages`、`Panel::IndexPages`、`Panel::Search`の並びを、アウトラインの行と同じく差分で送る。
並びは、パネルから`Changes`のフィールドに移す。

```rust
pub struct Changes<B> {
    pub rows: Vec<RowChange<B>>,
    pub headings: Vec<HeadingChange>,
    pub recent_pages: Vec<PageChange>,
    pub index_pages: Vec<PageChange>,
    pub search_results: Vec<SearchChange>,
    pub panels: Vec<Panel>,
}
```

並びでないフィールドは、パネルに残し、今と同じく丸ごと送る。
`HeadingsView.state`、`RecentPagesView.state`、`IndexPagesView`の`header`、`path`、`sort`、`parent_path`、`current_page_id`、`state`、`SearchView`の`query`、`show_dropdown`、`selected_index`、`searching`である。
最初の`changes()`は、並びごとに`Reset`を1つ送る。
今の行と同じである。

差分は、コアの`list_changes`で計算する。
鍵は、次のとおりにする。

| 並び | 鍵 |
|---|---|
| 見出し | `Heading.id` |
| 最近のページ、フォルダのページ | `SidebarPage.page.id` |
| 検索の結果 | 新しく足す`SearchEntry.key` |

### 検索の結果に鍵を足し、`position`を外す

`SearchEntry`に、要素を区別する`key: String`を足す。
値は、要素の種類（`SearchKind`）ごとに次のとおりにする。

| `SearchKind` | `key` |
|---|---|
| `Today` | `today` |
| `Page` | `page:`とページのid（`SearchResult.page_id`） |
| `Create` | `create:`と作るページのパス |
| `Item` | `item:`と項目のid（`SearchResult.item_id`） |

`SearchEntry.position`は外す。
`position`は並びの位置なので、先頭に1つ足すだけで、後ろの要素がすべて`Update`になるためである。
`Event::SearchResultClicked { position }`は今のまま残し、シェルは、自分の並びでの位置を送る。
シェルの並びは、コアの並びと同じ順なので、位置は同じになる。

### 差分の型は、マクロでリストごとに作る

`facet_generate`の制限を避けるため、差分の型を、`client-core`の`macro_rules!`で、並びごとに作る。
形は、今の`ListChange<T>`と同じにする。

```rust
list_change!(RowChange<B>, Row<B>);
list_change!(HeadingChange, Heading);
list_change!(PageChange, SidebarPage);
list_change!(SearchChange, SearchEntry);
```

`PageChange`は、最近のページとフォルダのページで共用する。
ジェネリックな型ではないので、2か所で使っても名前はぶつからない。

コアの中では、ジェネリックな`ListChange<T>`と`list_changes`を、今のまま使う。
境界に出すときだけ、マクロの型に移す。
マクロは、型の定義と一緒に、`ListChange<T>`から移す`From`も作る。

### アウトラインの行も、同じマクロの型にする

アウトラインの行の差分も、`RowChange<B>`にする。
4つの並びと行が、同じマクロの、同じ形の型になる。
生成される型の名前は、`ListChange`から`RowChange`に変わる。
`Insert`、`Remove`、`Move`、`Update`、`Reset`の中身は変わらないので、送るJSONは今と同じである。

### シェルは、差分を当てる処理を1つにする

各シェルの、行の差分を当てる処理を、要素の型を問わない処理に直す。
今の処理は、次の場所にある。

| シェル | 場所 |
|---|---|
| Web（`outliner-react`） | `screen.ts` |
| Emacs | `apps/emacs/outliner/src/protocol.rs` |
| Android | `Applied.kt`の`applied` |
| iOSとmacOS | `swift-shared`の`Applied.swift`の`applying` |
| Windowsの付箋紙 | `PetaBorder/ListChanges.cs`の`Apply` |

TypeScriptは、型を形で比べる。
`{ kind, at, item: T }`を受ける1つのジェネリックな関数が、そのまま4つの型に使える。
Swift、Kotlin、C#は、型を名前で比べる。
生成された型ごとに、言語のジェネリックな変更の型へ移す数行の橋渡しを書き、当てる処理は1つにする。

### 実装の順

1. 行の差分の型を`RowChange<B>`にし、各シェルの当てる処理を、要素の型を問わない形に直す。
   振る舞いは変わらないので、refactorとして1つのPRにする。
2. 見出し、最近のページとフォルダのページ、検索の結果を、1つずつ差分にする。
   1が済んでいれば、各並びは、マクロの型を1行足し、シェルで同じ処理を呼ぶだけになる。

### 範囲の外のこと

アトラスの並び（`Panel::AtlasCards`、`Panel::AtlasCallouts`、`Panel::AtlasLinks`、`Panel::AtlasLinkSuggestions`）は、このADRの範囲の外とする。
アトラスを通常のアウトラインと同じ形にする別のissueがあり、その後に扱うほうがよいためである。

次の3つは、差分とは別の話なので、別のissueにする。

- フォルダのページの数に上限を設けること
- このクライアントが自分で直したときに、最近のページとフォルダのページを読み直さないこと（今は、SSEの`pages-updated`のたびに`refresh_recent_pages`と`load_index_pages`が呼ばれる）
- Webのサイドバーの並びを仮想リストにすること

## Alternatives

### Alternative 1: 並びをパネルの中で丸ごと送り続ける

ADR 076のままにする。
長所は、コアにもシェルにも変更が要らないことである。
短所は、シェルが並びを2通りに扱い続けること、1000ページのフォルダを1ページの変更で送り直すことである。
並びの扱い方をアウトラインと揃えるという目的に合わないので、選ばなかった。

### Alternative 2: `ListChange<T>`をそのまま使い回す

`ListChange<Heading>`などを、そのまま境界に出す。
長所は、型を作るマクロが要らないことである。
`facet_generate`が止まるので、使えない。
`facet_generate`が同じジェネリックな型を複数の型引数で生成できるようになれば、この案に移れる。

### Alternative 3: 位置だけの操作と、値の並びを分けて送る

ジェネリックでない`ListOp`（`Insert { at }`、`Update { at }`など）と、値の並び（`items: Vec<Heading>`）を組にして送る。
`Insert`と`Update`が、`items`を前から順に1つずつ使う。
長所は、操作の型が1つで、Swift、Kotlin、C#でも橋渡しなしに1つの処理で当てられることである。
短所は、操作と値の対応が順番だけで決まり、型が対応を保証しないことである。
形もアウトラインの行と違い、考え方を揃えるという目的に合わない。

### Alternative 4: `facet_generate`を直す

型引数ごとに`ListChangeHeading`のような名前で生成するよう、`facet_generate`をforkするか、上流に変更を送る。
長所は、コアで`ListChange<T>`をそのまま使えることである。
短所は、forkなら版を上げるたびに追いかけること、上流なら取り込まれる時期が分からないことである。
この目的のためには重すぎるので、選ばなかった。

## Confidence Level

高い。
マクロの型が生成できることは、`facet_generate` 0.22.4で確かめた。
コアの差分の計算は、行で使っている`list_changes`をそのまま使う。

次のときは見直す。

- `facet_generate`が、同じジェネリックな型を複数の型引数で生成できるようになったとき。Alternative 2に移り、マクロを消せる。
- Swift、Kotlin、C#の橋渡しが、型を足すたびに負担になるとき。
- 検索の結果の`key`が、同じ結果の並びの中で重なるとき。たとえば、同じ項目が2回返るとき。

## Consequences

### 良い結果

- シェルは、アウトラインの行と4つの並びを、同じ1つの処理で扱う。
- 見出しを1文字直したとき、検索で1文字打ったとき、フォルダの1ページが変わったときに、変わった要素だけが送られる。
- シェルは、並びの要素ごとに、表示の状態（スクロールの位置、選択、仮想リストの部品）を保てる。Webのサイドバーの仮想リストの下準備になる。

### 悪い結果

- 生成される型の名前が`ListChange`から`RowChange`に変わり、すべてのシェルを直す。
- Swift、Kotlin、C#に、型ごとの橋渡しが要る。並びを足すたびに、3つの言語で数行ずつ増える。
- `SearchEntry.position`を外すので、`position`を読んでいるシェルを直す。

### 中立的な結果

- パネルの`Panel::Headings`、`Panel::RecentPages`、`Panel::IndexPages`、`Panel::Search`は残り、並びでないフィールドだけを持つ。
- 送るJSONの形は、アウトラインの行については変わらない。
- アトラスの並びは、別のissueのあとに、同じ形に移すかを決める。
