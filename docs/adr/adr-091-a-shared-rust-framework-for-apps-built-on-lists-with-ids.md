# ADR 091: A Shared Rust Framework for Apps Built on Lists with IDs

## Status

Accepted

このADRは、ADR 068を置き換える。

## Context

### 背景

PetaJournalのほかにも、Rustのコアとネイティブの画面で作りたいアプリがある。

- Rustを採用するのは、起動速度と実行速度のためである。
- ネイティブの画面を採用するのは、日本語入力の問題の少なさと、OSのイベントの取り扱いの容易さのためである。

アプリの例：

- 画像ビューアーの`sosuisen/sss-image-viewer`移植
- 画像ファイルエクスプローラーの`sosuisen/gallery-explorer`移植
- その他のJavaFXで作成してきたアプリ

いずれも日常的に使っているアプリである。

どのアプリも、Windows、macOS、Linuxで動作する形にしてみたい。
対象のプラットフォームにはWinUI、AppKit、SwiftUIのようなネイティブのGUIが含まれる。

PetaJournalを開発する中で、クロスプラットフォーム開発に共通して必要な枠組みが分かってきた。

- 対象のアプリの画面は、IDで管理できる部品の並びが中心である。
  - PetaJournalの場合それは、アウトラインの行、サイドバーのページ、検索の結果である。
  - 画像ビューアーとエクスプローラーでは、フォルダの中の項目、お気に入り、開いているウィンドウである。
- 疎結合部分の検証方法

### 必要性

- 共通コードにできる処理や知識はDRY原則でまとめたい。
- プラットホーム固有の部分は薄いシェルとする。
- 1つの言語で開発しないため、疎結合部分の検証（形式検証、静的解析、テスト）が重要である。
  - Rustコアの提供するAPIとネイティブの画面側で利用しているAPIにずれがないか。
  - コードは共通化できないがよく似た機能を持つネイティブの画面間でのテストやモジュールの名前と構造の共通化。共通化しておくと、実装の抜け落ちが減り、人間側がレビューする際にも楽である。

### ADR 068の決定と、その後

ADR 068（2026-11-20）は、IDつきの部品の並びを中心にしたアプリのために、共用のRustライブラリを作ると決めた。
進め方は、次の6段階だった。

1. PetaJournalの差分の処理を共通にする。
2. EmacsとWebのシェルも差分を受け取る形に変えるかを、1の結果を見て考える。
3. sss-image-viewerを、WinUIとAppKitで作り直すための共用ライブラリを作る。
4. GalleryExplorerも、共用ライブラリで作り直す。
5. 2つのアプリで共通にできたことを確かめてから、PetaJournalへの導入を検討する。
6. sss-image-viewerとGalleryExplorerを合体させたアプリなどは、その次に検討する。

ADR 068は、ライブラリかフレームワークか、Cruxとの関係、差分の作り方などを「このADRで決めないこと」に残した。
その後、1と2が終わり、残した点のいくつかが決まった。
ADR 068のContextの事実も、今のコードと違うところが増えた。
そのため、ADR 068を置き換えるこのADRで、今の状態と残りの決定を記録する。

### 終わったこと：差分の境界

進め方の1と2は、次のADRで終わった。

- ADR 076は、`ViewModel`を前回と比べて、リストの差分と変わったパネルを作ると決めた。
  ネイティブのシェル（WinUI、Compose、macOSとiOSのSwiftUI）は、`changes()`で差分を受け取り、当てるだけになった。
  ADR 024の「`ViewModel`を丸ごと通す」決定とADR 063は、ネイティブのシェルについて置き換わった。
- ADR 078は、差分を`client-core`に移し、本文の形`B`で一般化した（`Changes<B>`、`Shown<B>`）。
  Emacs版も、facetのJSONで書いたJSON Linesで差分を受け取る形になった。
- ADR 079とADR 080は、Web版のシェルをLeptosからReact（`client/apps/web-app/outliner-react`）に替えた。
  Web版も、`client-web-bridge`から差分を受け取り、当てるだけになった。
  Leptosのシェルは、2026-11-24に消えた。
