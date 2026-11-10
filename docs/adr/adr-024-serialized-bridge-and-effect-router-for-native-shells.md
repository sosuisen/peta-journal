# ADR 024: Serialized Bridge and Effect Router for Native Shells

## Status

Accepted

## Context

ADR-011は、WinUI3とSwiftUIの編集シェルを作る前提で、`Event`をデータだけにすると決めた。その移行は2026-11-10に終わり、`Event::Continue(Thunk)`はなくなった。ADR-011は、シェルがコアを呼ぶ方式と`ViewModel`のシリアライズを、別のADRに残した。このADRは、それを決める。

シェルは今、2つある。`shell-leptos`は、WASMの中で`Core`を直接呼ぶ。`shell-emacs`は、Rustのプロセスが`Core`を持ち、Emacsとは標準入出力のJSONの行でやり取りする。どちらもRustなので、`Event`、`Effect`、`ViewModel`をRustの型のまま使っている。

SwiftUIとWinUI3のシェルは、Rustではない。コアとの間に、言語をまたぐ境界が要る。境界を通る型は3つある。シェルが送る`Event`、コアが頼む`Effect`とその答え、画面を表す`ViewModel`である。

SwiftUIのシェルは、当面macOSだけを対象にする。iOSとiPadOSは、このADRでは扱わない。macOSでは、コアを同じプロセスに入れることも、別のプロセスとして起動することもできる。

今の型は、境界を通る準備ができていない。`Event`は`Deserialize`を実装していない。`ViewModel`は`Serialize`を実装しておらず、アトラスのカードやページの項目など、コアの内部の型を多く含む。`Effect`の操作の型は、すでに`Serialize`と`Deserialize`を実装している。

境界の置き方は、大きく2つある。1つは、Cruxのシリアライズの境界である。`Event`、`Effect`、`ViewModel`をそのまま境界に出し、SwiftとC#の型を生成する。もう1つは、`shell-emacs`の形である。Rust側にシェルを置き、`Effect`の実行をRustで済ませ、画面に要るものだけを狭いインターフェースで見せる。

`ViewModel`は、変更のたびに全体を作り直す。`shell-emacs`は、前回との差分をRust側で求めて送っている。長いページで1文字ごとに全体を境界に通すと、入力が遅くなるおそれがあった。

シェルは2つ予定している。境界のRust側は、SwiftUIとWinUI3で共有したい。シェルごとに書く量が、方式を選ぶ基準の1つになる。

2026-11-11に、捨てるブランチで3つを確かめた。

第一に、`ViewModel`の全体を境界に通す費用である。今ある最大のページは1063項目である（ADR-018）。1000項目のページで、bincodeに書く時間は0.3ms、大きさは574KB、Swiftで読む時間は1.3msだった。JSONでは、書く時間が2.8ms、大きさが1.0MB、Swiftの`JSONDecoder`で読む時間が9.7msだった。項目の本文は約60文字で、ASTは段落1つの形で近似した。Swiftで読む時間は、手で書いた読み取りで測った。

第二に、コアを変える量である。`ViewModel`の`Serialize`は、7ファイル、25行の変更で通った。`Event`の`Deserialize`には、少なくとも8つの型への追加が要る。プロパティの値は`serde_json::Value`で、bincodeはこれを読み戻せなかった。

第三に、`Effect`をRust側で実行する仕組みである。Cruxは、0.16.0でミドルウェアを入れ、0.19.0で`EffectRouter`に置き換えた。`EffectRouter`は、`Effect`の種類ごとに行き先を決める。コアを`EffectRouter`で包み、APIの答えを別のスレッドから返すテストは、コアを変えずに通った。

Swiftの型の生成、SSEのように何度も答える流れ、Xcodeへの組み込み、SwiftUIが長いページを描き直す時間は、確かめていない。

## Decision

Cruxのシリアライズの境界を使い、SwiftUIとWinUI3のシェルが、同じプロセスの中でコアを呼ぶ方式を採用する。API、SSE、タイマーの`Effect`は、Cruxの`EffectRouter`を使ってRust側で実行する。

**境界**

