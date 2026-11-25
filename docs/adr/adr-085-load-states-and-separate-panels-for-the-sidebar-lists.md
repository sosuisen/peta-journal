# ADR 085: Load States and Separate Panels for the Sidebar Lists

## Status

Accepted

## Context

### サイドバーには、別々に届く3つの一覧がある

コアの`SidebarView`（`client-core/src/view.rs`）は、サイドバーの3つの一覧を持つ。
3つは、コアの中でそれぞれ別の時に届く。

| 一覧 | フィールド | 要素の型 | いつ届くか |
|---|---|---|---|
| フォルダのページ | `index_pages` | `SidebarPage` | ページの読み込みが終わったあと。`load_index_pages`がRESTを1〜2回呼ぶ |
| 目次 | `headings` | `Heading` | ページの項目が届いたとき。行から作るので、RESTは呼ばない |
| 最近のページ | `recent_pages` | `SidebarPage` | 起動時と、SSEの再接続時。`refresh_recent_pages`がRESTを1回呼ぶ |

### シェルは「読み込み中」と「空」を区別できない

3つの一覧は、どれも`Vec`である。
最初の`changes()`は、リセットとして全パネルを送る（`client-core/src/changes/mod.rs`の`panels`）。
そのため、一覧は、届く前でも空の`Vec`としてシェルに届く。
シェルは、「まだ届いていない」と「一覧が空」を区別できない。

別のページへ移ったときは、さらに悪い。
`model.current_page`は、新しいページの読み込みが終わったときに`set_current_page`で差し替わる。
それまでの間、目次とフォルダのページは、前のページのものを見せ続ける。

失敗も伝わらない。
`refresh_recent_pages`は、RESTの失敗を捨てる。
`load_index_pages`とページの読み込みは、失敗をログに出すだけである。

本文のページには、前例がある。
`Screen::Page`の`PageView`は、読み込みが終わるまで`None`である。
検索には、別の前例がある。
`SearchView.searching`は、前の結果を残したまま、`bool`で読み込み中を表す。

### サイドバーは1つのパネルで、1つの枠で描かれる

3つの一覧は、どれも`Panel::Sidebar`に入っている。
どれか1つが変わると、コアはサイドバー全体を1つの値として送り直す。
Webのシェルは、#1110でパネル単位の購読にした。
しかし、パネルが1つなので、目次の見出しを1つ直すだけで、最近のページの枠も描き直される。

Webのシェルは、#1111でフレームごとに`Frame`（`Suspense`とError Boundaryの組）を置いた。
サイドバーは1つの`Frame`で包まれている。
目次で例外が出ると、最近のページも一緒に消える。

`Panel::Sidebar`を読むシェルは、次のとおりである。

| シェル | 読む場所 |
|---|---|
| Web（`outliner-react`） | `Sidebar.tsx` |
| Emacs | `apps/emacs/outliner/src/protocol.rs`の`SidebarView::of`、`shell.rs`の`tell_sidebar`と`tell_rows` |
| Android | `Applied.kt` |
| iOSとmacOS | `swift-shared`の`Applied.swift` |
| Windowsの付箋紙 | `PetaBorder/Applied.cs` |

Emacsの`tell_rows`は、`sidebar.current_page_id`を、アウトラインのページIDとしても読んでいる。

### 境界の型のジェネリックは、1種類の引数でしか使えない

境界の型は、`facet_generate`がTypeScript、Swift、C#、Kotlinに書き出す。
2026-11-25に、一時的なワークツリーで、ジェネリックな`Loadable<T> { Loading, Loaded(T), Failed }`を試した。
`Loadable<Vec<SidebarPage>>`と`Loadable<String>`の2種類で使うと、TypeScriptの生成が次のエラーで止まった。

```
unsupported generic type: Loadable<String>, the type may have already been used with different parameters
```

1種類だけで使うと、4つの言語とも生成できた。
ただし、どの言語でも、`T`を埋めたジェネリックでない`Loadable`になった。
いまある`Changes<HtmlBody>`が動くのも、1か所でしか使っていないためである。
3つの一覧は、要素の型が`SidebarPage`と`Heading`の2種類ある。
そのため、ジェネリックな型では、3つの一覧に読み込みの状態を付けられない。

