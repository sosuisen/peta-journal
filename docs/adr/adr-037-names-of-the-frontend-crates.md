# ADR 037: Names of the Frontend Crates

## Status

Accepted

## Context

`frontend-rust`のクレートの名前は、作った順に付けてきた。今は、名前が中身と合わないものがある。

- パッケージの名前は、`peta-`で始まる。`peta-`は、製品の名前である。フロントエンドの側か、ジャーナルの側かは、読めない。ジャーナルの側のパッケージは、`journal-`で始まる（`journal-core`）。
- フォルダーの名前は、パッケージの名前から接頭辞を外している（`crates/core`が`peta-core`）。2つの名前を、頭の中で結びつける必要がある。
- `shell-android`は、AndroidとiOSの両方が使う（ADR 034）。
- `shell-leptos`のパッケージの名前は、`peta-frontend`である。Webのシェルだけを指すのに、フロントエンドの全体に読める。
- `server-client`は、「サーバーとクライアント」という対の言葉に読める。中身は、ジャーナルサーバーへHTTPで尋ねるクライアントである。
- `embedded-journal`は、ジャーナルの側のクレートに読める。中身は、フロントエンドからjournal-coreを呼ぶ届け方である。

ADR 036は、`shell-native`を2つに分ける。新しいクレートに、名前が要る。この機会に、すべての名前をそろえる。

## Decision

フロントエンドのクレートの名前を、次のようにする。

| 今のフォルダー | 今のパッケージ | 新しい名前 | 中身 |
|---|---|---|---|
| `crates/core` | `peta-core` | `client-core` | 画面を持たないロジック |
| `crates/shell-native` | `peta-shell-native` | `client-native-bridge` | `Host`と`Clock`のtrait、`NativeShell`、HTMLの関数 |
| （新しく作る） | | `client-shell-http` | `JournalServer`、`connect`、BoltFFIの入口。WinUIとSwiftUIが使う |
| `crates/shell-android` | `peta-shell-android` | `client-shell-embedded` | `Embedded`、BoltFFIの入口。AndroidとiOSが使う |
| `crates/shell-emacs` | `peta-shell-emacs` | `client-shell-emacs` | Emacsのシェル |
| `crates/shell-leptos` | `peta-frontend` | `client-shell-leptos` | Webのシェル |
| `crates/server-client` | `peta-server-client` | `client-api-http` | `ApiOp`を、HTTPでジャーナルサーバーへ送る |
| `crates/embedded-journal` | `peta-embedded-journal` | `client-api-embedded` | `ApiOp`を、同じプロセスのjournal-coreで実行する |
| `crates/page-links` | `peta-page-links` | `client-page-links` | ページへのリンクの行き先と、コピーのHTML（ADR 027） |

名前の決まりは、次のとおりである。

- `client-`で始まる名前は、フロントエンドの側である。`journal-`で始まる名前は、ジャーナルの側である。
- 新しい名前は、パッケージの名前である。フォルダーも同じ名前にする（`crates/client-core`）。
- パッケージの名前に、`peta-`を付けない。
- 接頭辞のあとに、役割の言葉を置く。`core`は、画面を持たないロジックである。`api`は、ジャーナルへの届け方である。`shell`は、画面とEffectの実行である。`shell`と`core`は、Cruxの言葉のままにする。
- `client-shell-`で始まる名前は、1つの完成したシェルだけに付ける。`client-native-bridge`は、単体では動かない土台なので、`shell`と呼ばない。`bridge`は、Cruxの言葉である（`BridgeError`）。
- `client-shell-http`は`client-api-http`を使い、`client-shell-embedded`は`client-api-embedded`を使う。`http`と`embedded`は、ジャーナルへの届け方である。OSの名前は入れない。
- 同じクレートを、2回改名しない。途中の名前を作らず、最初から新しい名前にする。

## Alternatives

### Alternative 1: 接頭辞を`frontend-`にする