- ADR 088は、パネルの中にあった並び（見出し、最近のページ、フォルダのページ、検索の結果）も、差分で送ると決めた。

この結果、ADR 068が問題にした「同じ突き合わせの判断が、6つのシェルに散っている」状態はなくなった。
差分は`client-core`の`list_changes`で1回だけ作られる。
6つのシェルは、どれも差分を当てるだけである。

### 決まったこと

ADR 068が残した点のうち、次のものは決まった。

- 差分の作り方は、毎回`view`を作って前回と比べる形に決まった（ADR 076）。
  `Model`は、差分の部品の型に依存しない。
- パネルとリストの分け方は、変わり方で分ける形のまま、並びをすべてリストにする形に進んだ（ADR 088）。
- 型の生成は、facet_generate 0.22を直接呼ぶ形にそろった。
  TypeScriptの型も、facet_generateで作る（ADR 080）。
  JSONの境界は、`serde_json`ではなく、facetのJSONで書く（ADR 078、ADR 080）。

### 変わったこと：境界の型の制限

facet_generateは、ジェネリックな型を、型引数を埋めた1つの型として生成する（ADR 088）。
同じジェネリックな型を2種類の型引数で使うと、生成が止まる。
そのため、ADR 088は、`macro_rules!`で並びごとに名前の違う差分の型（`RowChange<B>`、`HeadingChange`、`PageChange`、`SearchChange`）を作る形にした。
共通の部品を、ジェネリックな型として境界に出すことはできない。
この制限は、複数のアプリで使う部品の形に影響する。

### 変わったこと：シェルを確かめる仕組み

ADR 068のあとで、シェルがコアの決まりを守っているかを確かめる仕組みができた。
置き場所は、PetaJournalの`client/tools/shell-check/`である。
各シェルの決まりは`rules/<シェル>.toml`に、コアの決まりは`rules/core.toml`に書く。

- S（静的）は、シェルのソースを読む（ADR 064）。
  S1は依存の許可リスト、S2は`Event`の参照、S3はOSの部品の編集コマンド、S4は関数の分岐の数である。
- D（動的）は、コアが作った例を、各シェルのテストで回す（ADR 089、ADR 090）。
  D1は差分を当てた結果、D2は境界の値の読み書きである。
  例は`common/`の`list-changes.json`、`border-web.json`、`border-native.json`にある。

今、実装できているのはS2、D1、D2である。
D1とD2は、Web、iOS、macOSで回る。
Windows、Android、Emacsは、issueを立てて進めている（`docs/verification.html`）。
S1、S3、S4は、まだ試す前である。

この仕組みは、`rules/`と例のJSONが、PetaJournalの型に合わせて書かれている。
仕組みそのものは、IDつきのリストとパネルの境界を持つアプリなら、どれでも使える。

### 変わっていないこと：Crux

`client-core`は、今もcrux_coreを`=0.20.0`で固定して使っている。
crux_coreに依存するcrateは、`client-core`、`client-native-bridge`、`client-web-bridge`、`client-shell-embedded`、`client-shell-http`、`outliner-emacs`の6つである。

Cruxから実際に得ている価値は、ほぼ`Command`の非同期のエフェクトの実行である（ADR 068）。
サーバーの答えを`send_event`で`Event`に戻すため、`SplitDone`などの内部の変種がシェルの型に出る問題（#767）も残っている。
Cruxは、差分の境界も、シェルを確かめる仕組みも持たない。
今のPetaJournalでは、この2つをCruxの外に自分で作り、`client-core`と`client/tools/shell-check/`に置いている。

自前のコアについては、コアを静的に縛る10の決まりを考えた（`docs/learn-rust-for-adr068.md`）。
たとえば、「エフェクトの答えの型は、エフェクトごとに決まる」「処理は、答えを待っている間`Model`を持てない」「コアは、I/O、時計、スレッドを直接使わない」である。
決まりを破ると、コンパイルエラーやclippyのエラーになる形を目指す。
開発者は、決まりの元になるRustの仕組みを1つずつ学んでいる途中である。
今は1番の決まりの説明中である。

