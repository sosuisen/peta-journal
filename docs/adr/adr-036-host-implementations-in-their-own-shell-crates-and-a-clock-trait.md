# ADR 036: Host Implementations in Their Own Shell Crates and a Clock Trait

## Status

Accepted

## Context

ネイティブのアプリは、4つある。WinUIとSwiftUIのシェルは、ジャーナルサーバーにHTTPで尋ねる（ADR 024、ADR 028）。AndroidとiOSのシェルは、journal-coreをアプリに組み込む（ADR 029、ADR 030、ADR 034）。

どのアプリも、`shell-native`の`NativeShell`を使う。`NativeShell`は、Cruxのコアを動かし、Effectを実行する。ジャーナルへの届け方は、`Host`というtraitで受け取る。

今は、アプリの形が2つある。

- WinUIとSwiftUIでは、`Host`の実装（`JournalServer`）、注入（`connect`）、BoltFFIの入口（`ffi.rs`）が、`shell-native`の中にある。
- AndroidとiOSでは、同じ3つが`shell-android`にある。`Host`の実装は`Embedded`である。ADR 030は、Androidの入口を「`shell-native`には足さない」とだけ決めた。`shell-native`の中にある入口は、そのまま残った。

この形には、問題が3つある。

1つ目は、土台と入口が同じクレートにあることである。`shell-native`は、4つのアプリの土台であり、2つのアプリの入口でもある。図に描くと、WinUIとSwiftUIの注入が見えない。

2つ目は、要らない依存である。`shell-android`は、`Host`と`NativeShell`のために`shell-native`に依存する。`shell-native`は、`JournalServer`のために`server-client`と`ureq`に依存する。AndroidとiOSは、HTTPを使わない。最終のバイナリに`ureq`が残っているかは、まだ確かめていない。

3つ目は、時計である。`Host`は、`call`と`events`のほかに、`now`と`sleep`を持つ。`JournalServer`と`Embedded`は、同じ`now`と`sleep`を別々に書いている。`now`は`chrono`で今の時刻を読み、`sleep`は`thread::sleep`を呼ぶ。時計は、ジャーナルへの届け方と関係がない。

## Decision

`Host`の実装を、`shell-native`から出す。時計を、`Clock`という別のtraitに分ける。

4つのアプリを、同じ形にする。上から、入口のシェル、土台、コア、APIのクレートである。入口のシェルは、BoltFFIの入口を持つ。`Host`の実装を作り、土台に注入する。`Host`の実装は、`call`と`events`を、APIのクレートに任せる。

`shell-native`を、2つに分ける。

- 新しいクレートへ出すものは、`JournalServer`、`connect`、`ffi.rs`、`ffi`のfeature、`boltffi`への依存、`server-client`への依存である。WinUIとSwiftUIは、このクレートに結びつく。
- 残るものは、`Host`と`Clock`のtrait、`SystemClock`、`NativeShell`、HTMLの関数、境界の型である。これが土台になる。土台を使うのは、Rustの中の依存だけになる。

`shell-android`は、もう1つの入口のシェルである。中身は変えない。

時計は、次のようにする。

- 新しいtraitの`Clock`が、`now`と`sleep`を持つ。
- 本物の時計の実装の`SystemClock`を、土台に1つだけ置く。
- `Host`には、`call`と`events`だけを残す。`Host`は、ジャーナルへの届け方だけを表す。
- `NativeShell::new`は、`Host`と時計の2つを受け取る。2つの入口は、どちらも`SystemClock`を渡す。
- テストは、偽の`Host`と偽の時計を、別々に渡す。

時計を先に分け、そのあとでクレートを分ける。時計の変更は、`shell-native`の中だけで済む。分けるときに動かすコードも減る。

クレートの名前は、ADR 037で決める。

## Alternatives

### Alternative 1: `server-client`への依存を、Cargoのfeatureの後ろに置く

`shell-native`に`http`のようなfeatureを作り、`JournalServer`と`server-client`への依存をその後ろに置く。`shell-android`は、featureなしで`shell-native`を使う。クレートを増やさずに、`ureq`を外せる。短所は、2つの系統が同じ形にならないことである。WinUIとSwiftUIの入口は、土台の中に残る。選ばなかった。

### Alternative 2: `now`と`sleep`を、`Host`のtraitの既定の実装にする

`Host`のtraitに、本物の時計を使う既定の実装を書く。`JournalServer`と`Embedded`から、同じコードが消える。変更は小さい。短所は、テスト用の`Host`が上書きを忘れても、コンパイルが通ることである。そのテストは、本物の時計で動く。遅くなり、結果が時刻で変わる。選ばなかった。

### Alternative 3: 今のままにする

変更がない。短所は、上の3つの問題が残ることである。入口に関数を足すたびに、2つの違う場所を直す。選ばなかった。

## Confidence Level

確信度は、時計については高く、分割については中程度である。

時計の変更は、Rustの中だけである。BoltFFIの入口は変わらない。アプリの側のコードは直さない。テストで確かめられる。

分割の確信が下がる理由は、3つある。

- AndroidとiOSの最終のバイナリに、今`ureq`が残っているかを、確かめていない。リンカーがすでに落としていれば、大きさは変わらない。その場合も、形がそろう利点は残る。
- WinUIとSwiftUIが結びつくライブラリが変わる。`boltffi pack`の設定、生成されるC#とSwiftの束ね、ビルドの手順を直す。SwiftUIのビルドは、Macでしか確かめられない。
- 型を生成する`codegen`のバイナリは、境界の型がある土台に残すつもりである。Kotlinの型の生成がどこにあるかは、分ける前に確かめる。

次のどれかが起きたら、この決定を見直す。

- BoltFFIが、依存するクレートの`export`を、最終のクレートから外に出せるようになる。そのときは、2つの入口にある同じ関数を、土台に1つだけ置く。
- 3つ目の届け方が出る。そのときは、入口のシェルをもう1つ作る。土台は変えない。
- 時計を、アプリの側から渡したくなる。今は、4つのアプリが同じ`SystemClock`を使う。

## Consequences

良い結果は、次のとおりである。

- 4つのアプリが、同じ形になる。図の1つの列を読めば、ほかの列も読める。
- 土台が、HTTPを知らなくなる。AndroidとiOSの依存から、`server-client`と`ureq`が消える。
- `shell-android`から、`chrono`への依存が消える。
- `Host`が小さくなる。新しい届け方を足すときに、時計を書かずに済む。
- テストが、偽の時計を`Host`と別に持てる。

悪い結果は、次のとおりである。

- クレートが1つ増える。
- `PetaShell`のtraitと`item_html`などの関数が、2つの入口に同じものとして残る。BoltFFIの`export`は、最終のクレートに書くためである。入口に関数を足すときは、両方に書く。
- `NativeShell::new`の引数が、1つ増える。
- WinUIとSwiftUIのビルドの配線を、1回直す。

中立的な結果は、次のとおりである。

- ADR 030の「`shell-native`には足さない」は、この決定で置き換わる。入口は、どちらの系統でも、土台の外にある。
- 「同じ処理をRustの側に寄せる」という計画の置き場所は、土台になる（`docs/plans.md`）。
- 図（`docs/frontend-architecture.svg`）を、分割のあとの形に描き直す。