## Decision

サイドバーの3つの一覧に、読み込みの状態を付ける。
状態は、一覧の中身を持たない型`LoadState`で表し、一覧の横に置く。
あわせて、`Panel::Sidebar`を一覧ごとの3つのパネルに分ける。

### `LoadState`

`client-core/src/view.rs`に、次の型を足す。

```rust
pub enum LoadState {
    Loading,
    Loaded,
    Failed,
}
```

`LoadState`は、中身の型を持たない。
そのため、ジェネリックを使わずに、どの一覧にも同じ型を使える。

### 3つのパネル

`ViewModel`の`sidebar: SidebarView`を、次の3つのフィールドに分ける。
`Panel`も、同じ3つに分ける。
`Panel::Sidebar`はなくす。

```rust
pub struct IndexPagesView {
    pub header: String,
    pub path: String,
    pub sort: IndexSort,
    pub parent_path: Option<String>,
    pub pages: Vec<SidebarPage>,
    /// The page on screen, which the list marks.
    pub current_page_id: Option<String>,
    pub state: LoadState,
}

pub struct HeadingsView {
    pub headings: Vec<Heading>,
    pub state: LoadState,
}

pub struct RecentPagesView {
    pub pages: Vec<SidebarPage>,
    pub state: LoadState,
}
```

パネルは、`Panel::IndexPages`、`Panel::Headings`、`Panel::RecentPages`とする。
フィールドの名前は、実装のときに今の`SidebarView`の名前に合わせて直してよい。

### `Loading`にする場面

コアは、次の場面で`Loading`にする。

- 最近のページは、起動してから最初の答えが届くまで`Loading`にする。
  SSEの再接続による読み直しの間は、`Loaded`のまま前の一覧を残す。
  最近のページは、読み直しの前後でほとんど変わらないためである。
- フォルダのページは、最初の答えが届くまでと、別のフォルダのページに移ったときに`Loading`にする。
  同じフォルダの中で移ったときは、`Loaded`のまま前の一覧を残す。
  一覧の中身が変わらないためである。
- 目次は、表示するページの項目が届くまで`Loading`にする。
  ページを移るたびに`Loading`になる。

`Loading`と`Failed`のとき、一覧は空にする。
前のフォルダや前のページの一覧は、新しいページには正しくないためである。
この決まりは、型では守れないので、コアのテストで守る。

### 失敗

コアは、前の一覧がないときに読み込みが失敗したら、`Failed`にする。

- 最近のページは、最初の読み込みが失敗したら`Failed`にする。
  再接続での読み直しが失敗したときは、`Loaded`のまま前の一覧を残す。
- フォルダのページは、最初の読み込みか、別のフォルダへ移ったあとの読み込みが失敗したら`Failed`にする。
- 目次は、ページの読み込みが失敗したら`Failed`にする。

`refresh_recent_pages`は、失敗を捨てずに、イベントでコアに伝える。
読み直しは、次の再接続か、次のページの移動に任せる。

### 文書でないページ

日誌などの文書でないページでは、今もフォルダのページと目次は空である。
このとき、フォルダのページと目次は、`Loaded`で空の一覧にする。
シェルは、`Loaded`で空なら、今と同じく何も出さない。

### シェル

Webのシェルは、3つの一覧の枠をそれぞれ別のコンポーネントにし、それぞれを`Frame`で包む。
各コンポーネントは、自分のパネルだけを購読する。
`Loading`ではスケルトンを、`Failed`では短い文言を出す。
#1111の確認で、読み込み中のサイドバーの背景が白いことがわかった。
灰色の背景と右の線は、サイドバーの中身が描いているためである。
背景と線は、`<aside>`の側で描くように直す。