- `Event`、`ViewModel`、シェルが実行する`Effect`を、bincodeで通す。
- Swiftの型とC#の型は、Cruxの型の生成で作る。手では書かない。
- `ViewModel`は、描画のたびに全体を通す。差分は、シェルのUIの仕組みに任せる。

**Rust側が受け持つこと**

- ネイティブのシェルのためのcrateを新しく作る。このcrateが、`Core`を`EffectRouter`で包む。
- `Api`、`Sse`、`Timer`は、Rustの中のレーンで実行する。`Render`、`Ui`、`Navigate`は、シリアライズのレーンでシェルへ送る。
- このcrateは、SwiftUIとWinUI3で共有する。HTTPの実行で`shell-emacs`と共有できるコードは、作りながら切り出す。
- ミドルウェア（`crux_core::middleware`）は使わない。Cruxが`EffectRouter`に置き換えたためである。

**コアの変更**

- `Event`に`Deserialize`を、`ViewModel`に`Serialize`を実装する。
- プロパティの値を、`serde_json::Value`から、文字列か文字列のリストを表す型に変える。bincodeが`Value`を読み戻せないためである。
- コアは、`Effect`を誰が実行するかを知らないままにする。`shell-leptos`と`shell-emacs`は、これまでどおり`Core`を直接呼ぶ。

**このADRで決めないこと**

iOSとiPadOSは扱わない。WinUI3のシェルの詳細も決めない。RustのライブラリをSwiftから呼ぶための道具（BoltFFIなど）は、Walking Skeletonで決める。

## Alternatives

### Alternative 1: Cruxの`Bridge`だけを使い、すべての`Effect`をシェルが実行する

Cruxの既定の形である。`EffectRouter`に頼らないので、Cruxの中で最も安定したAPIだけを使う。入出力は、OSの道具（`URLSession`など）で行う。しかし、APIの呼び出し、SSEの再接続、タイマーを、SwiftとC#でそれぞれ書くことになる。`ApiOp`をHTTPの要求に直す処理は、コアの`rest.rs`にある。シェルがこれを使うには、境界をもう1つ作るか、同じ処理を2つの言語で書き直す必要がある。相手は手元のjournal-serverだけなので、OSの道具を使う利点も小さい。そのため、選ばなかった。

### Alternative 2: Rust側のシェルをライブラリで組み込み、狭いメッセージだけを通す

`shell-emacs`の`shell.rs`と同じ形である。Rust側が`Core`を持ち、`Effect`を実行し、前回との差分をJSONのメッセージで送る。コアを変えずに済み、Cruxの新しいAPIにも頼らない。境界を通るデータは最も小さく、シェルは変わった行だけを書き換えられる。しかし、メッセージの型をSwiftとC#で手で書くので、Rustの型とずれてもコンパイルでは気づけない。差分を求める仕組みと、メッセージの定義も、自分で作って保つことになる。全体を通す費用が小さいと分かったので、選ばなかった。SwiftUIの描き直しが遅いと分かったときは、この案に戻る。

### Alternative 3: Rust側のシェルを別のプロセスにする

`shell-emacs`の全体と同じ形である。標準入出力でJSONの行をやり取りする。既存のコードを最も多く使い回せる。しかし、Alternative 2の短所に加えて、プロセスの起動、終了、異常終了の扱いが要る。iOSとiPadOSでは使えない。そのため、選ばなかった。

### Alternative 4: Cruxのミドルウェアで、`Effect`をRust側で実行する

`crux_core::middleware`を使う。コアがミドルウェアの条件を満たすことは、コンパイルで確かめた。しかし、Cruxはこの仕組みを0.19.0で`EffectRouter`に置き換え、非推奨にする計画を書いている。新しいシェルを、消える予定の仕組みに載せる理由はないので、選ばなかった。

### Alternative 5: 境界の形式をJSONにする

Cruxの`JsonFfiFormat`を使う。プロパティの`serde_json::Value`を、そのまま通せる。境界を流れるデータを、人が読める。しかし、1000項目のページで、書いて読む時間はbincodeのおよそ8倍だった。`Value`は、型の生成では表せないとみているので、プロパティの型を変える作業は、どちらにしても要る。そのため、選ばなかった。

## Confidence Level

方向についての確信度は中程度である。細部についての確信度は低い。