### 変わっていないこと：ほかのアプリ

sss-image-viewerとGalleryExplorerには、まだ手を付けていない。
ADR 068に書いた次の状態は、今も同じである。

- GalleryExplorerは、Rustと`windows-reactor`（宣言的なWinUIのライブラリ、プレビュー版）で書いている。
  0.100.0には、キーのイベント、項目の仮想化、`ScrollViewer`のスクロール、`MenuFlyout`がない。
  そのため、Win32のフック、`StackPanel`、Tabキーを送る方法、Win32のメニューで補っている（GalleryExplorerのADR 001、ADR 002）。
  画面に関わる判断は、すでに`explorer`と`domain`に分けてあり、テストできる。
- sss-image-viewerは、JavaFXで書いている。
  起動を速くするために、AOTとCDSを使っている。
  WindowsAPIで書かれたランチャーはWindowsだけを対象にしているが、Javaだけを使うならmacOS、Linuxで動作する。

### 類似事例

| 事例 | 何を差分にするか | 差分を作る場所 | シェル |
|---|---|---|---|
| matrix-rust-sdk（`matrix-sdk-ui`、`eyeball-im`） | データのリスト（`VectorDiff`） | 同じプロセスのRust | Element XのSwiftUIとJetpack Compose |
| Hotwire（Turbo Streams） | IDを狙った`append`、`replace`、`remove` | サーバー | ブラウザ、Hotwire Native |
| Phoenix LiveView、LiveView Native | 画面のテンプレート | サーバー | ブラウザ、SwiftUI、Jetpack Compose |
| Redwood（Cash App） | 画面の部品の木 | 同じプロセスのKotlin | SwiftUI、UIKit、AndroidのView、DOM |
| Dioxus | 仮想DOM（Mutationの一覧） | 同じプロセスのRust | Web、デスクトップ、ターミナルなど |
| React Native（Fabric） | 画面の部品の木 | 同じプロセスのJavaScript | iOSとAndroidのネイティブの部品 |
| Crux | 差分は作らない。`ViewModel`を丸ごと渡す | なし | どれでもよい |

データのリストを差分にする形（matrix-rust-sdk、Turbo Streams）では、どう描くかはシェルが決める。
PetaJournalには、付箋紙、アウトライナー、Emacsのように、見た目がまったく違うシェルがある。
そのため、データのリストを差分にする形が合う。
この形の考え方（コアはMVU、シェルはSignals、リストの差分はその通訳）は、ADR 068の「この考え方に意味はあるか」に書いた。

どの事例も、シェルが決まりを守っているかを確かめる仕組みは持たない。

## Decision

IDつきの部品の並びを中心にしたアプリのために、クロスプラットフォームの共用のRustのフレームワークを作る。
フレームワークの名前は、praesepeを予定する。
praesepeは、Cruxを使わない自前のコアである。

ただし、いきなり作らない。
ADR 068の進め方の3から、順に1つずつ進める。

### 言葉の定義

- **IDつきのリスト**：画面の部分のうち、長さに上限がなく、各項目がIDを持つもの。
  例は、PetaJournalの`Changes`の`rows`、`headings`、`recent_pages`、`index_pages`、`search_results`である。
- **リストの差分**：IDつきのリストに対する、挿入、削除、移動、更新、入れ替えの操作の列（ADR 076の`ListChange`）。
- **パネル**：画面の部分のうち、形が決まっていて小さいもの。
  変わったときに丸ごと送る。
  例は、PetaJournalの`Panel::Calendar`、`Panel::Config`、`Panel::Connection`である。
- **決まり**：コアやシェルが守るべき約束のうち、破るとエラーになる形で確かめるもの。
  コアの決まりは、`docs/learn-rust-for-adr068.md`の10の決まりである。
  シェルの決まりは、S1〜S4とD1〜D2である（ADR 064、ADR 089、ADR 090）。

