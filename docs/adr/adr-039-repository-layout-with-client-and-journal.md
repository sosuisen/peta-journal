# ADR 039: Repository Layout with client and journal

## Status

Accepted

## Context

リポジトリの直下のフォルダーは、作った順に増えてきた。今は、名前と置き場が中身と合わないものがある。

- `frontend-rust`と`journal-server-rust`の`-rust`は、AngularとQuarkusから移植したときに、元の実装と区別するための名前である。元の実装は、もうない。
- `frontend-`で始まるフォルダーが5つ並ぶ。`frontend-rust`は、ライブラリ（`client-core`など）と、完成したアプリ（Web版とEmacs版）の両方を持つ。ほかの4つは、アプリである。名前からは、この違いが読めない。
- `frontend-winui`と`frontend-swiftui`は道具の名前で、`frontend-android`と`frontend-ios`はOSの名前である。
- `ext-web-app`と`ext-desktop-app`は、REST APIを使うアプリである。アプリなのに、`frontend-`のアプリとは別の並びにある。
- `e2e`は、Web版とジャーナルサーバーを通すテストである。Web版のテストなのに、直下にある。
- `journal-server-rust/crates`のフォルダーは、パッケージの名前から接頭辞を外している（`crates/core`が`journal-core`）。ADR 037は、フロントエンドの側だけを揃えた。
- `journal-core`と`hmtid`は、サーバーだけの部品ではない。AndroidとiOSのアプリは、`journal-core`を組み込む（ADR 029）。`client-core`は、`hmtid`を使う。

`docs/plans.md`の「リポジトリの構成」は、フロントエンドを1つのフォルダーにまとめるか、別のリポジトリに分けるかを、決める項目として持っていた。ADR 037は、直下のフォルダーを変えないと決め、この項目に残した。

アプリは、ライブラリにパス依存している。`client-core`は`hmtid`と`atlas`に、`client-api-embedded`と`client-shell-embedded`は`journal-core`に依存する。APIを変えるときは、ジャーナルの側とアプリの側を、1つのPRで変えている。

## Decision

リポジトリの直下を、`client/`、`journal/`、`atlas/`、`ext-functions/`、`docs/`、`images/`にする。リポジトリは分けない。

```
client/
  Cargo.toml  Cargo.lock  .cargo/
  libs/
    client-core/  client-api-http/  client-api-embedded/
    client-native-bridge/  client-page-links/
    client-shell-http/  client-shell-embedded/
  apps/
    web-app/
      outliner/          ← frontend-rust/crates/client-shell-leptos と e2e
      todo/              ← ext-web-app/todo
    webview-app/
      memoview-win/      ← ext-desktop-app（中の stickies と stickies-tests はそのまま）
    native-app/
      stickies-win/      ← frontend-winui
      stickies-mac/      ← frontend-swiftui
      outliner-android/  ← frontend-android
      outliner-ios/      ← frontend-ios
    emacs/
      outliner/          ← frontend-rust/crates/client-shell-emacs
journal/
  Cargo.toml  Cargo.lock  .cargo/
  journal-core/    ← journal-server-rust/crates/core
  journal-server/  ← journal-server-rust/crates/server
  hmtid/           ← journal-server-rust/crates/hmtid
```

決まりは、次のとおりである。