リポジトリのフォルダーの名前（`frontend-rust`）とそろう。短所は、長いことである。`frontend-shell-embedded`のように、名前が3つの言葉を超えて読みにくい。選ばなかった。

### Alternative 2: 接頭辞を`front-`、`ui-`、`fe-`にする

短い。短所は、どれも言葉として弱いことである。`front-`と`fe-`は略した形である。`ui-`は、画面を持たないコアに合わない。選ばなかった。

### Alternative 3: `peta-`のままにする

改名が少ない。短所は、決まりを知らないと、どちらの側かが読めないことである。選ばなかった。

### Alternative 4: すべてを`shell-`と呼ぶ

Cruxの見方では、コアの外はすべてシェルである。短所は、`core`との区別のほかに、何も言わなくなることである。土台と、APIのクレートと、完成したシェルが、同じ言葉になる。選ばなかった。

### Alternative 5: 届け方の言葉を`com`にする

`communication`の略である。短い。短所は、WindowsのCOM（Component Object Model）と紛れることである。WinUIのシェルがあるので、紛れやすい。`api`は、コードの中の言葉（`ApiOp`、`ApiResult`）とそろう。選ばなかった。

### Alternative 6: クレートごとの、ほかの名前

- `client-api-http`には、`peta-client-http`、`journal-http-client`、`http-client`を考えた。`journal-`で始まる名前は、ジャーナルの側に読める。
- `client-api-embedded`には、`peta-client-embedded`、`journal-embedded-client`、`journal-embedded`を考えた。短所は同じである。
- `client-shell-embedded`には、`shell-embedded`（`peta-shell-embedded`）を考えた。接頭辞の決まりに合わない。
- `client-native-bridge`には、`client-native`、`client-bridge`、`client-shell-bridge`を考えた。`client-native`は、役割を言わない。`client-bridge`は、ネイティブのアプリだけの土台であることを言わない。`client-shell-bridge`は、3つ目の種類のシェルに見える。

どれも選ばなかった。

## Confidence Level

確信度は高い。

決まりは、3つの言葉（側、役割、種類）だけでできている。9つのクレートのすべてに、当てはまる。改名は、動作を変えない。ビルドとテストで確かめられる。

次のどれかが起きたら、この決定を見直す。

- フロントエンドを、別のリポジトリに分ける（`docs/plans.md`の「リポジトリの構成」）。そのときは、`client-`が要るかを考える。
- クレートを、crates.ioに公開する。`client-core`のような名前は、公開の場では広すぎる。
- ジャーナルの側に、クライアントと呼ぶものが出る。

## Consequences

良い結果は、次のとおりである。

- 名前から、側と役割が読める。
- フォルダーとパッケージが、同じ名前になる。探すときに、名前を変換せずに済む。
- 入口のシェルと、それが使うAPIのクレートが、同じ言葉で結びつく（`http`、`embedded`）。
- OSの名前が、Rustのクレートから消える。iOSを足したときのような、名前のずれが起きにくい。

悪い結果は、次のとおりである。

- 9つのクレートのうち、8つを改名する。依存、`use`、ビルドの配線、文書を直す。
- 結びつくライブラリの名前が変わる。WinUI、SwiftUI、Android、iOSのビルドの手順と、Emacsの実行ファイルの名前、Webのwasmの名前を直す。
- 改名は、段に分けて行う。その間は、`peta-`と`client-`の名前が混ざる。
- ADR 036より前のADRは、古い名前で書いてある。書いた時点の記録なので、書き換えない。古い名前は、上の表で引く。

中立的な結果は、次のとおりである。

- ジャーナルの側のフォルダーの名前（`journal-server-rust/crates/core`）と`hmtid`は、この決定では変えない。別に決める（`docs/plans.md`）。
- リポジトリの直下のフォルダー（`frontend-rust`、`frontend-winui`など）は、変えない。
- 改名の順序は、`docs/plans.md`の「実行の順序」にある。