確かめたことは3つある。`ViewModel`の全体をbincodeで通す費用は、今ある最大のページで2ms弱だった。コアは、変更なしで`EffectRouter`の上で動いた。`ViewModel`の`Serialize`は、小さな変更で通った。どれも、捨てるブランチでの試しである。

確かめていないことが5つある。第一に、Swiftの型の生成である。`Event`の変種は151個あり、`ViewModel`はコアの内部の型を多く含む。生成が最後まで通るかは、動かしていない。第二に、SwiftUIが長いページを描き直す時間である。`ViewModel`の全体を受けると、SwiftUIは1000行の新しい値から差分を求める。入力のたびにこれが起きる。第三に、SSEのように、1つの要求に何度も答える流れである。第四に、RustのライブラリをXcodeのアプリに組み込む手順である。第五に、C#の側である。WinUI3では、何も試していない。

Cruxのこの部分は、まだ動いている。`Effect`をRust側で実行する仕組みは、1年で2回作り直された。`EffectRouter`のRFCは、未解決の問いを5つ残している。`crux_core`は`=0.20.0`に固定しているが、更新のたびに、ネイティブのシェルのcrateを書き直すおそれがある。

計測にも限界がある。項目のASTは、段落1つの形で近似した。実際のASTは、もっと大きいかもしれない。Swiftで読む時間は、手で書いた読み取りで測った。生成されたコードは、これより遅いかもしれない。

次のいずれかが起きたら、この決定を見直す。

- Swiftの型の生成が、今の`Event`と`ViewModel`で通らず、型を直す作業が大きいと分かった。
- SwiftUIで、長いページの入力が遅く、直せなかった。Alternative 2の、差分だけを送る形を検討する。
- Cruxが`EffectRouter`を大きく変えたか、やめた。
- WinUI3で、この方式が使えないと分かった。
- iOSやiPadOSを対象にすると決めた。この方式はそのまま使えるとみているが、確かめる。
- WinUI3とSwiftUIの編集シェルを作らないと決めた。

## Consequences

**良い結果**

Rustの型を変えると、SwiftとC#のシェルがコンパイルで落ちる。境界の型のずれに、動かす前に気づける。

APIの呼び出し、SSE、タイマーは、Rustで1回だけ書く。SwiftUIとWinUI3のシェルは、画面と入力だけを受け持つ。

コアは、`Effect`を誰が実行するかを知らないままである。`harness.rs`のテストは、そのまま通る。`shell-leptos`と`shell-emacs`は変わらない。

プロパティの値が型になる。文字列か、文字列のリストかを、コンパイラが確かめる。

**悪い結果**

`Event`と`ViewModel`の形が、シェルとの約束になる。フィールドを1つ変えると、SwiftとC#の生成をやり直し、両方のシェルを直すことになる。

`Event`の151個の変種は、すべてSwiftとC#の型として見える。サーバーの答えを表す内部の変種も、シェルから送れてしまう。ADR-011が挙げた短所が、ネイティブのシェルにも及ぶ。

`ViewModel`の全体を、描画のたびに境界へ通す。費用は小さいと測ったが、ページが大きくなるほど増える。

Rust側に、スレッドとHTTPクライアントを持つcrateが1つ増える。この部分は、`shell-emacs`の`runtime.rs`と同じく、単体テストでは確かめにくい。

Cruxの新しいAPIに頼る。`crux_core`を更新するたびに、ネイティブのシェルのcrateを直すおそれがある。

プロパティの型を変える作業は、コアの全体に及ぶ。`Item`、`Page`、`Event`、Undoの記録が、今は`serde_json::Value`を持っている。

**中立的な結果**

ビルドの手順が増える。型の生成と、RustのライブラリをXcodeとVisual Studioのプロジェクトへ組み込む手順である。

`shell-emacs`は、狭いメッセージの形のまま残る。シェルがコアを呼ぶ形は、直接呼ぶ形、狭いメッセージの形、シリアライズの境界の形の3つになる。

RustのライブラリをSwiftから呼ぶための道具と、WinUI3のシェルの詳細は、Walking Skeletonで決める。必要なら、別のADRに書く。

ADR-011が残した2つの決定（シェルがコアを呼ぶ方式と、`ViewModel`のシリアライズ）は、このADRで決まる。