- `client/`は、ジャーナルを使う側のすべてである。`journal/`は、ジャーナルの側である。ADR 037の`client-`と`journal-`の接頭辞と、同じ分け方である。
- `client/libs/`は、単体では動かないライブラリを置く。`client-shell-http`と`client-shell-embedded`は、2つのアプリが結びつく入口なので、ここに置く。
- `client/apps/`は、完成したアプリを置く。1段目は、アプリの種類である（`web-app`、`webview-app`、`native-app`、`emacs`）。2段目は、アプリの名前である。
- アプリの名前は、使う人から見た役割で付ける。`outliner`はアウトラインを編集するアプリ、`stickies`は付箋で編集するアプリ、`memoview`は付箋で見るだけのアプリである。1つのOSだけで動くアプリには、OSの名前を後ろに付ける（`stickies-win`、`memoview-win`）。道具の名前（`winui`、`swiftui`、`leptos`）は、フォルダーの名前に入れない。
- `client/libs/`と`journal/`のフォルダーは、パッケージと同じ名前にする（ADR 037）。`client/apps/`のフォルダーは、アプリの名前にする。
- このADRの並べ替えでは、フォルダーだけを動かす。パッケージの名前、実行ファイルの名前、アプリの中の名前は変えない。`client-shell-leptos`、`client-shell-emacs`、`Stickies.csproj`、`PetaStickies`、`PetaBorder`は、そのままである。
- アプリのパッケージの名前と中の名前は、あとでフォルダーの名前に合わせて変える。並べ替えとは別のPRで行う。移動と改名を分けると、差分が読みやすく、Gitがファイルの移動を追いやすい。新しい名前は、そのときに決め、`docs/plans.md`の「リポジトリの構成」に項目を置く。
- 並べ替えのあと、改名までの間は、フォルダーの名前とパッケージの名前が合わない（`client/apps/web-app/outliner`が`client-shell-leptos`）。
- アプリのテストは、そのアプリのフォルダーの中に置く。`e2e`は、`client/apps/web-app/outliner/e2e`に移す。
- Cargoのワークスペースは、`client/`と`journal/`の2つである。`client/`のメンバーは、`libs/`の7つと、Web版とEmacs版の2つである。リリースのプロファイルが違うので、1つにまとめない。
- `atlas/`と`ext-functions/`は動かさない。`atlas/`には、sosuisen.netがパス依存している（ADR 038）。`ext-functions/`は、サーバーが実行する拡張であり、アプリではない。
- 並べ替えは、1つのPRで行う。同じフォルダーを2回動かさない。
- ADR 037の「リポジトリの直下のフォルダーは変えない」と、「ジャーナルの側のフォルダーの名前は別に決める」は、このADRで置き換える。

## Alternatives

### Alternative 1: アプリを別のリポジトリに分ける

アプリごとに、または`client/`の全体を、別のリポジトリにする。リポジトリは小さくなり、アプリごとにCIを持てる。しかし、アプリはライブラリにパス依存している。分けるには、`journal-core`、`hmtid`、`peta-atlas`を公開するか、Gitの依存にする必要がある。APIを変えるたびに、2つのリポジトリで版を合わせる。ADR 038のきっかけは、別のリポジトリのsosuisen.netが、フォルダーの改名で止まったことだった。そのため選ばなかった。`todo`と`memoview-win`は、REST APIとnpmのパッケージだけに頼るので、あとで切り出せる。

### Alternative 2: 直下に`apps/`と`client-libs/`を置く

`client/`を作らず、直下に2つのフォルダーを置く。パスは1段浅い。しかし、Cargoのワークスペースのルートが、リポジトリの直下になる。直下に`Cargo.toml`、`Cargo.lock`、`.cargo/`、`target/`が並び、`journal/`と`atlas/`を`exclude`に書く必要がある。そのため選ばなかった。

### Alternative 3: `journal-core`と`journal-server`を、直下の別のフォルダーにする

`journal-core`は、サーバーなしで使えるライブラリなので、`atlas/`のように直下に置く。独立したライブラリであることが、構成から読める。しかし、`journal-core`のテストの多くは、サーバーの結合テストである（139個と388個）。カバレッジとミューテーションテストは、2つを1つのワークスペースで測っている。分けると、`journal-core`だけでは確かめられない。そのため、今は選ばなかった。

### Alternative 4: ジャーナルの側を`server/journal-api-http`と`server/journal-core`にする

クライアントの側の`client-api-http`と、対になる名前にする。しかし、`journal-core`は、AndroidとiOSのアプリも組み込むので、サーバーの部品ではない。サーバーのクレートは、RESTのほかに、MCP、画面の配信、拡張の実行を持つ。パッケージと実行ファイルの名前も`journal-server`である。そのため選ばなかった。

### Alternative 5: ネイティブのアプリのフォルダーを、道具の名前のままにする

`winui`、`swiftui`のように、作るのに使った道具で呼ぶ。今の名前に近く、改名が少ない。しかし、AndroidとiOSはOSの名前であり、決まりが2つ混ざる。道具の名前は、何をするアプリかを言わない。そのため選ばなかった。

### Alternative 6: 移動と一緒に、パッケージの名前も変える

フォルダーとパッケージの名前が合わない期間がなくなる。しかし、1つのPRに、移動と、`use`や実行ファイルの名前の書き換えが混ざる。差分が大きくなり、Gitが移動を追いにくくなる。そのため選ばなかった。

## Confidence Level

確信度は、分け方については高く、ビルドの配線については中くらいである。

