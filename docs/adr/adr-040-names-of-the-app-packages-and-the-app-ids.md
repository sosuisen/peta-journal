# ADR 040: Names of the App Packages and the App IDs

## Status

Accepted

## Context

ADR 039は、フォルダーだけを並べ替えた。アプリのパッケージの名前と中の名前は、あとで変えると決め、新しい名前を決めずに残した。今は、フォルダーの名前と中の名前が合わないものがある。

- `client/apps/web-app/outliner`のパッケージは、`client-shell-leptos`である。`client/apps/emacs/outliner`のパッケージは、`client-shell-emacs`である。どちらも、道具の名前と、Cruxの言葉の`shell`を持つ。ADR 039は、アプリの名前を、使う人から見た役割で付けると決めた。
- 2つのフォルダーは、どちらも`outliner`である。Cargoは、1つのワークスペースに同じ名前のパッケージを置けない。フォルダーの名前を、そのままパッケージの名前にはできない。
- `client/apps/webview-app/memoview-win`の中は、`stickies`と`stickies-tests`、`Stickies.csproj`、名前空間の`Stickies`である。アプリの名前は`memoview`になったのに、中は前の名前のままである。`stickies`は、今は別のアプリ（`stickies-win`、`stickies-mac`）の名前である。
- npmのパッケージの名前は、`peta-journal-todo`と`e2e`である。Gradleのプロジェクトの名前は、`peta-journal-android`である。どれも、フォルダーの名前と合わない。

アプリのIDも、そろっていない。

- AndroidとiOSのアプリのIDは、`net.sosuisen.petajournal`である。2026-11-12にAndroidのシェルを足したときに入った。選んだ理由の記録は、ADRにもコミットにもない。
- `stickies-mac`の`bundleIdPrefix`は、`petajournal`である。ドメインを逆にした形ではない。
- 公開しているほかのパッケージは、`com.sosuisha`を使う（Mavenの`com.sosuisha:flatmarkdown`、`com.sosuisha:hmtid`）。

AndroidとiOSのアプリは、まだストアに出していない。IDを変える費用は、今がいちばん小さい。

一方で、アプリの部品には、`Peta`で始まる名前がある（`PetaBorder`、`PetaSync`、`PetaDropbox`、`PetaScreens`、生成する`PetaCore`と`PetaTypes`）。`stickies-win`と`stickies-mac`の実行ファイルは、`PetaStickies`である。ADR 037は、Cargoのパッケージの名前から`peta-`を外した。SwiftとC#の名前については、何も決めていない。

## Decision

アプリのパッケージの名前と中の名前を、アプリのフォルダーの名前に合わせる。アプリのIDは、`com.sosuisha`で始める。

名前は、次のように変える。フォルダーは、`client/apps/`の下である。

| フォルダー | 今の名前 | 新しい名前 |
|---|---|---|
| `web-app/outliner` | Cargoのパッケージと実行ファイルの`client-shell-leptos` | `outliner-web` |
| `web-app/outliner/e2e` | npmの`e2e` | `outliner-web-e2e` |
| `emacs/outliner` | Cargoのパッケージと実行ファイルの`client-shell-emacs` | `outliner-emacs` |
| `web-app/todo` | npmの`peta-journal-todo` | `todo` |
| `webview-app/memoview-win` | フォルダーの`stickies`と`stickies-tests` | `memoview`と`memoview-tests` |
| | `Stickies.csproj`、`Stickies.Tests.csproj`、`Stickies.sln` | `MemoView.csproj`、`MemoView.Tests.csproj`、`MemoView.sln` |
| | 名前空間の`Stickies` | `MemoView` |
| `native-app/outliner-android` | Gradleのプロジェクトの`peta-journal-android` | `outliner-android` |
| | アプリのIDとKotlinのパッケージの`net.sosuisen.petajournal` | `com.sosuisha.petajournal` |
| `native-app/outliner-ios` | アプリのIDの`net.sosuisen.petajournal` | `com.sosuisha.petajournal` |
| `native-app/stickies-mac` | `bundleIdPrefix`の`petajournal` | `com.sosuisha` |

決まりは、次のとおりである。

