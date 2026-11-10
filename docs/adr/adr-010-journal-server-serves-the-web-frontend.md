# ADR 010: The Journal Server Serves the Web Frontend

## Status

Accepted

## Context

Web版のフロントエンド（`shell-leptos`）は、Trunkでビルドするwasmのアプリである。2026-11-09の時点で、画面とAPIは別のサーバーから届く。

- 画面は、`trunk serve`がポート4200で配信する。
- APIは、journal-serverがポート8087で受ける。`trunk serve`は、`/api/`と`/assets/`を8087へ中継する（`Trunk.toml`）。

そのため、Web版を使うには、いつも2つのプロセスを起動する必要がある。開発でも普段使いでも同じである。開くURLは4200で、APIのポートとは違う。journal-serverは、4200からのリクエストを受けるためにCORSを許可している。

`trunk serve`には、コードを変えると画面を自動で読み直す機能がある。しかし、この機能は使っていない。

アプリは、APIを相対URL（`/api/...`、`/api/events`、`/assets/...`）で呼ぶ。画面とAPIを同じオリジンから配信すれば、アプリのコードを変えずに動く。

アプリは、ページをパスで表す。`/pages/<name>`、`/today`、`/settings`などである。ブラウザでこれらのURLを直接開いたり、読み直したりすると、サーバーには、ファイルのないパスへのリクエストが届く。

journal-serverはaxumで書かれている。`tower-http`をすでに使っている。

## Decision

journal-serverが、Web版の画面も配信する。開発でも普段使いでも、起動するのはjournal-serverだけにする。開くURLは`http://localhost:8087`になる。

journal-serverは、Trunkが出力したフォルダ（`dist/`）のファイルを配信する。配信には`tower-http`の`ServeDir`を使い、ルーターのfallbackに置く。`/api`、`/mcp`、`/assets`のルートは、今のまま優先される。どのルートにもファイルにも当たらないパスには、`index.html`を返す。

`dist/`の場所は、次の順に決める。

1. `--frontend-dir`の引数か、`PETAJOURNAL_FRONTEND_DIR`の環境変数があれば、その場所を使う。
2. なければ、サーバーの実行ファイルと同じフォルダの`frontend/`を使う。配布するときの置き場所である。
3. どちらもなければ、画面を配信しない。今と同じく、APIだけのサーバーとして動く。

開発では、`journal-server-rust/.cargo/config.toml`の`[env]`に、`PETAJOURNAL_FRONTEND_DIR`として`shell-leptos/dist`の場所を書く。`cargo run`で起動すると、手で設定しなくても画面が配信される。

journal-serverは、リクエストのたびにディスクからファイルを読む。フロントエンドのコードを変えたときは、`trunk build`で`dist/`を作り直し、ブラウザを手で読み直す。journal-serverを再起動する必要はない。

開発ではデバッグ版（`trunk build`）を、配布では小さくした版（`trunk build --release`）を作る。どちらも同じ`dist/`に出力し、journal-serverは同じ方法で配信する。

キャッシュは、次のように扱う。

- `index.html`はキャッシュさせない。作り直したアプリが、読み直しで必ず届くようにするためである。
- 名前にハッシュの付いたjs、css、wasmは、長くキャッシュさせてよい。中身が変わると名前も変わるからである。

## Alternatives

### Alternative 1: 今のまま`trunk serve`で配信する

画面は`trunk serve`が配信し、APIだけをjournal-serverが受ける。

コードを変えると、画面が自動で読み直される。しかし、いつも2つのプロセスを起動する必要がある。開くURLとAPIのURLが違い、CORSの設定も要る。自動の読み直しは使っていないので、利点が効かない。そのため採らなかった。

### Alternative 2: 開発は`trunk serve`、普段使いはjournal-serverで配信する

開発の形を今のまま残し、普段使いと配布のときだけ、journal-serverが配信する。

開発の手順は変わらない。しかし、開発と普段使いで、起動するものと開くURLが違う。2つの構成を知って、保ち続ける必要がある。自動の読み直しを使わないなら、開発で`trunk serve`を残す理由がない。そのため採らなかった。

### Alternative 3: `dist/`をサーバーの実行ファイルに埋め込む

`rust-embed`などで、`dist/`の中身をjournal-serverの実行ファイルに埋め込む。

実行ファイル1つで配布できる。しかし、journal-serverをビルドする前に、必ず`trunk build --release`を実行する必要がある。2つのビルドが結び付く。フロントエンドを変えるたびに、サーバーもビルドし直すことになる。そのため、今は採らない。配布の形を決めるときに見直す。

### Alternative 4: 別のWebサーバーで画面とAPIをまとめる

nginxやCaddyなどの前に置くサーバーで、画面の配信とAPIへの中継をまとめる。

journal-serverは変えずに済む。しかし、利用者が入れて設定するプログラムが1つ増える。手元で動かすノートアプリには重すぎる。そのため採らなかった。

## Confidence Level

高い。アプリはAPIを相対URLで呼ぶので、同じオリジンから配信すればそのまま動く。axumと`tower-http`の静的ファイルの配信は、よく使われている方法である。

次のときに見直す。

- 画面の自動の読み直しが必要になったとき。`trunk watch`と、ブラウザへ読み直しを伝える仕組みを検討する。
- 配布の形を決めるとき。Alternative 3の埋め込みを検討する。
- アプリのページのパスが、サーバーのルートやファイルの名前とぶつかるようになったとき。たとえば、`/pages/`などの前置きのないURLを、アプリが作るようになったときである。

## Consequences

良い結果は、次のとおりである。

- Web版を使うときに起動するのは、journal-serverだけになる。
- 開発と普段使いで、構成が同じになる。開くURLは、どちらも`http://localhost:8087`である。
- 画面とAPIが同じオリジンになるので、CORSの設定がWeb版には要らなくなる。
- SSEが、Trunkの中継を通らずに、journal-serverから直接届く。
- フロントエンドを作り直しても、journal-serverを再起動しなくてよい。

悪い結果は、次のとおりである。

- コードを変えたあとに、`trunk build`とブラウザの読み直しを手で行う必要がある。
- journal-serverが、`dist/`の場所を知っている必要がある。開発では`.cargo/config.toml`に、配布では実行ファイルの隣に置く。
- PlaywrightのE2Eテストは`trunk serve`（ポート4300）を使っている。同じ形に移すまで、開発とテストで構成が違う。

中立的な結果は、次のとおりである。

- `Trunk.toml`の`[serve]`と中継の設定は、E2Eテストを移したあとに不要になる。消すかどうかは、そのときに決める。
  - 2026-11-10の追記: 消すと決めて、消した。`trunk serve`の画面は、APIに届かなくなった。
- CORSの許可は、Web版のためには要らなくなる。ただし、`ext-web-app/todo`のような別のオリジンの外部アプリのために残る。
  - 2026-11-10の追記: 既定値から`http://localhost:4200`を消した。`trunk serve`だけが使っていたためである。既定値は、`ext-web-app/todo`の`http://localhost:5180`だけになった。
- アプリは、前置きのないパス（`/<name>`）もページ名として読む。そのため、`/mcp`や`/favicon.ico`のようなパスは、サーバーのルートかファイルが先に応える。アプリが作るURLは`/pages/...`か`/today`なので、実際の操作では困らない。
