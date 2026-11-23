# ADR 082: Split the Answer Types into a Shared Part and a Server Part

## Status

Accepted

## Context

### 答えの型が2つずつある

サーバーが返す答えのJSONを、サーバーとクライアントが別々の型で扱っている。

| JSON | サーバー（`journal-core/src/dto.rs`） | クライアント（`client-core/src/model.rs`） |
|---|---|---|
| 項目 | `ItemDto`（書くだけ） | `Item`（読むだけ） |
| ページ | `PageDto` | `Page` |
| 検索の結果 | `SearchResultDto` | `SearchResult` |
| バックリンク | `BacklinkEntryDto` | `BacklinkEntry` |

ほかにも、#805の本文の表に挙げた型がある。

クライアントの型は、サーバーの型をコメントで指し、欄を手で写している。
JSONの名前の付け替え（`rename_all = "camelCase"`、`rename = "type"`）は、両方の型に書かれている。
名前のずれは、`client-api-embedded/tests/journal.rs`の結合テストが、本物のサーバーを動かして見つける。

### クライアントの欄は、サーバーの欄の部分集合である

項目では、次のようになっている。

| | `ItemDto` | `Item` |
|---|---|---|
| 欄 | `Item`の欄のすべてと、`parent_item_id`、`ast`、`url` | `id`、`page_id`、`body`、`level`、`properties`など |
| `properties`の値 | `serde_json::Value` | `PropertyValue`（文字列か、文字列の配列） |
| ほかの役目 | ない | ネイティブの境界を越える（facet、ADR 024）。画面の状態でもあり、メソッドを9個持つ |

`Page`と`PageDto`も同じ形である。
`PageDto`にだけ`url`がある。

`ItemDto`の`properties`は`serde_json::Value`である。
ただし、サーバーが入れるのは、文字列か、文字列の配列だけである（`properties_json`）。

### 変更の理由は2つある

ADR 081では、位置の型の変更の理由が1つだけだったので、1つの型にまとめた。
答えの型には、変更の理由が2つある。

- クライアントも使う欄が増える。
  サーバーの型とクライアントの型の両方が変わる。
- サーバーだけの欄が増える。
  `ast`や`url`は、MCPやほかのクライアントのために、サーバーの型にだけ足された。
  クライアントの型は変わらなかった。

1つの型にまとめると、2つ目の理由でも、クライアントの型が変わってしまう。
`ast`はネイティブの境界も越え、Swift、Kotlin、C#の型にも出る。
クライアントは、使わない欄に依存することになる。

### 確かめたこと

2026-11-24に、次のことを確かめた。

- サーバーの答えの型は、`JsonSchema`を持たない。
  MCPのスキーマには出ないので、型の形を変えてもスキーマは変わらない。
- 小さなプロジェクトで、`#[serde(flatten)]`を試した。
  - 共有の部分を`flatten`で持つ型を書くと、JSONは今と同じ平らな形になり、欄もすべて出た。
  - `serde_json::to_value`のオブジェクトから欄を絞ると、`?fields=`（`ItemFields::of`）と同じに絞れた。
  - 共有の部分の型だけで、サーバーの答えを読めた。
    サーバーだけの欄は、serdeが読み飛ばした。
  - 1万件を書く時間は、`flatten`なしの型と変わらなかった（約2ms）。
- #994（ADR 081）で、facetの型を`journal-api-types`に移した。
  ネイティブの型は、コメントのほかは変わらなかった。

## Decision

答えの型を、クライアントも使う部分と、サーバーだけの部分に分ける。
クライアントも使う部分を`journal-api-types`に置き、サーバーとクライアントの両方で使う。

### 共有の部分

`journal-api-types`に、`Item`と`Page`を置く。
欄は、今のクライアントの`Item`と`Page`の欄と同じにする。
JSONの名前の付け替えは、この型にだけ書く。

ネイティブの境界の書き方も、この型が持つ。
ADR 081の`ItemPosition`と同じく、`facet`の機能の中で`derive`する。

### サーバーだけの部分

