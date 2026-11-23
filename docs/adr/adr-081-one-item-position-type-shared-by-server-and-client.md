# ADR 081: One Item Position Type Shared by Server and Client

## Status

Accepted

## Context

### 位置の意味が3か所に書かれている

項目の位置は、`first`、`last`、`after:<id>`の3種類である。
この意味は、今は3か所に書かれている。

| 場所 | 型 | 中身 |
|---|---|---|
| `client/libs/client-core/src/model.rs` | `ItemPosition` | `First`、`Last`、`After(String)` |
| `journal/journal-api-types/src/lib.rs` | `String` | `"first"`、`"last"`、`"after:<id>"`。意味はコメントだけにある |
| `journal/journal-core/src/domain/model.rs` | `ItemPosition` | `First`、`Last`、`After(Hmtid)` |

クライアントは、`ItemPosition::to_api`で型を文字列にして送る。
サーバーは、`ItemPosition::parse`で文字列を型に戻す。
`"after:"`という書き方の決まりは、クライアントの`to_api`と`parse`、サーバーの`parse`の3か所にある。

`journal-api-types`の要求の型で`position`を持つのは、次の5か所である。
すべて`String`か`Option<String>`である。

- `MoveToJournalInput`
- `CreateItemInput`
- `MoveItemsInput`
- `BatchOpInput::Create`
- `BatchOpInput::Move`

### 一括の要求だけ、共有の型を使っていない

クライアントは、ほかの要求を`journal-api-types`の型で送る（`client-core/src/rest.rs`）。
一括（`ApplyBatch`）だけは、クライアントの`BatchOp`をJSONにして送る。
`BatchOp`と共有の`BatchOpInput`は、同じJSONになる。
ただし、名前の付け替え（`tag = "op"`、`camelCase`）を2か所に書いている。
これは#805の1つ目のタスクである。

### クライアントの`ItemPosition`は、2つの相手に使われる

1つ目の相手は、RESTのJSONである。
ここでは`"after:<id>"`の文字列になる。

2つ目の相手は、ネイティブのシェルとの境界である（ADR 024）。
facet（`facet_typegen`の機能）で、Swift、Kotlin、C#の型を作る。
境界はbincodeなので、タグ付きの列挙型として書く（`ItemPositionWithTag`）。
`Serialize`と`Deserialize`は、`is_human_readable()`を見て、この2つの書き方を切り替える。

### 変更の理由は1つである

`ItemPosition`が変わる場面を考えた。
仮に、RESTのみを対象とする変更の理由が、境界の書き方にも影響してしまうならば、1つの型にまとめるのは良くないと言える。

- 位置の種類が増える（たとえば`Before(id)`）。
  RESTの書き方も、境界の書き方も変わる。
- RESTの書き方を変える。
  MCPと、ほかのクライアント（todoなど）も変わる。
  位置の型だけの変更ではなく、REST全体の決定になる。
- 境界の書き方を変える（bincodeをやめる）。
  ネイティブの境界を越えるすべての型が変わる。
  位置の型だけの変更ではなく、境界全体の決定になる。

主要な変更の理由は1つ目である。
RESTの書き方と境界の書き方は、位置の意味から機械的に決まる表現であり、あまり変更されるとは言えない。
変更の理由が1つなので、単一責任の原則からは、1つの型にまとめるのが正しい。

## Decision

位置の型を`journal-api-types`に1つだけ置き、サーバーとクライアントの両方で使う。

### 共有の`ItemPosition`

`journal-api-types`に`ItemPosition`を置く。
中身は`First`、`Last`、`After(String)`である。
`"after:"`の決まりは、この型の`Serialize`と`Deserialize`の1か所にだけ書く。

JSONの形は今と同じにする。
`"first"`、`"last"`、`"after:<id>"`の文字列である。
そのため、MCP、todo、そのほかのクライアントには影響がない。

### ネイティブの境界の書き方も、この型が持つ

今のクライアントの`ItemPosition`が持つ境界の書き方を、共有の型に移す。

- facetの`derive`と`repr(C)`を付ける。
  `journal-api-types`に`facet`の機能を足し、その中でだけfacetに依存する。
  `client-core`の`facet_typegen`は、この機能を入れる。
  サーバーは入れないので、サーバーのビルドにfacetは入らない。
  `peta-atlas`も、同じ形で`facet`の機能を持っている。
- `is_human_readable()`で、JSONの文字列とbincodeのタグ付きの列挙型を切り替える。
- facetの版は、`client-core`と同じ版に固定する（今は`=0.46.5`）。

### MCPのスキーマ

