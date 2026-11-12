# ADR 034: The iOS Shell with the Core and the Sync of the Android Shell

## Status

Proposed

## Context

Androidのシェル（`frontend-android`）が、ほぼできた。Dropboxからの取り込み、検索、装飾つきの表示、編集、アップロードがある（ADR 029、ADR 030、ADR 031、ADR 033）。同じものを、iPhoneでも使いたい。

iOSにも、ジャーナルサーバーはない。Androidと同じく、journal-coreをアプリに組み込む必要がある。Androidのために作ったクレート（`frontend-rust/crates/shell-android`）は、Cruxのコアとjournal-coreを1つにまとめ、BoltFFIで外に出している。中身にAndroidだけのものはない。BoltFFIは、同じクレートをApple向け（xcframework）にも固められる。

macOSには、SwiftUIのシェル（`frontend-swiftui`）がある。これは、ジャーナルサーバーに尋ねるコア（`shell-native`）を使う。境界の越え方（`Border`と`Shell`）と、テストの書き方は、iOSでも使える。

決めることは4つある。コアをどう持つか。同期をどう書くか。項目の文字をどう描くか。サインインをどうするか。

## Decision

iOSのシェルを、`frontend-ios`にSwiftとSwiftUIで作る。Androidのシェルと同じことをする。

コアは、`shell-android`のクレートをそのまま使う。`boltffi.toml`にAppleの出力先を足し、`boltffi pack apple`で、実機、シミュレーター、Macの3つのライブラリを1つのxcframeworkにする。Macのライブラリは、`swift test`のためにある。クレートの名前は、今は変えない。

同期は、Androidのクラスを、同じ名前と同じ分け方でSwiftに書き直す（`Cloud`、`CloudRecord`、`MarkdownFolder`、`Downloader`、`Uploader`、`SyncRound`、`CloudSync`）。決まりは、ADR 030とADR 031のままである。Dropboxとのやり取りは、公式のSDK（SwiftyDropbox）で行う。同期の本体（`PetaSync`）は、コアもDropboxも知らない。

画面を持たない部分は、Swift Packageにして、Macの上で`swift test`でテストする。同期（`PetaSync`）、境界と表示の中身（`PetaBorder`）、Dropbox（`PetaDropbox`）である。アプリ（`App`）は、XcodeGenで作るプロジェクトでビルドし、シミュレーターの上でXCUITestでテストする。

項目の文字は、RustがASTから作るHTMLを、Swiftで読む（`ShownParagraphs`）。HTMLを、見た目を持つ文字のかたまりの段落にして、SwiftUIの`Text`で描く。コードブロック、表、画像は、Androidと同じくASTから読み、専用の部品で描く。

編集する項目の入力欄は、UIKitの`UITextView`にする。Enterが押された位置と、キーボードが変換の途中かどうかが、分かるためである。

サインインは、SDKの決めた流れに任せる。端末にDropboxのアプリがあればそれで、なければアプリの中のブラウザーで行う。トークンは、SDKがキーチェーンに置く。

App keyとDropboxのフォルダーは、リポジトリの外から渡す（`Dropbox.xcconfig`）。キーのないビルドは、同期をしない。

## Alternatives

### Alternative 1: iOS用のクレートを別に作る

`shell-ios`を作り、`shell-android`と同じ中身を持たせる。名前は分かりやすい。短所は、同じコードが2つになることである。中身にOSごとの違いはない。選ばなかった。

### Alternative 2: 同期をRustに書き、両方のOSで使う

同期の決まりを1か所に置ける。KotlinとSwiftに、同じクラスを2回書かずに済む。短所は、Androidの同期がもう動いていて、テストもあることである。Rustに移すと、Dropboxとのやり取りもRustで書き直すか、境界を越えて呼ぶことになる。iOSを足すための変更として、大きすぎる。今回は選ばなかった。3つ目のOSが出たら、見直す。

### Alternative 3: HTMLを`NSAttributedString`に読ませる

iOSには、HTMLから属性つきの文字を作る機能がある。自分でHTMLを読まずに済む。短所は、これがWebKitを使い、メインスレッドでしか動かず、遅いことである。項目の数だけ呼ぶと、スクロールが止まる。SwiftUIの描画の途中で呼ぶと、落ちることもある。コアが書くタグは少ないので、自分で読むほうが確かである。選ばなかった。

### Alternative 4: 入力欄をSwiftUIの`TextField`にする

コードは短くなる。短所は、Enterの位置と、変換の途中かどうかが、取れないことである。Enterで項目を分けることと、日本語の入力の確定を分けることが、できない。選ばなかった。

### Alternative 5: サインインを、いつもブラウザーにする（ADR 033と同じ）

Androidでは、Dropboxのアプリでのサインインが失敗したので、いつもブラウザーにした。iOSでも同じにすれば、流れがそろう。短所は、iOSのSDKにその道がないことである。SDKは、Dropboxのアプリを尋ねる許可（`LSApplicationQueriesSchemes`）がないアプリでは、サインインを始めない。SDKのサインインを捨てて、自分でOAuthを書くことになる（ADR 033のAlternative 3と同じ短所）。iOSで同じ失敗が起きるかも、まだ分からない。選ばなかった。

## Confidence Level

確信度は中程度である。

コアと同期には、確信がある。コアは、Androidと同じクレートである。Swiftからコアを呼ぶテストが、Macの上で通っている。同期のテストは、Androidのテストを書き写したもので、同じ決まりを確かめている。

確信が下がる理由は3つある。

- 本物のDropboxとの同期と、サインインを、まだ実機でもシミュレーターでも確かめていない。App keyがないと動かせない。
- アプリを、実機で動かしていない。シミュレーターだけで確かめた。日本語の入力、後ろに回ったときの同期、大きなフォルダーの最初の同期は、実機で見る必要がある。
- HTMLを自分で読む部分は、コアが書くタグだけを知っている。HTMLを作るクレート（`flatmarkdown-ast2html`）が新しいタグを書くようになると、その見た目は出ない。文字は残る。

次のどれかが起きたら、この決定を見直す。

- Dropboxのアプリでのサインインが、iOSでも失敗する。そのときは、OAuthを自分で書くかを考える。
- 後ろに回ったときの同期が、30秒ほどの持ち時間で終わらないことが多い。そのときは、バックグラウンドのURLセッションを考える。
- KotlinとSwiftの同期が、食い違い始める。そのときは、同期をRustに移す（Alternative 2）。

## Consequences

良い結果は、次のとおりである。

- iPhoneで、Androidと同じことができる。
- コアは1つのクレートのままである。Rustの変更は、両方のOSに同じように届く。
- 画面を持たない部分は、シミュレーターなしでテストできる。速く、Macだけで回る。
- 文字の見た目を決めるコードが、Macでテストできる場所にある。

悪い結果は、次のとおりである。

- 同期のコードが、KotlinとSwiftの2つになる。決まりを変えるときは、両方を直す。
- クレートの名前（`shell-android`）が、中身と合わなくなる。名前を変えるのは、別の変更にする。
- サインインの流れが、AndroidとiOSで違う。Androidはいつもブラウザーで、iOSはDropboxのアプリがあればそれを使う。
- HTMLを読むコードを、自分で持つ。
- アプリは、arm64だけである。IntelのMacのシミュレーターでは動かない。

中立的な結果は、次のとおりである。

- `frontend-ios`のテストは、CIでは回らない。macOSのCIは、費用のために先送りにしている。
- App keyは、Androidと同じDropboxのアプリのものを使える。
- 拡張関数は、Androidと同じく入らない（ADR 031）。スマホで作るページには、テンプレートの項目が入らない。
