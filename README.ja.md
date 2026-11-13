<img alt="PetaJournal" src="https://github.com/sosuisen/peta-journal/blob/main/images/peta-journal.png" width=60 height=60 align="left"> 

# PetaJournal

<br>

[English](README.md) | 日本語

ローカルのMarkdownファイルの上で動く、ヘッドレスでオフラインファーストのアウトライナーである。

このファイルには、Web版の起動手順を書く。ほかの内容は、次の文書にある。

- 機能の説明とフォルダ同期: [README.md](README.md)
- ビルドの詳細、サーバーの設定の一覧、テスト: [BUILD.md](BUILD.md)
- Emacs版: [client/apps/emacs/outliner/README.ja.md](client/apps/emacs/outliner/README.ja.md)

## 起動手順

Web版を使うときに起動するのは、journal-serverだけである。journal-serverが、APIと画面の両方を`http://localhost:8087`で配信する（[ADR 010](docs/adr/adr-010-journal-server-serves-the-web-frontend.md)）。

画面は、Trunkが`client/apps/web-app/outliner/dist/`に出力したファイルである。journal-serverは、リクエストのたびにこのフォルダからファイルを読む。

### 準備（一度だけ）

- Rust 1.85以上
- wasmのターゲット: `rustup target add wasm32-unknown-unknown`
- Trunk: `cargo install trunk --locked`

Windows ARM64では、[README.md](README.md)の「Build」にある準備も行う。

### デバッグ（開発）

画面をデバッグ版でビルドし、journal-serverを`cargo run`で起動する。

```powershell
cd peta-journal/client/apps/web-app/outliner
trunk build

cd ../../../../journal
cargo run
```

ブラウザで`http://localhost:8087`を開く。

- 画面のフォルダは、指定しなくてよい。`cargo run`では、`journal/.cargo/config.toml`が`PETAJOURNAL_FRONTEND_DIR`に`dist/`の場所を設定する。
- 画面のコードを変えたら、`trunk build`をやり直して、ブラウザを読み直す。journal-serverは再起動しなくてよい。
- サーバーのコードを変えたら、Ctrl+Cで止めて、`cargo run`をやり直す。
- 起動時のログに`Frontend: <フォルダ>`と出れば、画面が配信されている。`Frontend: not served`と出たら、配信されていない。

`cargo run`は、普段使いと同じデータ（`~/.peta-journal`）を読み書きする。普段のデータに触れずに試すときは、ポートと3つのフォルダをすべて指定する。

```powershell
cargo run -- --port 8088 --data-dir <作業フォルダ>/data --index-dir <作業フォルダ>/index --config-dir <作業フォルダ>/config
```

- `--config-dir`は必ず付ける。付けないと、`~/.peta-journal/config.json`にある普段のMarkdownフォルダへ、試したページが書き出される。
- 初めて起動すると、Markdownフォルダを選ぶダイアログが開く。普段とは別のフォルダを選ぶ。
- 開くURLは、指定したポート（上の例では`http://localhost:8088`）になる。

### 本番（普段使い）

画面とサーバーをリリース版でビルドし、実行ファイルを直接起動する。

```powershell
cd peta-journal/client/apps/web-app/outliner
trunk build --release

cd ../../../../journal
cargo build --release
./target/release/journal-server --frontend-dir ../client/apps/web-app/outliner/dist
```

ブラウザで`http://localhost:8087`を開く。データは`~/.peta-journal`に置かれる。

直接起動した実行ファイルには、`.cargo/config.toml`の設定が効かない。画面のフォルダは、次のどちらかで伝える。どちらもなければ、journal-serverはAPIだけを配信する。

1. `--frontend-dir`の引数か、`PETAJOURNAL_FRONTEND_DIR`の環境変数で、フォルダを指定する。上の例はこの方法である。
2. `dist/`の中身を、実行ファイルの隣の`frontend/`フォルダに置く。引数なしで起動できる。配布するときの形である。

   ```
   journal-server（Windowsでは journal-server.exe）
   frontend/
     index.html
     outliner-web-<ハッシュ>.js
     ...
   ```

デバッグ版とリリース版の画面は、同じ`dist/`に出力される。本番のサーバーが`dist/`を直接読んでいると、開発で`trunk build`を実行したときに、本番の画面もデバッグ版に変わる。開発と普段使いを同じPCで続けるなら、2の方法で、リリース版の`dist/`の中身を`frontend/`にコピーして使う。

更新するときは、次のようにする。

- 画面だけを更新する: `trunk build --release`をやり直す。`frontend/`を使っているなら、中身をコピーし直す。そのあと、ブラウザを読み直す。
- サーバーを更新する: journal-serverを止めて、`cargo build --release`をやり直し、起動し直す。

ポートやデータの場所などの設定は、[BUILD.md](BUILD.md)の「Rust server」にある。