### フレームワークにする

praesepeは、ライブラリではなくフレームワークにする。
フレームワークでは、フレームワークがアプリのコードを呼ぶ。
アプリは、praesepeの決めた形で`Model`の更新と`view`を書き、praesepeがそれを呼ぶ。

フレームワークにするのは、決まりを守らせるためである。
ライブラリでは、アプリが部品を使わない道を選べる。
すると、コアの10の決まりも、S1〜S4とD1〜D2も、アプリごとに守るかどうかが変わってしまう。
フレームワークがループと境界を持てば、決まりは、praesepeを使うすべてのアプリに同じように効く。

### フレームワークが持つもの

praesepeは、次のものを持つ。

- **コア**：エフェクトの実行器、要求と答えの対応、ルーター。
  Cruxの`Command`と`Core`に代わる。
  PetaJournalの`harness.rs`のように、コアの振る舞いをサーバーなしで決定的に試せる形を保つ（ADR 058）。
- **差分の境界**：`list_changes`、リストの差分、パネル、前回の`view`を持って比べる部品。
  PetaJournalの`client-core`の`changes`モジュールが元になる。
- **境界の型の生成**：facet_generateで、Swift、C#、Kotlin、TypeScriptの型を書く仕組み。
- **コアの決まり**：`docs/learn-rust-for-adr068.md`の10の決まり。
  破るとコンパイルエラー、clippyのエラー、依存の検査のエラーになる形にする。
- **シェルの決まり**：S1〜S4とD1〜D2。
  praesepeは、チェックの仕組みと、各アプリの型から例のJSONを作る仕組みを持つ。
  各アプリは、自分の`rules/`と、自分の型から作った例を持つ。

AIコーディングの視点から見ると、harness-includedであるといえる。

フレームワークには、今PetaJournalで実装できている決まりだけでなく、まだ実装していない決まりもすべて含める。
まだ実装していないのは、コアの10の決まりと、S1、S3、S4である。
決まりの1つずつの形は、実装するときに、それぞれのADRで決める。

### 進め方

ADR 068の進め方の3以降を、そのまま続ける。

3. **sss-image-viewerを、WinUIとAppKitで作り直すために、praesepeを作る。**
   このとき、GalleryExplorerとPetaJournalでも使えることを念頭に置く。
   差分の境界とシェルの決まりは、PetaJournalで動いているものを元にする。
4. **GalleryExplorerも、praesepeで作り直す。**
5. **2つのアプリで共通にできたことを確かめてから、PetaJournalへの導入を検討する。**
   導入するときは、Cruxをpraesepeのコアに置き換える。
   PetaJournalの`client-core`の差分の部品と、`client/tools/shell-check/`の仕組みも、praesepeのものに置き換える。
   PetaJournalに残るのは、PetaJournalの型、`rules/`、例のJSONである。
6. **sss-image-viewerとGalleryExplorerを合体させたアプリなどは、その次に検討する。**

praesepeをPetaJournalではなく画像ビューアーで先に作るのは、PetaJournalのコードが大きく、作り途中のフレームワークを入れる危険が大きいためである。
使うアプリが2つそろうまで、共通の形は見えない。
それまでの間、PetaJournalの差分の境界とshell-checkは、PetaJournalの中で育て続ける。
そこで分かったことを、praesepeに持ち込む。

### このADRで決めないこと（要検討）

実装の際に改めて比較検討したほうがよい。

**コアの形**

- 非同期の処理が`Model`を変える方法。
  Cruxのように答えを`Event`にして送り返すか、「`Model`をこう変える」関数をループに送るかを選ぶ。
  前者は#767の原因である。
  10の決まりの4番と5番は、後者を前提にしている。
  決まりの学習が終わってから決める。
- 1回分の処理がいつ終わったかを、ループがどう判断するか。
  差分は、1回分の処理にまとめて送る。
- どのスレッドでコアを動かすか。
  サーバーの答えやタイマーは別のスレッドから戻るので、メインスレッドへ渡す決まりが要る。

