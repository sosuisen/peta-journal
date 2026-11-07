# peta-shell-emacs

[English](README.md) | 日本語

PetaJournalのEmacs版フロントエンドである。次の2つからなる。

- `peta-shell-emacs`はRustのプログラムである。Cruxのコア（`peta-core`）を持ち、HTTP、イベントストリーム、タイマーを実行する。
- `lisp/peta-journal.el`はEmacsのモードである。送られてきた行を描画し、利用者の操作を伝える。

両者は、stdinとstdoutのJSON Linesでやり取りする。メッセージは`src/protocol.rs`で定義している。

## セットアップ

プログラムをビルドする。

```powershell
cd peta-journal/frontend-rust
cargo build --release -p peta-shell-emacs
```

Emacsの初期設定ファイルに、次の内容を追加する。

```elisp
(add-to-list 'load-path "<peta-journal>/frontend-rust/crates/shell-emacs/lisp")
(autoload 'peta-journal "peta-journal" nil t)
```

ジャーナルサーバーを起動した状態で、`M-x peta-journal`を実行する。今日のジャーナルが開く。

モードは、`target/release`、`target/debug`、`exec-path`からプログラムを探す。別のプログラムを使うときは、`peta-journal-shell-program`を設定する。Emacs 29.1以降が必要である。

### 別のサーバーを使う

接続先は、既定で`http://localhost:8087`である。別のサーバーを使うときは、Emacsの初期設定ファイルに次の内容を追加し、そのサーバーのアドレスを書く。

```elisp
(setq peta-journal-server "http://localhost:18087")
```

今のセッションだけ変えるときは、`M-:`を押して同じ式を入力する。

この設定は、シェルの起動時にだけ効く。すでにPetaJournalを開いているときは、`C-x k`でそのバッファをすべて閉じる。シェルも一緒に止まる。そのあとで、`M-x peta-journal`を実行し直す。

## 編集

ページ全体が1つのバッファに入る。本文は、他のバッファと同じように入力する。`[[`を打つと`]]`が補われ、続けて`]]`を打つとその上を進む。行頭でバッククォートを3つ打つと、閉じるコードフェンスが補われる。バレットと項目の間の改行は、読み取り専用である。アウトラインは、次のキーでだけ変わる。

| キー | 動作 |
|---|---|
| `RET` | 項目を分割する、または下に項目を追加する |
| `S-RET`、`C-j` | 項目の中で改行する |
| `TAB`、`M-<right>` | 字下げする |
| `<backtab>`、`M-<left>` | 字下げを戻す |
| `M-<up>`、`M-<down>` | 項目を上下に動かす |
| 本文の先頭で`DEL` | 上の項目につなぐ |
| 本文の末尾で`C-d` | 下の項目をつなぐ |
| 本文の末尾で`C-k` | 下の項目をつなぐ |
| `C-c C-t` | タスクの状態を切り替える |
| `C-c C-c` | 子項目を隠す、または表示する |
| `C-c C-d` | 項目を子項目ごと削除する |
| `C-/`、`C-?` | コアの履歴で、元に戻す、やり直す |
| `C-x C-s` | すぐに保存する |

本文は、入力が止まると自動でも保存される。Emacsが`peta-journal-idle-finish-seconds`の間なにもしないと、項目の編集は終わる。編集の間、コアはファイルの変更を取り込まないためである。

次のキーで、他の場所へ移る。

| キー | 動作 |
|---|---|
| `C-c C-j` | 今日のジャーナルを開く |
| `C-c C-f` | ページを名前で開く |
| `C-c C-v` | ページを名前で新しいバッファに開く |
| `C-c C-n`、`C-c C-p` | 次または前のジャーナルを開く |
| `C-c C-l` | 前のページへ戻る |
| `C-c C-o` | カーソル位置のウィキリンクやタグを開く |
| `C-c /` | 検索窓を開く |
| `C-c C-b` | このページのバックリンクを一覧する |

## 2つのページを同時に編集する

`C-c C-v`または`M-x peta-journal-new-buffer`で、ページを新しいバッファに開ける。どちらのページも編集できる。名前を空にすると、今日のジャーナルが開く。バッファごとに、シェルとUndoの履歴が別になる。バッファを閉じても、止まるのはそのバッファのシェルだけである。

そのページを表示中のバッファがあれば、このコマンドはそのバッファへ移る。ページへ移る他のキーでは、1つのページが2つのバッファに表示されることがあり、そのときはEmacsが警告する。この状態は避けること。コアが開いているページを読み直すのはファイルが変わったときだけなので、片方で保存した内容はもう片方に表示されない。同じ項目を2つのバッファで編集すると、編集が互いを上書きするおそれもある。

## 検索窓

`C-c /`を押して、ミニバッファに入力する。結果は下の窓に表示され、入力に合わせて変わる。結果には、ページ、新しいページを作る項目、項目が並ぶ。

| キー | 動作 |
|---|---|
| `C-n`、`<down>` | 次の結果を選ぶ |
| `C-p`、`<up>` | 前の結果を選ぶ |
| `RET` | 選んだ結果を開く |
| `C-g` | 検索窓を閉じて、今のページに留まる |

これらのキーは、入力の直後に押してよい。入力した語の結果が届くのを待ってから動く。

どのバッファからでも検索窓を開くには、Emacsの初期設定ファイルに次の内容を追加する。

```elisp
(autoload 'peta-journal-search "peta-journal" nil t)
(global-set-key (kbd "C-c /") #'peta-journal-search)
```

## この版にないもの

ウィキリンクの補完、画像、地図、拡張機能、設定、複数項目を選択しての操作。

## テスト

`cargo test -p peta-shell-emacs`は、偽のホストを使って、コアを動かすシェルをテストする。

バッファのテストは、プロセスなしで動く。

```powershell
cd peta-journal/frontend-rust/crates/shell-emacs
emacs -Q --batch -L lisp -l lisp/test/peta-journal-test.el -f ert-run-tests-batch-and-exit
```

E2Eテストは、実際のキー入力、本物のシェル、本物のサーバーを使う。`src/runtime.rs`など、他のテストでは動かせない部分を動かす。先に両方のプログラムをビルドする。各テストは、ポート18087でサーバーを起動する。サーバーのファイルは一時フォルダに置き、専用の`--config-dir`を渡す。サーバーの報告するMarkdownフォルダが一時フォルダの外なら、テストは止まる（ADR-002を参照）。`PETA_E2E_SERVER`が設定されていなければ、テストはスキップされる。

```powershell
$env:PETA_E2E_SERVER = "<peta-journal>\journal-server-rust\target\debug\journal-server.exe"
emacs -Q --batch -L lisp -L lisp/test -l lisp/test/peta-journal-e2e.el -f ert-run-tests-batch-and-exit
```

Windowsでは、`emacs`は本物の`emacs.exe`でなければならない。`emacsclient`のラッパーは使えない。