- 名前の順は、見る人で決まる。フォルダーは、開発者が見る。そのため、環境を前に置く（`web-app/outliner`、`emacs/outliner`）。アプリの名前は、使う人が見る。そのため、アプリの名前を前に置き、環境を後ろに置く（`outliner-web`、`outliner-emacs`）。
- アプリのパッケージの名前は、アプリの名前にする。フォルダーの名前が、もう環境を後ろに持つときは、そのまま使う（`outliner-android`、`stickies-win`、`memoview-win`）。持たないときは、フォルダーの1段目の環境を、後ろに付ける（`web-app/outliner`が`outliner-web`）。
- アプリのパッケージの名前に、道具の名前（`leptos`）と、Cruxの言葉（`shell`）を入れない。`client-shell-`で始まる名前は、`client/libs/`のライブラリだけに残る（`client-shell-http`、`client-shell-embedded`）。
- フォルダーは動かさない。動かすのは、`memoview-win`の中の2つのフォルダーだけである。
- アプリのIDは、`com.sosuisha`で始める。AndroidとiOSは、`com.sosuisha.petajournal`にする。生成するKotlinのパッケージも、`com.sosuisha.petajournal.types`と`com.sosuisha.petajournal.core`にする。
- `Peta`で始まる部品の名前は、変えない（`PetaBorder`、`PetaSync`、`PetaDropbox`、`PetaScreens`、`PetaCore`、`PetaTypes`）。部品の名前であり、アプリの名前ではない。`Peta`を外すと、`Border`や`Core`のように広すぎる名前になる。
- 使う人が見る名前は、変えない。`stickies-win`と`stickies-mac`の実行ファイルとプロジェクトの`PetaStickies`、iOSのプロジェクトと表示名の`PetaJournal`、Emacsの`peta-journal.el`とそのコマンドである。
- `memoview-win`の設定のファイルは、`~/.peta-stickies`のままにする。変えると、古いファイルを読む移行が要る。
- `client/libs/`と`journal/`のパッケージは、変えない。フォルダーと同じ名前に、もうなっている（ADR 037、ADR 039）。

改名は、4つのPRに分ける。1つのPRは、1つのビルドの単位を閉じる。

1. `outliner-web`と`outliner-emacs`。Cargoのワークスペース、`build-frontend`、Trunk、Playwrightの設定、Emacsの実行ファイルの名前を直す。`e2e`のnpmの名前も、ここで変える。
2. `memoview-win`の中のフォルダー、プロジェクト、名前空間。
3. アプリのID。Android、iOS、`stickies-mac`、生成するKotlinのパッケージ。Gradleのプロジェクトの名前も、ここで変える。
4. `todo`のnpmの名前。

## Alternatives

### Alternative 1: `client-shell-leptos`と`client-shell-emacs`のままにする

改名が要らない。wasmのファイルの名前と、Emacsの実行ファイルの名前も変わらない。しかし、フォルダーの名前とパッケージの名前が合わないままになる。ADR 039が残した項目が片付かない。そのため選ばなかった。

### Alternative 2: フォルダーも`outliner-web`と`outliner-emacs`にする

フォルダーとパッケージが、同じ名前になる。しかし、パスは`web-app/outliner-web`になり、種類を2回言う。ADR 039で動かしたフォルダーを、もう一度動かすことにもなる。そのため選ばなかった。

### Alternative 3: 環境を前に付ける（`web-outliner`、`emacs-outliner`）

フォルダーの並び（`web-app/outliner`）と、同じ順になる。しかし、パッケージの名前は、アプリの名前である。アプリの名前は、使う人が見るので、環境を後ろに置く。ほかのアプリも、その順である（`outliner-android`、`stickies-win`）。そのため選ばなかった。

### Alternative 4: アプリのIDを`net.sosuisen`のままにする

何も直さなくてよい。実機のアプリも、入れ直さなくてよい。しかし、`net.sosuisen`を選んだ理由の記録がない。公開しているほかのパッケージは、`com.sosuisha`を使う。ストアに出したあとは、IDを変えられない。そのため選ばなかった。

### Alternative 5: アプリのIDを、アプリの名前にする（`com.sosuisha.outliner`）

IDも、フォルダーの名前に合う。しかし、IDは、使う人と外のサービスが見る名前である。使う人が見るアプリの名前は、`PetaJournal`である。`outliner`は、リポジトリの中でアプリを呼び分ける名前である。そのため選ばなかった。

### Alternative 6: `Peta`で始まる部品の名前から、`Peta`を外す

ADR 037がCargoのパッケージから`peta-`を外したことと、そろう。しかし、`Border`、`Core`、`Types`、`Sync`は、SwiftとC#のモジュールの名前として広すぎる。ほかのライブラリの名前とぶつかりやすい。そのため選ばなかった。

### Alternative 7: 実行ファイルの`PetaStickies`を、`Stickies`にする

フォルダーの名前（`stickies-win`、`stickies-mac`）に合う。しかし、macOSには、標準の`Stickies.app`がある。使う人が見る名前を変えると、ショートカットや起動の設定を直すことになる。そのため選ばなかった。

### Alternative 8: 1つのPRで、すべてを改名する

名前が混ざる期間がない。しかし、Cargo、.NET、Gradle、Xcodeの4つのビルドを、1つのPRで直すことになる。CIは2026-11-25まで止まっているので、手元の確認だけが頼りである。失敗したときに、どの改名が原因かを切り分けにくい。そのため選ばなかった。

## Confidence Level

確信度は、名前の決まりについては高く、アプリのIDを変える手間については中くらいである。

