# ADR 076: List Diffs and Panels for the Native Shells

## Status

Accepted

## Context

ADR 068は、進め方の1として、リストの差分とパネルをRustの側で1回だけ作ると決めた。
ネイティブのシェルは、受け取った差分を当てるだけにする。
ADR 068は、そのときにADR 024の「`ViewModel`を丸ごと通す」決定とADR 063が置き換わり、新しいADRで記録するとした。
このADRがその記録である。
作業のissueは#858で、Rustの側は#823、計測は#824である。

### 今の形

ネイティブのシェルは、`client-native-bridge`の`NativeShell::view()`で、`ViewModel`の全体をbincodeで受け取る。
WinUIのシェルは、`StickyWindow.Sync`で、`ViewModel`の`outline.rows`と画面の`ObservableCollection`を、項目のIDで突き合わせる。
SwiftUIとComposeのシェルは、毎回新しい配列を受け取り、行のIDをキーにしてフレームワークに比べさせる。
Web版は、`RowStore`（`client/apps/web-app/outliner/src/components/outline.rs`）が、項目ごとのシグナルで、中身が違う行だけを更新する。
同じ突き合わせの判断が、シェルごとに別々に書かれている。

ADR 063の`view_if_changed()`は、今のコードにはない。

### 前の計測

#824で、`stickies-win`の`PetaScreens.Tests/TypingSpeedTests.cs`を使い、1000項目のページで1つのキーにかかる時間を測った（2026-11-22、AMD Ryzen 7 7700）。
エディタの変更から行の突き合わせまでの中央値は8.73msだった。
そのうち、Rustが`ViewModel`の全体をbytesで書き出すのに4.03ms、C#がそれを読むのに1.92msかかった。
`Draw`と`StickyWindow.Sync`は、残りの約2.5msだった。
1つのキーで変わるのは1行でも、時間の大部分は全部の行を書いて読むことに使われている。

### 決めること

ADR 068は、次の点を「このADRで決めないこと」に残した。
進め方の1を始めるには、このうち差分の作り方を決める必要がある。

- 毎回`view`を作って前回と比べるか、`Model`の中のリストを監視できる型にするか。

ほかに、次の点も決める必要がある。

- どのIDつきのリストを差分にするか。
- 差分の操作の種類と、位置の数え方。
- パネルの単位。
- シェルが差分を受け取る呼び出しと、今の`view()`との関係。

## Decision

`client-native-bridge`の`NativeShell`が、前にシェルへ渡した`ViewModel`と新しい`ViewModel`を比べて、リストの差分と変わったパネルを作る。
ネイティブのシェルは、`ViewModel`の全体を読む代わりに、新しい呼び出し`changes()`で差分を受け取り、それを当てるだけにする。
この決定は、ネイティブのシェル（WinUI、SwiftUI、Compose）について、ADR 024の「`ViewModel`を丸ごと通す」決定とADR 063を置き換える。

### 差分の作り方

`NativeShell`は、前にシェルへ渡した`ViewModel`を1つ持つ。
`changes()`が呼ばれると、`NativeShell`はコアの今の`ViewModel`を作り、持っている`ViewModel`と比べる。
比べた結果を返したあと、持っている`ViewModel`を今のものに替える。
`client-core`の`Model`と`view::build`は変えない。
差分の部品は、`ViewModel`を比べるだけで、`Model`に何も求めないためである。

### リスト

差分にするIDつきのリストは、`ViewModel`の`outline.rows`だけにする。
リストの項目のIDは、`Row`の`item.id`である。
シェルが自分で行を突き合わせているのは、アウトラインの行だけのためである。
サイドバーのページ、検索の結果、バックリンク、アトラスのカードは、当面はパネルの中で丸ごと送る。
シェルが行ごとの扱いを必要としたときに、リストに移す。

### 操作

リストの差分は、次の5種類の操作の列である。

- 挿入：位置と、新しい`Row`。
- 削除：位置。
- 移動：元の位置と、新しい位置。
- 更新：位置と、中身が変わった`Row`。
- 入れ替え：新しい`Row`の全部。

シェルは、操作を列の順に当てる。
各操作の位置は、その前の操作を当てたあとのリストで数える。
WinUIの`ObservableCollection`の`Insert`、`RemoveAt`、`Move`、インデクサー、`Clear`に、そのまま当てられるためである。
SwiftUIとComposeのシェルも、同じ順で配列に当てる。