サーバーの`ItemDto`は、共有の`Item`を`#[serde(flatten)]`で持ち、サーバーだけの欄を足す。

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    #[serde(flatten)]
    pub item: journal_api_types::Item,
    pub parent_item_id: Option<String>,
    pub ast: String,
    pub url: String,
}
```

`PageDto`も、同じ形で共有の`Page`と`url`を持つ。
JSONの形は変わらない。

### プロパティの値

`PropertyValue`を`journal-api-types`に移す。
JSONでは文字列か文字列の配列、bincodeではタグ付きの列挙型として書く。
今のクライアントの`PropertyValue`と同じである。

サーバーは、`properties_json`の値を`PropertyValue`に変えて、共有の`Item`に入れる。

一括の要求の`PropertyValueInput`は、JSONでは`PropertyValue`と同じ形である。
`PropertyValueInput`をやめて、`PropertyValue`にまとめる。
#994で足した`From<PropertyValue> for PropertyValueInput`も消す。

### クライアントのメソッド

`client-core`は、`journal-api-types`の型に、メソッドを直接書けない（孤児ルール）。
`Item`と`Page`のメソッドは、`client-core`のトレイトに移す。
使う側は、そのトレイトを`use`する。

### ほかの答えの型

`SearchResult`、`BacklinkEntry`などの答えの型も、同じ考え方で扱う。

- クライアントの欄がサーバーの欄と同じなら、共有の型を1つにする。
- クライアントの欄がサーバーの欄の部分集合なら、`Item`と同じく分ける。
- それ以外の形なら、作業の中で決める。

### `#[serde(default)]`

今のクライアントの`Item`と`Page`は、ほとんどの欄に`#[serde(default)]`を持つ。
理由は記録に残っていない。
サーバーは、すべての欄を書く。
共有の型に移すときに、外せるかを確かめる。
外せない欄があれば、その理由をコメントに書く。

### 作業の置き場所

作業は#805で行う。
#805の2つ目のタスクを、この決定に書き換える。
型の数が多いので、型ごとにPRを分けてよい。

## Alternatives

### Alternative 1: 共有の読むための型を置き、クライアントは`From`で自分の型に変える

`ItemDto`をそのまま`journal-api-types`に移し、サーバーとクライアントの両方で使う。
クライアントは、その型でJSONを読み、`From`で自分の`Item`に変える。

長所は、クライアントの`Item`が今のまま残ることである。
メソッドもトレイトに移さなくてよい。

短所は、クライアントに型が2つ残ることである。
読むための型と`Item`で、欄を2回書く。
読むための型は、クライアントが使わない`ast`や`url`も持つ。
選ばなかった。

### Alternative 2: クライアントの`Item`を、サーバーの`ItemDto`そのものにする

ADR 081の`ItemPosition`と同じく、1つの型にまとめる。

短所は、サーバーだけの欄が、クライアントの`Item`に入ることである。
`ast`はネイティブの境界も越え、Swift、Kotlin、C#の型にも出る。
サーバーだけの欄が増えるたびに、クライアントの型とネイティブの型も変わる。
変更の理由が2つあるので、選ばなかった。

### Alternative 3: 今のままにする

ずれは、結合テストが見つける。

長所は、作業が要らないことである。

短所は、JSONの名前の付け替えが2か所に残ることである。
クライアントの欄を足すときも、2つの型に書く。
選ばなかった。

## Confidence Level

中くらいである。
次のことを、まだ確かめていない。

- `Item`と`Page`の`#[serde(default)]`を外せるか。
- ほかの答えの型が、上の3つの形のどれに当たるか。
- メソッドをトレイトに移したときに、使う側の`use`がどれだけ増えるか。

次のときは、この決定を見直す。

- クライアントだけが持つ欄が、答えの型に要るようになったとき。
  クライアントの欄が、サーバーの欄の部分集合でなくなる。
  そのときは、Alternative 1の形にする。
- `#[serde(flatten)]`が、ほかの書き方（bincodeなど）でサーバーの答えを書くときに使えないと分かったとき。

## Consequences

### 良い結果

- JSONの名前の付け替えが、共有の部分の1か所になる。
- サーバーだけの欄を足しても、クライアントの型とネイティブの型は変わらない。
- クライアントも使う欄を足すときは、共有の型の1か所を直す。
- `PropertyValue`と`PropertyValueInput`が1つになる。
- 欄のずれは、結合テストを待たずに、コンパイラーが見つける。

### 悪い結果

- `Item`と`Page`のメソッドが、トレイトに移る。
  使う側は、トレイトを`use`する。
- `journal-api-types`に、ネイティブの境界の書き方を持つ型が増える。
- サーバーは、`properties_json`の値を`PropertyValue`に変える処理を持つ。

### 中立的な結果

- JSONの形は変わらない。
  MCP、todo、そのほかのクライアントには影響がない。
- `client-api-embedded`は、今はサーバーの答えをJSONにしてから`client-core`に渡している。
  共有の型ができたあとは、JSONを通さずに渡せるかもしれない。
  この決定では扱わない。
