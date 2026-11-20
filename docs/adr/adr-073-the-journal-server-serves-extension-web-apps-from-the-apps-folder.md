# ADR 073: The Journal Server Serves Extension Web Apps from the Apps Folder

## Status

Accepted

## Context

ADR 009は、画面を必要とする拡張を、サーバーと通信する外部アプリとして作ると決めた。
その例が、Todoアプリ（`client/apps/web-app/todo`）である。
Todoアプリは、React、Vite、TypeScriptで書かれたWebアプリである。

2026-11-20の時点で、Todoアプリを使う手順は次のとおりである。

1. journal-serverを起動する。
   APIはポート8087で受ける。
2. Todoアプリのフォルダで、Viteの開発サーバーを起動する。
   画面はポート5180で配信される。
3. ブラウザで`http://localhost:5180`を開く。
   journal-serverが8087以外のポートで動くときは、`?host=&port=`をURLに付ける。

画面とAPIは別のオリジンから届く。
そのため、journal-serverは、CORSの既定値に`http://localhost:5180`を持っている。
Todoアプリのためだけの設定である。

拡張Webアプリを1つ使うたびに、プロセスを1つ起動し、ポートを1つ覚える必要がある。
拡張Webアプリが増えるほど、この手間が増える。

ADR 010で、journal-serverはWeb版の画面（`shell-leptos`）を配信するようになった。
配信は`journal-server/src/presentation/frontend.rs`にあり、`tower-http`の`ServeDir`を使っている。
ファイルはリクエストのたびにディスクから読むので、ファイルを置き換えてもサーバーの再起動は要らない。

journal-serverは、設定フォルダ（`--config-dir`、既定は`~/.peta-journal`）を持つ。
設定フォルダには、`config.json`と、拡張関数の`functions/`がある。

## Decision

journal-serverが、拡張Webアプリの静的ファイルも配信する。
拡張Webアプリを使うときに起動するのは、journal-serverだけにする。

配信の仕方は、すべて規約で決める。
拡張Webアプリごとの設定ファイルは作らない。
フォルダを置くだけで配信が始まり、消せば止まるようにするためである。

### 置き場所とURL

拡張Webアプリは、設定フォルダの`apps/{url_fragment}/`に置く。
journal-serverは、そのフォルダを`/apps/{url_fragment}/`で配信する。
フォルダの名前が、そのままURLの一部になる。

たとえば、Todoアプリをビルドして、`dist/`の中身を`~/.peta-journal/apps/todo/`に置く。
すると、`http://localhost:8087/apps/todo/`でTodoアプリが開く。

URLには`/apps/`の前置きを付ける。
`/api`、`/mcp`、`/assets`、`/pages`、`/today`、`/settings`などのパスとぶつからないようにするためである。
前置きがあるので、フォルダの名前は自由に付けてよい。

### 配信するファイル

- `apps/{url_fragment}/`のフォルダそのものを、配信のルートにする。
  `dist/`のようなサブフォルダは探さない。
- 拡張子のないパスで、ファイルがないときは、そのアプリの`index.html`を返す。
  拡張Webアプリが、`/apps/todo/tasks/1`のようなパスで画面を表せるようにするためである。
- 拡張子のあるパスで、ファイルがないときは、404を返す。
  `app.js`を置き忘れたときに、HTMLがJSとして読まれて分かりにくいエラーになるのを防ぐためである。
- `apps/{url_fragment}/`のフォルダがないときは、404を返す。
- `/apps/{url_fragment}`のように末尾の`/`がないときは、`/apps/{url_fragment}/`へリダイレクトする。
  拡張Webアプリの相対URLが、正しいフォルダを指すようにするためである。

### キャッシュ

すべてのファイルを`Cache-Control: no-cache`で返す。
ブラウザは、読み込むたびにファイルが変わったかをjournal-serverに確かめる。
拡張Webアプリのビルドツールは、Trunkとは別のハッシュをファイル名に付ける。
journal-serverはビルドツールごとの名前の形を知りたくないので、ハッシュで長くキャッシュさせることはしない。
journal-serverは手元で動くので、確かめる手間は小さい。

### 拡張Webアプリの側の約束

拡張Webアプリは、次の2つを守る。

- アセットを相対URLで参照する。
  Viteなら`base: './'`にする。
  フォルダの名前を変えても、そのまま動くようにするためである。
- APIのURLの既定値を、画面と同じオリジンにする。
  つまり、`/api/...`と`/api/events`を相対URLで呼ぶ。

Todoアプリは、この約束に合わせて直す。
今のTodoアプリは、APIのURLの既定値を`http://localhost:8087`にしている。
`?host=&port=`は、別のサーバーにつなぐときのために残す。