**境界の型**

- facet_generateがジェネリックな型を1種類の型引数でしか生成できない制限を、フレームワークでどう扱うか。
  PetaJournalでは`macro_rules!`で並びごとに型を作った（ADR 088）。
  praesepeでも、アプリが自分の並びの型をマクロで作る形にするかを決める。

**シェルの決まり**

- S1〜S4とD1〜D2のチェックの仕組みを、アプリの`rules/`とどう分けるか。
  今の`rules/core.toml`の中身のうち、どこまでがpraesepeのもので、どこからがアプリのものかを決める。
- 新しいシェルの言語（たとえばLinuxのGTK4）を足すとき、チェックの仕組みをどう広げるか。

**シェルの技術**

- WindowsのシェルをC#のWinUIにするか、`windows-reactor`を続けるか。
  GalleryExplorerは`windows-reactor`の制約（キー、仮想化、スクロール、メニュー）に何度も当たっている。
  `windows-reactor`はプレビュー版であるが、開発自体は活発である。
  状況をみて決める。
  C#の問題は起動速度であるが、AOTで常に改善されつつはある。
- macOSで、AppKitとSwiftUIをどう使い分けるか。
  画像ビューアーの枠なしや全画面のウィンドウは、AppKitのほうが扱いやすい。
- LinuxをGTK4（gtk4-rs）にするか。
  Rustから`Core`を直接呼べるので、境界の形式が要らない。

**画像のアプリでの切り分け**

- 画像の画素は境界を越えさせず、コアはパスだけを渡す。
  表示用のデコードと描画は、WIC、ImageIO、GdkPixbufのようなOSの仕組みに任せる。
- ズーム、パン、ウィンドウのドラッグをシェルが持ち、残す値だけを操作の終わりにコアへ伝えるか。
- Rustで速くするのを、フォルダの読み取りと並べ替え、EXIFの読み取り、サムネイルの生成とキャッシュ、前後の画像の先読みの判断に絞るか。

**その他**

- praesepeを置くリポジトリ。

## Alternatives

### Alternative 1: ライブラリにする

ADR 068の議論では、部品はライブラリにし、組み立て済みの「標準のループ」を使うかどうかは選べる形にする案に傾いた。
アプリごとに形を変えやすい。
しかし、アプリが部品を使わない道を選べるので、決まりがすべてのアプリに効くとは限らない。
S1〜S4とD1〜D2のような仕組みも、アプリごとに入れるかどうかが変わる。
そのため、選ばなかった。

### Alternative 2: Cruxの上にフレームワークを作る

Cruxの`Command`と`Core`を使い、その上に差分の境界とシェルの決まりを足す。
エフェクトの実行器を自分で書かずに済む。
しかし、答えを`Event`に戻す形のままなので、#767の内部のイベントが残る。
コアの10の決まりのうち、`Model`の変更をデータとして送る決まりは、Cruxの形では守らせられない。
crux_coreの版を上げるたびに、APIの変更と型の生成の回避策を確かめ直すことにもなる。
そのため、選ばなかった。

### Alternative 3: PetaJournalの中だけで続ける

フレームワークを作らず、PetaJournalの差分の境界とshell-checkを、PetaJournalの中だけで育てる。
作業は最も少ない。
しかし、sss-image-viewerとGalleryExplorerでは、同じ仕組みと同じ決まりをもう一度作ることになる。
そのため、選ばなかった。

### Alternative 4: praesepeを先に作り、すぐPetaJournalに入れる

PetaJournalを最初の利用者にする。
差分の境界とshell-checkはPetaJournalにあるので、切り出すだけで始められる。
しかし、PetaJournalのコアは大きく、作り途中のフレームワークに合わせて変えると、壊したときの費用が大きい。
使うアプリが1つでは、共通の形も見えない。
そのため、選ばなかった。

### Alternative 5: 既存のフレームワークを使う