`NativeShell`は、今の`StickyWindow.Sync`と同じ手順で操作を作る。
操作を作るときは、次の3つのリストを使う。

- リストA：前の`changes()`でシェルに渡した`outline.rows`。
- リストB：今の`ViewModel`の`outline.rows`。
- リストC：リストAに、ここまでに出した操作を当てたもの。
  最初はリストAと同じで、手順を終えるとリストBと同じになる。

1. リストAの行が、リストBに1つも残らないときは、入れ替えを1つだけ出す。
   別のページを開いたときである。
2. リストBにない行を、リストCから、後ろの位置から順に削除する。
3. リストBの位置0から順に、リストCの同じ位置の行を決める。
   リストBの位置`i`を決めるとき、リストCの位置0から`i-1`までは、リストBの位置0から`i-1`までと同じ行になっている。
   1つのリストに同じIDは1つしかないので、リストBの位置`i`の行は、リストCの位置`i`か、それより後ろにしかない。
   リストCの位置`i`に同じIDの行があり、中身が違うときは、更新を出す。
   リストCの位置`i`より後ろにあるときは、位置`i`への移動を出し、中身が違えば続けて更新を出す。
   リストCのどこにもないときは、位置`i`への挿入を出す。

中身が同じ行には、操作を出さない。
中身が同じかどうかは、`Row`の`PartialEq`で決める。

### パネル

パネルは、`ViewModel`の最上位の欄ごとに1つとする。
`screen`、`atlas_cards`、`atlas_callouts`、`atlas_links`、`atlas_link_suggestions`、`sidebar`、`search`、`config`、`connection`、`today`、`calendar`、`title`、`history`が、それぞれ1つのパネルである。
`outline`は、`rows`を除いた残り（`selection_mode`、`focus_request`、`focus_order`、`focused_item_id`、`key_relay_holding`、`wiki`、`scrolled_from_top`）を、1つのパネルにする。
`changes()`は、前と値が違うパネルだけを丸ごと返す。
シェルは、受け取ったパネルの部分だけを描き直せる。

### 呼び出し

`NativeShell`に、`changes()`を足す。
`client-shell-http`と`client-shell-embedded`のFFIにも足す。
`changes()`は、リストの差分と、変わったパネルを1つのbincodeの値で返す。
最初の`changes()`は、入れ替えと全部のパネルを返す。
何も変わっていなければ、空の差分を返す。

シェルは、`changes()`をメインスレッドだけから呼ぶ。
`NativeShell`が持つ「前に渡した`ViewModel`」は、シェルが画面に当てたものと同じでなければならないためである。

### `view()`

`changes()`に移したシェルは、受け取ったパネルと行を自分で持つ。
`Shell`が描くとき以外に読む値（`title`、`history`、`config`、`outline`の`focused_item_id`など）も、そこから読む。
移したシェルは、`view()`を呼ばない。
1つのシェルの中で`view()`と`changes()`を混ぜると、画面と「前に渡した`ViewModel`」がずれるためである。

3つのシェルを移し終えたら、`client-shell-http`と`client-shell-embedded`のFFIから`view()`を消す。
`NativeShell::view()`は、`client-native-bridge`の`test-support`のfeatureの中に入れる。
`client-shell-http`と`client-shell-embedded`は、`[dev-dependencies]`でだけこのfeatureを有効にする。
Cargoのresolver 2では、dev-dependenciesのfeatureはアプリのビルドに入らないため、FFIのライブラリには`view()`が入らない。
各シェルの`Border`のテスト（`PetaBorder.Tests`、`BorderTests.swift`など）は、`view()`ではなく、シェルが`changes()`から組み立てた状態を読んで確かめる。
これにより、シェルが差分を当てる処理も、テストで通る。

移している途中は、まだ移していないシェルが、アプリの本体で`view()`を使う。
そのため、`view()`を`test-support`のfeatureに入れるのは、3つのシェルを移し終えたときとする。

### 型

差分とパネルの型は、ほかの境界の型と同じく、今の生成の仕組みで、Swift、C#、Kotlinの型にする。
C#とKotlinは`codegen.rs`で、Swiftは今の生成器で作る。

### 移し方