### Web版の画面との関係

拡張Webアプリの配信は、ADR 010の画面の配信とは別に動く。
`--frontend-dir`がなく、Web版の画面を配信しないときでも、拡張Webアプリは配信する。
`/apps/`のルートは、ADR 010のfallbackより先に置く。

## Alternatives

### Alternative 1: 拡張Webアプリごとに設定ファイルを置く

`apps/{url_fragment}/app.json`などに、URLの前置き、配信のルート、`index.html`を返すかを書く。

拡張Webアプリごとに配信の仕方を変えられる。
しかし、拡張Webアプリを足すたびに設定ファイルを書く手間が増える。
今ある拡張Webアプリは、どれも同じ配信の仕方で足りる。
そのため採らなかった。

### Alternative 2: 拡張Webアプリごとにポートを分ける

journal-serverが、拡張Webアプリごとに別のポートで配信する。

拡張Webアプリごとにオリジンが分かれる。
`localStorage`などが、Web版の画面やほかの拡張Webアプリと混ざらない。
しかし、ポートを割り当てて覚える仕組みが要る。
画面とAPIのオリジンが分かれるので、CORSの設定も残る。
そのため採らなかった。

### Alternative 3: フォルダの名前をURLの直下に置く

`apps/todo/`を`/todo/`で配信する。

URLは短くなる。
しかし、`/api`や`/pages`などのパスとぶつかる。
使えないフォルダの名前を決めて、保ち続ける必要がある。
そのため採らなかった。

### Alternative 4: ファイルがないときは常に`index.html`を返す

ADR 010と同じく、拡張子があってもなくても、ファイルのないパスには`index.html`を返す。

規約は1つ減る。
しかし、ファイルを置き忘れたときに、HTMLがJSやCSSとして読まれる。
ブラウザには、原因の分かりにくいエラーが出る。
拡張Webアプリは開発者が手で置くので、置き忘れは起こりやすい。
そのため採らなかった。

### Alternative 5: 今のまま拡張Webアプリが自分で配信する

拡張Webアプリは、Viteなどの開発サーバーで配信する。

journal-serverは変えずに済む。
しかし、拡張Webアプリごとにプロセスを起動し、ポートを覚える手間が残る。
CORSの設定も残る。
そのため採らなかった。

## Confidence Level

高い。
配信の仕組みは、ADR 010で使っている`ServeDir`と同じである。
拡張Webアプリが相対URLでAPIを呼べば、同じオリジンからそのまま動く。

次のときに見直す。

- ある拡張Webアプリだけ、別の配信の仕方が要るようになったとき。
  Alternative 1の設定ファイルを検討する。
- 開発者が書いていない拡張Webアプリを配って動かすようになったとき。
  同じオリジンの拡張Webアプリは、すべてのAPIを呼べて、Web版の画面の`localStorage`も読める。
  Alternative 2のオリジンを分ける形を検討する。
- 拡張Webアプリの読み込みが、キャッシュがないために目に見えて遅くなったとき。

## Consequences

良い結果は、次のとおりである。

- 拡張Webアプリを使うときに起動するのは、journal-serverだけになる。
- 拡張Webアプリを足すには、設定フォルダの`apps/`にフォルダを置くだけでよい。
  設定ファイルも、journal-serverの再起動も要らない。
- 画面とAPIが同じオリジンになるので、拡張WebアプリのためのCORSの設定が要らなくなる。
- 拡張Webアプリは、`?host=&port=`を付けなくても、自分を配信したjournal-serverにつながる。

悪い結果は、次のとおりである。

- 拡張Webアプリは、相対URLでアセットとAPIを参照する必要がある。
  今のTodoアプリは、Viteの設定とAPIのURLの既定値を直す必要がある。
- 拡張Webアプリのコードを変えたあとは、ビルドして、設定フォルダの`apps/`へ置き直す必要がある。
- 同じオリジンで動くので、拡張Webアプリはすべての`/api`を呼べる。
  Web版の画面や、ほかの拡張Webアプリの`localStorage`も読める。
  開発者が自分で置く拡張Webアプリなので、今は問題にしない。
- `/apps`という名前のページを、前置きのないパス（`/apps`）で開くことはできなくなる。
  ADR 010の見直し条件の1つに当たる。
  アプリが作るURLは`/pages/...`なので、実際の操作では困らない。

中立的な結果は、次のとおりである。

- CORSの既定値の`http://localhost:5180`は、Todoアプリを直したあとに不要になる。
  Viteの開発サーバーでTodoアプリを開発するときのために残すかは、そのときに決める。
- 拡張Webアプリのファイルは、すべて`no-cache`で返る。
  ADR 010のWeb版の画面とは、キャッシュの扱いが違う。