Emacs、Android、iOSとmacOS、Windowsの付箋紙のシェルは、`Panel::Sidebar`の代わりに3つのパネルを読む。
Emacsの`tell_rows`は、アウトラインのページIDを`IndexPagesView`の`current_page_id`から読む。
`Loading`と`Failed`をどう描くかは、各シェルが決める。
描かないシェルは、今と同じく空の一覧として扱ってよい。

## Alternatives

### Alternative 1: `Option<Vec<SidebarPage>>`

`None`を「まだ届いていない」とする。
いちばん小さな変更である。
しかし、失敗を表せない。
今の`refresh_recent_pages`は失敗を捨てるので、失敗するとずっと`None`のままになる。
また、目次の`Vec<Heading>`には、別の`Option`が要る。

### Alternative 2: ジェネリックな`Loadable<T>`

`Loadable<Vec<SidebarPage>>`と`Loadable<Vec<Heading>>`のように、中身を持つ列挙型を1つの定義で使い回す。
型で矛盾した状態を作れない。
しかし、Contextに書いたとおり、`facet_generate`は2種類の引数を扱えず、生成が止まる。
1種類だけなら生成できるが、シェルの側ではジェネリックでない型になり、使い回せない。

### Alternative 3: 一覧ごとの列挙型

`PageList { Loading, Loaded(Vec<SidebarPage>), Failed }`と`HeadingList { Loading, Loaded(Vec<Heading>), Failed }`を作る。
型で矛盾した状態を作れない。
シェルは1つの値を見るだけで描き分けられる。
しかし、読み込みの状態が要る用途が増えるたびに、同じ形の型が増える。
被リンクやアトラスのカードにも付けると、4つのシェルがそのたびに型を1つずつ覚える。
また、`Loading`が中身を持たないので、前の一覧を見せながら読み直す描き方ができない。

### Alternative 4: `bool`の旗

`SearchView.searching`と同じく、`recent_loading: bool`のような旗を足す。
前の一覧を残したまま、読み込み中を示せる。
しかし、失敗を表せない。

### Alternative 5: パネルを分けない

`SidebarView`の中に`LoadState`を足すだけにする。
変更は小さい。
しかし、どれか1つの一覧が変わると、サイドバー全体が送り直され、描き直される。
Webのシェルで、1つの枠の例外がほかの枠を消す問題も残る。
境界の型を変えるのは同じなので、ここで一緒に分ける。

## Confidence Level

中程度から高い。
読み込みの状態をコアが決めることと、パネルを分けることには、強い確信がある。
本文のページと検索に前例があり、シェルが推測しなくて済むためである。

`LoadState`を一覧の横に置く形には、中程度の確信がある。
矛盾した状態をテストで防ぐことになるためである。
次のときは、この決定を見直す。

- `facet_generate`が、ジェネリックな型を複数の引数で使えるようになったとき。
  中身を持つ`Loadable<T>`に移すと、矛盾した状態を型で防げる。
- 矛盾した状態の不具合が、コアのテストをすり抜けて起きたとき。

## Consequences

### 良い結果

- シェルは、読み込み中、空、失敗を描き分けられる。
- 別のページへ移っても、前のページの目次とフォルダの一覧が残らない。
- 最近のページの最初の読み込みの失敗が、捨てられずにシェルに届く。
- Webのシェルで、一覧が変わった枠だけが描き直される。
  1つの枠の例外は、その枠の中で止まる。
- `LoadState`は、ほかの一覧にもそのまま使える。

### 悪い結果

- 境界の型が変わるので、5つのシェル（Web、Emacs、Android、iOSとmacOS、Windowsの付箋紙）をすべて直す。
  macOSとiOSのビルドとテストは、Windowsの手元では確かめられない。
- `Loading`なのに一覧に中身がある、といった矛盾した状態を、型では防げない。
  コアのテストで守る。
- パネルが2つ増える。
  ネイティブのシェルの`Applied`は、パネルの種類を網羅して扱っているので、それぞれに足す。

### 中立的な結果

- `SidebarPage`という名前は、サイドバーに限らない中身を持つが、このADRでは変えない。
  名前を変えるかは、別のissueで決める。
- 被リンクやアトラスのカードに読み込みの状態を付けるかは、このADRでは決めない。