名前の決まりは、ADR 039の決まりの続きである。フォルダーは環境を前に置き、アプリの名前は環境を後ろに置く。アプリの名前のこの形は、すでに`outliner-android`、`outliner-ios`、`stickies-win`、`stickies-mac`、`memoview-win`で使っている。改名は動作を変えないので、ビルドとテストで確かめられる。

まだ確かめていないのは、次の4つである。それぞれのPRの中で確かめる。

- wasmのファイルの名前が変わったあとに、`build-frontend`とジャーナルサーバーの配信が動くか。
- Emacsの`peta-journal.el`が、新しい名前の実行ファイルを見つけるか。
- BoltFFIと`codegen`が、新しいKotlinのパッケージで生成し、Androidのアプリがビルドできるか。
- iOSと`stickies-mac`が、新しいIDで署名でき、実機とシミュレーターで動くか。

Dropboxのサインインの戻り先は、App keyで決まる（`db-<App key>`）。アプリのIDには頼らないと見ている。これは、設定のファイルを読んだだけで、実機では確かめていない。AndroidとiOSのPRで、実機のサインインを確かめる。

CIは、2026-11-25まで止まっている。それまでは、手元の確認だけが頼りである。iOSと`stickies-mac`のビルドは、Macで確かめる。

次のどれかが起きたら、この決定を見直す。

- `client/apps/`に、同じ名前のアプリが、同じ種類でもう1つできたとき。環境だけでは、呼び分けられなくなる。
- アプリを、別のリポジトリに切り出すとき、またはパッケージを公開するとき。`todo`や`outliner-web`のような名前は、公開の場では広すぎる。
- AndroidかiOSのアプリを、ストアに出すとき。IDは、そのあと変えられない。出す前に、`com.sosuisha.petajournal`でよいかを、もう一度確かめる。
- 使う人が見るアプリの名前（`PetaJournal`、`PetaStickies`）を変えるとき。IDと実行ファイルの名前も、合わせて決める。

## Consequences

良い結果は、次のとおりである。

- アプリのフォルダーの名前から、パッケージの名前が分かる。探すときに、名前を変換せずに済む。ADR 039が残した項目が片付く。
- `client-shell-`で始まる名前は、`client/libs/`のライブラリだけになる。名前から、ライブラリかアプリかが読める。
- `stickies`は、付箋で編集するアプリ（`stickies-win`、`stickies-mac`）だけを指す。`memoview-win`の中に、前の名前が残らない。
- アプリのIDが、`com.sosuisha`にそろう。公開しているほかのパッケージと、同じ持ち主の名前になる。
- 名前の順の決まりが、文書になる。次にアプリを足すときに、迷わない。

悪い結果は、次のとおりである。

- wasmのファイルの名前と、Emacsの実行ファイルの名前が、また変わる（ADR 037でも変わった）。使う人は、`build-frontend`を回し直し、Emacsの版をビルドし直す。古い`target/`の中身は、手で消す。
- AndroidとiOSの端末は、新しいIDのアプリを、別のアプリとして扱う。古いアプリは手で消し、新しいアプリでDropboxにサインインし直す。アプリのデータはDropboxの写しなので（ADR 030）、失われない。
- `stickies-mac`も、IDが変わる。付箋のファイルは、`Application Support/PetaStickies/stickies.json`にあり、IDに頼らないので、そのまま読める。`defaults write`で古いIDに書いた設定（`server`、`url`、`stickies`）は、引き継がれない。新しいIDに書き直す。
- Androidでは、Kotlinの50個のファイルが、フォルダーごと動く。`git log`で履歴を追うには、`--follow`が要る。
- `memoview-win`では、2つのフォルダーが動き、名前空間が変わる。開いているブランチとworktreeは、この変更と衝突する。
- 改名は、4つのPRに分ける。その間は、新しい名前と古い名前が混ざる。
- ADR 039までのADRは、古い名前で書いてある。書いた時点の記録なので、書き換えない。古い名前は、このADRのDecisionの表で引く。

中立的な結果は、次のとおりである。

- アプリの動きは変わらない。
- 使う人が見る名前（`PetaJournal`、`PetaStickies`、`peta-journal.el`）と、`Peta`で始まる部品の名前は、変わらない。SwiftとC#には、`Peta`が残る。Cargoのパッケージには、`peta-`がない（ADR 037）。この違いは、そのままにする。
- `memoview-win`の設定のファイルは、`~/.peta-stickies`のままである。アプリの名前と合わないが、使う人の設定は、そのまま読める。
- フォルダーの名前とCargoのパッケージの名前は、`web-app/outliner`と`emacs/outliner`では、同じにならない。決まりに沿った違いである。
- `client/libs/`と`journal/`のパッケージ、`atlas/`、`ext-functions/`は、変わらない。
- 改名の順序と、まだ済んでいないPRは、`docs/plans.md`の「リポジトリの構成」に置く。