`ItemPosition`の`JsonSchema`は手で書く。
スキーマでは、今と同じく文字列として示す。
説明も、今のコメント（`'first', 'last', or 'after:<id>'`）と同じにする。

### 差し替える場所

- `journal-api-types`の5か所の`position`を、`ItemPosition`か`Option<ItemPosition>`にする。
- `client-core`の`ItemPosition`を消し、共有の型を使う。
  `to_api()`の呼び出しは要らなくなる。
- `client-core`の`BatchOp`を消し、共有の`BatchOpInput`を使う。
  `BatchOpInput`の`task`と`properties`の型も、クライアントの`TaskState`と`PropertyValue`に合わせられるかを、作業の中で確かめる。
- サーバーの`journal-core`は、共有の`ItemPosition`を受け取る。
  ドメインの`ItemPosition`（`After(Hmtid)`）は残す。
  `After`のIDが`Hmtid`として正しいかは、今までどおりサーバーが確かめる。
  文字列を解く`ItemPosition::parse`は、共有の型からドメインの型への変換に置き換える。

### 作業の置き場所

作業は#805で行う。
#805の1つ目のタスクを、この決定に書き換える。
答えの型を共有するか（#805の2つ目のタスク）は、このADRでは決めない。

## Alternatives

### Alternative 1: 共有の型はRESTだけ、境界用の型はクライアントに残す

`journal-api-types`の`ItemPosition`は、RESTの書き方だけを持つ。
ネイティブの境界の書き方は、クライアントの別の型が持ち、`From`で行き来する。

長所は、`journal-api-types`がネイティブの境界の事情を持たないことである。
インターフェース分離の原則に沿って見える。

短所は、位置の種類を足すときに、共有の型、境界用の型、`From`の3か所を直すことである。
変更の理由が1つしかないので、分けても変更は1か所に収まらない。
書く場所が増えるだけなので、選ばなかった。

### Alternative 2: 今のままにする

位置を文字列で送り、両側で`parse`する。
一括は、クライアントの`BatchOp`のまま送る。

長所は、作業が要らないことである。
名前のずれは、`client-api-embedded/tests/journal.rs`のテストが見つける。

短所は、`"after:"`の決まりが3か所に残ることである。
位置の種類を足すと、3つの型と、2つの`parse`と、1つの`to_api`を直す。
選ばなかった。

### Alternative 3: 外から境界用のトレイトを実装する

共有の型には何も足さず、`client-core`から`Facet`を実装する。

Rustでは、トレイトの実装は、型のクレートかトレイトのクレートの中でしか書けない（孤児ルール）。
`client-core`は、`journal-api-types`の型に`Facet`を実装できない。
serdeには`remote`の仕組みがあるが、facetに同じ仕組みがあるかは確かめていない。
facetの型の生成にも使えるかが分からないので、選ばなかった。

## Confidence Level

中くらいである。
次の2つを、まだ確かめていない。

- `facet_generate`が、`journal-api-types`の型を、今と同じSwift、Kotlin、C#の型にするか。
  型の名前や置き場所が変わると、ネイティブのシェルのコードも変わる。
- `BatchOpInput`の`task`と`properties`を、クライアントの型に揃えられるか。

次のときは、この決定を見直す。

- ネイティブの境界の書き方（bincode）を変えるとき。
  境界の書き方が`journal-api-types`にあるので、ここも直すことになる。
- 位置の型だけで、RESTか境界の片方の書き方を変える必要が出たとき。
  変更の理由が2つになるので、Alternative 1が正しくなる。

## Consequences

### 良い結果

- `"after:"`の決まりが1か所になる。
- 位置の種類を足すときは、共有の型とサーバーのドメインの型を直す。
  漏れは、`match`のコンパイルエラーで見つかる。
- 一括の要求も、ほかの要求と同じく共有の型で送る。
  名前の付け替えが1か所になる。
- クライアントから`to_api()`の呼び出しが消える。

### 悪い結果

- `journal-api-types`が、機能フラグの中でfacetに依存する。
  facetの版を上げるときは、`journal-api-types`も直す。
- `journal-api-types`に、ネイティブの境界の書き方（bincodeのタグ付きの形）が入る。
  サーバーは使わないコードである。
- `JsonSchema`を手で書く。

### 中立的な結果

- サーバーのドメインの`ItemPosition`は残る。
  共有の型は通信の形、ドメインの型は確かめたIDを持つ型である。
- 答えの型を共有するかは、#805で別に決める。
  共有すると決めたときは、`Item`や`PropertyValue`も、この決定と同じ形で`journal-api-types`に置くことになる。
