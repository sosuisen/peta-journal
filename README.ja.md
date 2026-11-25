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

画面は、React版のシェル（[ADR 079](docs/adr/adr-079-a-typescript-web-shell-that-applies-list-diffs.md)）を`npm run build`で`client/apps/web-app/outliner-react/dist/`に出力したファイルである。journal-serverは、リクエストのたびにこのフォルダからファイルを読む。

### 準備（一度だけ）

- `rustup`。Rustの版とwasmのターゲットは、`rust-toolchain.toml`が決める。最初のビルドで、rustupが入れる
- Node.js
- wasm-pack: `cargo install wasm-pack`

Windows ARM64では、[README.md](README.md)の「Build」にある準備も行う。

### デバッグ（開発）

画面をビルドし、journal-serverを`cargo run`で起動する。

```powershell
cd peta-journal/client/apps/web-app/outliner-react
npm install
npm run build

cd ../../../../journal
cargo run
```

ブラウザで`http://localhost:8087`を開く。

- 画面のフォルダは、指定しなくてよい。`cargo run`では、`journal/.cargo/config.toml`が`PETAJOURNAL_FRONTEND_DIR`に`dist/`の場所を設定する。
- 画面のコードを変えたら、`npm run build`をやり直して、ブラウザを読み直す。journal-serverは再起動しなくてよい。
- サーバーのコードを変えたら、Ctrl+Cで止めて、`cargo run`をやり直す。
- 起動時のログに`Frontend: <フォルダ>`と出れば、画面が配信されている。`Frontend: not served`と出たら、配信されていない。

`cargo run`は、普段使いと同じデータ（`~/.peta-journal`）を読み書きする。普段のデータに触れずに試すときは、ポートと`--throwaway-dir`を指定する。

```powershell
cargo run -- --port 8088 --throwaway-dir <作業フォルダ>
```

- データ、検索の索引、設定、Markdownフォルダは、すべて`<作業フォルダ>`の下（`data/`、`index/`、`config/`、`markdown/`）に置かれる。
- Markdownフォルダは、`<作業フォルダ>`の外には出ない。設定の画面で外のフォルダを指定しても、サーバーは受け付けない。
- この機械からの接続だけを受ける（`::1`と`127.0.0.1`）。別のPCからはつながらない。
  - 理由: 外からの接続を待ち受けると、Windowsはプログラムのパスごとに、ファイアウォールの許可を聞く。ワークツリーごとにパスが変わるので、テストのたびにダイアログが出て、画面のテストの邪魔をする（#1150）。
- `--data-dir`だけを付けて起動してはならない。`~/.peta-journal/config.json`にある普段のMarkdownフォルダへ、試したページが書き出される（ADR 002）。
- 開くURLは、指定したポート（上の例では`http://localhost:8088`）になる。

### 本番（普段使い）

画面とサーバーをリリース版でビルドし、実行ファイルを直接起動する。

リポジトリのルートの`release-all.ps1`（macOS、Linux、Git Bashでは`release-all.sh`）が、次の手順をまとめて行う。

1. `git pull --ff-only`で最新にする。`-NoPull`（shでは`--no-pull`）を付けると、pullしない。pullでスクリプト自身が変わったときは、新しい版を起動し直す。
2. PetaStickiesとMemoViewを閉じ、journal-serverを`Ctrl+C`で止める。
3. 画面、journal-server、Emacs版、todo、PetaStickies、MemoViewを、リリース版でビルドする。画面は、実行ファイルの隣の`frontend/`にコピーする。
4. todoのリリース版（`dist/`）を、設定フォルダの`apps/todo/`にコピーする（ADR 073）。前のビルドのファイルは、先に消す。設定フォルダは`~/.peta-journal`で、環境変数`PETAJOURNAL_CONFIG_DIR`があればそれを使う。
5. journal-serverを起動し、ポート8087が応答するのを待つ。
6. ブラウザで`http://localhost:8087`と`http://localhost:8087/apps/todo/`を開き、PetaStickiesを起動する。

Emacs版のプログラムが動いているときは、そのビルドを飛ばす。EmacsでPetaJournalのバッファを閉じてから、もう一度実行する。

手で行うときは、次のようにする。

```powershell
cd peta-journal/client/apps/web-app/outliner-react
npm run build

cd ../../../../journal
cargo build --release
./target/release/journal-server --frontend-dir ../client/apps/web-app/outliner-react/dist
```

ブラウザで`http://localhost:8087`を開く。データは`~/.peta-journal`に置かれる。

直接起動した実行ファイルには、`.cargo/config.toml`の設定が効かない。画面のフォルダは、次のどちらかで伝える。どちらもなければ、journal-serverはAPIだけを配信する。

1. `--frontend-dir`の引数か、`PETAJOURNAL_FRONTEND_DIR`の環境変数で、フォルダを指定する。上の例はこの方法である。
2. `dist/`の中身を、実行ファイルの隣の`frontend/`フォルダに置く。引数なしで起動できる。配布するときの形である。

   ```
   journal-server（Windowsでは journal-server.exe）
   frontend/
     index.html
     index-<ハッシュ>.js
     ...
   ```

開発のビルドと普段使いのビルドは、同じ`dist/`に出力される。本番のサーバーが`dist/`を直接読んでいると、開発で`npm run build`を実行したときに、本番の画面も変わる。開発と普段使いを同じPCで続けるなら、2の方法で、`dist/`の中身を`frontend/`にコピーして使う。

更新するときは、次のようにする。

- 画面だけを更新する: `npm run build`をやり直す。`frontend/`を使っているなら、中身をコピーし直す。そのあと、ブラウザを読み直す。
- サーバーを更新する: journal-serverを止めて、`cargo build --release`をやり直し、起動し直す。

ポートやデータの場所などの設定は、[BUILD.md](BUILD.md)の「Rust server」にある。