3つのシェルを、1つずつ`changes()`に移す。
順は、WinUI、SwiftUI、Composeとする。
WinUIでは、`StickyWindow.Sync`を、差分を当てる処理に置き換える。
移し終えたシェルは、`ViewModel`の行を自分で突き合わせない。
WinUIを移したあと、#824の計測を同じテストでやり直す。

## Alternatives

### Alternative 1: `Model`の中のリストを監視できる型にする

`eyeball-im`のように、`Model`の中のリストを、変更を記録する型にする。
変わった分だけを処理すれば済み、毎回比べる必要がない。
しかし、`client-core`の`Model`が差分の部品の型に依存し、コアの多くの箇所が変わる。
ADR 068は、PetaJournalのリファクタリングを進め方の1で止めるとしている。
そのため、選ばなかった。
共用ライブラリを作る段階で、もう一度比べる。

### Alternative 2: ADR 068の例のIDつきのリストを、最初から全部差分にする

`outline.rows`に加えて、サイドバーのページ、検索の結果、バックリンク、アトラスのカードも差分にする。
境界の形が、最初から全部そろう。
しかし、シェルが行ごとに突き合わせているのはアウトラインの行だけで、ほかのリストを差分にしても消える突き合わせがない。
そのため、選ばなかった。

### Alternative 3: 入れ替えを持たない

ADR 068の「リストの差分」の定義どおり、挿入、削除、移動、更新だけにする。
操作の種類が1つ少ない。
しかし、別のページを開くたびに、全部の行の削除と全部の行の挿入が出る。
今の`StickyWindow.Sync`も、残る行がないときは`Clear`している。
そのため、選ばなかった。

### Alternative 4: FFIにテスト用の`view()`を残す

3つのシェルを移し終えたあとも、FFIに`view()`を残し、シェルのテストだけが使う。
シェルのテストを書き換える量が少ない。
しかし、FFIのライブラリは、アプリとテストで同じもの（`generate.ps1`のreleaseのビルド）である。
そのため、アプリにも`view()`が入り、テスト用だけにはならない。
シェルのテストも、シェルが差分を当てる処理を通らなくなる。
そのため、選ばなかった。

## Confidence Level

差分を`NativeShell`で作り、`Model`を変えないことには、確信がある。
前の計測で、時間の大部分が全体の書き出しと読み取りだったためである。
`StickyWindow.Sync`と同じ手順にすることにも、確信がある。
WinUIで動いている手順を、Rustに移すだけのためである。

次のときは、このADRを見直す。

- WinUIを移したあとの計測で、1つのキーの時間がほとんど減らなかったとき。
  毎回`ViewModel`を作って比べる費用が大きいと分かれば、Alternative 1を考え直す。
- SwiftUIやComposeのシェルで、操作を順に当てる形が合わないと分かったとき。
- アウトラインの行のほかに、行ごとに扱いたいリストがシェルに出てきたとき。

## Consequences

### 良い結果

- 行の突き合わせの判断が、`NativeShell`の1か所に集まる。
  WinUIの`StickyWindow.Sync`はなくなる。
- 1つのキーで境界を越えるデータが、変わった行と変わったパネルの分だけになる。
- 差分の手順を、Rustのテストで、シェルなしに確かめられる。
- 3つのシェルを移し終えると、FFIから`ViewModel`の全体を読む呼び出しがなくなる。
  シェルが`view()`と`changes()`を混ぜる誤りは、FFIの形で防がれる。

### 悪い結果

- `NativeShell`が、シェルごとに前の`ViewModel`を1つ持つ。
  1000項目のページでは、その分のメモリを使う。
- `changes()`を呼ぶたびに、`NativeShell`は`ViewModel`の全体を作って比べる。
  境界を越える量は減るが、Rustの側の作る費用は残る。
- シェルが`changes()`をメインスレッド以外から呼ぶと、画面がずれる。
  型では防げないので、シェルの側の決まりとして守る。
  移している途中は、`view()`と混ぜる誤りも、同じく決まりとして守る。
- シェルの`Border`のテストを、`changes()`から組み立てた状態を読む形に書き換える。

### 中立的な結果

- Emacsのシェルは`NativeShell`を使わず、Cruxの`Core`を直接使う。
  Emacsのシェルと、Web版の`RowStore`は、今のままである。
  差分を受け取る形にするかは、ADR 068の進め方の2で、この作業の計測を見て決める。
- サイドバーや検索の結果をリストに移すときは、このADRの「リスト」を書き換える新しいADRで記録する。