Dioxus、Redwood、LiveView Nativeは、画面の部品の木を差分にする。
どの部品を置くかまでコアが決めるので、見た目がまったく違うシェルを持てない。
matrix-rust-sdkの`eyeball-im`は、リストの差分の部品だけで、ループやエフェクトの実行は含まない。
どれも、シェルが決まりを守っているかを確かめる仕組みを持たない。
そのため、選ばなかった。

### Alternative 6: RustだけのGUIで書く

Slint、egui、DioxusのネイティブのレンダラーなどのRustのGUIで、すべてのプラットフォームの画面を書く。
境界がなくなり、言語も1つになる。
しかし、ネイティブのGUIを使いたいという目的に反する。
OSのIME、ネイティブイベントへの対応、アクセシビリティ、見た目に合わせる作業も、自分で負うことになる。
そのため、選ばなかった。

## Confidence Level

進め方の順番には、確信がある。
小さく始めて、2つのアプリで共通の形を確かめてから広げるためである。
差分の境界の形にも、確信がある。
PetaJournalの6つのシェルで動き、D1で同じ結果になることを確かめているためである。
フレームワークにすることにも、確信がある。
決まりをすべてのアプリに効かせるには、ループと境界をフレームワークが持つ必要があるためである。

コアの形には、確信がない。
非同期の処理が`Model`を変える方法と、10の決まりをどこまでコンパイルエラーにできるかは、まだ確かめていない。
S1、S3、S4も、まだ試す前である。

次のときは、このADRを見直す。

- 10の決まりの学習や試作で、決まりの多くがコンパイルエラーにできないと分かったとき。
- sss-image-viewerやGalleryExplorerの画面が、IDつきのリストとパネルに分けられないと分かったとき。
- sss-image-viewerとGalleryExplorerで、フレームワークの形が合わず、アプリごとに形を変える必要が大きいと分かったとき。
- Cruxが、差分を作る仕組みや、内部のイベントを隠す仕組みを持ったとき。
- facet_generateが、ジェネリックな型を複数の型引数で生成できるようになったとき。
- `windows-reactor`が、キーのイベントや仮想化を持ち、プレビューでなくなったとき。

## Consequences

### 良い結果

- コアの10の決まりと、S1〜S4、D1〜D2が、praesepeを使うすべてのアプリに同じように効く。
  アプリごとに決まりを作り直すテストが要らなくなる。
- 差分の判断が、どのアプリでもRustの1か所に集まる。
  シェルは、差分を当てる処理とパネルを差し替える処理だけを持つ。
- PetaJournalで動いている差分の境界とshell-checkを元にするので、praesepeの最初の形を手探りで作らずに済む。
- PetaJournalにpraesepeを入れれば、crux_coreの版の固定と、#767の内部のイベントがなくなる。
- sss-image-viewerは、JavaFXのAOTとCDSの工夫に頼らずに、速く起動できるようになる。
- GalleryExplorerは、`windows-reactor`の制約から抜けられる可能性がある。

### 悪い結果

- フレームワークの形に合わないアプリは、praesepeを使えない。
  アプリごとの自由はライブラリより小さい。
- エフェクトの実行器、要求と答えの対応、ルーターを、自分で書いて保守することになる。
- 5の段階まで、PetaJournalにはCruxが残る。
  差分の境界とshell-checkも、PetaJournalとpraesepeに2つある期間が続く。
  その間に分かったことを、2つの間で手で持ち運ぶことになる。
- 3と4の段階では、PetaJournalの開発の時間が、ほかのアプリに回る。

### 中立的な結果

- ADR 068のStatusは、`Superseded by ADR 091`になる。
  ADR 068の「この考え方に意味はあるか」の節は、このADRの考え方の根拠として、そのまま参照できる。
- `docs/learn-rust-for-adr068.md`の10の決まりは、praesepeのコアの決まりになる。
  ファイルの名前と、その中の「ADR 069」の予定は、このADRに合わせて直す必要がある。
- コアの形と、各決まりの形を決めるときに、このADRの「このADRで決めないこと」が、次のADRのContextになる。