分け方は、すでにある依存の向きに沿っている。アプリは`client/libs/`に依存し、`client/libs/`は`journal/`と`atlas/`に依存する。逆向きの依存はない。`client-`と`journal-`の分け方は、ADR 037で決めたものと同じである。

Cargoのワークスペースは、一時のworktreeで確かめた。直下に`apps/`と`client-libs/`を置く形で、`cargo metadata`が2つのワークスペースを解決できた。`client/`の下にメンバーを並べる形は、Cargoの普通の形であり、同じように解決できると見ている。

まだ確かめていないのは、次の4つである。並べ替えのPRの中で確かめる。

- Trunkが、`client/apps/web-app/outliner`から、`../../../../atlas/web/`のスクリプトを読めるか。
- BoltFFIの`pack`と、4つのネイティブのアプリの`generate`が、新しい場所で動くか。
- AndroidのGradleと、iOSとmacOSの`project.yml`が、新しい場所のライブラリを見つけるか。
- `cargo cov`と`cargo-mutants`の設定が、新しいフォルダーの名前で動くか。

CIは、2026-11-25まで止まっている。それまでは、手元の確認だけが頼りである。SwiftUIとiOSのビルドは、Macで確かめる。

次のどれかが起きたら、この決定を見直す。

- `journal-core`のテストが、サーバーの結合テストに頼らなくなったとき。`journal-core`を直下に出すかを決める（Alternative 3）。
- `journal-core`、`hmtid`、`client-core`のどれかを、crates.ioに公開するとき。リポジトリを分ける費用が下がる。
- `todo`か`memoview-win`を、ほかの人が使う例として配るとき。別のリポジトリに切り出すかを決める。
- アプリの種類が増えて、`web-app`、`webview-app`、`native-app`、`emacs`の4つに収まらなくなったとき。
- リリースのプロファイルの違いがなくなったとき。`client/`と`journal/`のワークスペースを、1つにまとめるかを決める。

## Consequences

良い結果は、次のとおりである。

- 直下のフォルダーが、13個から6個になる。直下を見れば、クライアント、ジャーナル、アトラスの3つの部分が分かる。
- アプリとライブラリが、フォルダーで分かれる。`client/apps/`を見れば、アプリの一覧が分かる。
- `-rust`と`frontend-`が消える。移植のときの名前が残らない。
- ジャーナルの側のフォルダーも、パッケージと同じ名前になる。ADR 037が残した項目が片付く。
- `todo`と`memoview-win`が、ほかのアプリと同じ並びになる。
- Web版のE2Eのテストが、Web版のフォルダーにある。EmacsのE2Eと、同じ置き方になる。
- `todo`と`memoview-win`は、フォルダーごと履歴付きで切り出せる。

悪い結果は、次のとおりである。

- ほぼすべてのファイルのパスが変わる。パス依存を持つ`Cargo.toml`、`.cargo/config.toml`、`Trunk.toml`、`index.html`、`build-frontend`、4つのアプリの`generate`、Gradle、`project.yml`、Playwrightの設定、CI、`BUILD.md`、READMEを直す。
- パスが深くなる。Web版は、`client/apps/web-app/outliner`になる。コマンドと文書のパスが長くなる。
- 使う人は、手元の設定を直す。Emacsの`load-path`、`build-frontend`の出力先、`--frontend-dir`、`target/`にある古いビルドである。`target/`は、新しい場所で作り直しになる。
- 開いているブランチとworktreeは、この変更と大きく衝突する。並べ替えの前に、マージするか、並べ替えのあとに作り直す。
- 改名までの間は、アプリのフォルダーの名前とパッケージの名前が合わない。
- `git log`でファイルの履歴を追うには、`--follow`が要る。
- ADR 038までのADRと、`docs/`の計画の文書は、古いパスで書いてある。書いた時点の記録なので、書き換えない。古いパスは、このADRのDecisionの図で引く。

中立的な結果は、次のとおりである。

- パッケージの名前、実行ファイルの名前、アプリの動きは変わらない。
- `atlas/`は動かない。sosuisen.netは、何も直さなくてよい。
- `client/`の`Cargo.lock`と、`flatmarkdown-ast`の版の固定は、今のまま1つである。
- `journal-core`のテストの多くが、サーバーの結合テストである状態は、そのまま残る。`docs/plans.md`の「journal-core」の節に、テストを移す項目を足す。
- アプリのパッケージの名前と中の名前を変える項目を、`docs/plans.md`の「リポジトリの構成」に足す。
