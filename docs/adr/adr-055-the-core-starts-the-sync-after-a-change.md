# ADR 055: The Core Starts the Sync after a Change

## Status

Accepted

## Context

AndroidとiOSのアプリは、ページが変わると、少し待ってからDropboxと同期する（ADR 031）。
同期の決まりは、ADR 054でRustのクレート`client-cloud-sync`にまとめた。
しかし、同期を始めるきっかけは、まだ両方のシェルにある。

- シェルの`Shell`（Androidは`Shell.kt`、iOSは`Shell.swift`）が、どの操作をページの変更と見なすかを決める。
  ページを変える操作は`change`で、変えない操作は`send`で、コアに送る。
  `change`は、送ったあとに`onEdit`を呼ぶ。
  TodayとCreateを選んだときと、`make`と`today`のあとにも、`onEdit`を呼ぶ。
- シェルの`JournalModel`が、`onEdit`のたびに3秒のタイマーを掛け直し、時間が来たら同期を始める。
  Androidは`Handler.postDelayed`で、iOSは`Timer`で待つ。
- シェルの`JournalModel`が、Markdownフォルダーの中に`docs`、`journals`、`assets`のフォルダーを作る（ADR 030）。
  `journal-core`は、`docs`と`journals`のどちらもないフォルダーからは、何も取り込まないためである。
saa
この分け方には、取り違えの余地がある。
2026-11-17に調べると、Androidの`Shell.answer`は`onEdit`を呼ばない。
コアが「ほかの項目が写している項目を消しますか」と尋ね、ユーザーが「はい」と答えると、項目は消える。
しかし、アプリは同期を始めない。
消えた項目がDropboxに届くのは、次に別の編集をしたときか、アプリが前に出たときである。
新しい操作を足すときも、シェルを書く人が、`change`と`send`のどちらで送るかを2か所で正しく選ぶ必要がある。

`journal-core`には、ページの変更を知る方法が2つある。

- 変更の知らせ（`JournalEvents::page_changed`）。
  アプリの操作でページが変わると、`pagesUpdated`（`source`が`app`）のメッセージが出る。
  ただし、ページを作る`get_or_create_doc`と`get_or_create_journal`は、知らせを出さない。
  TodayとCreateで作ったページは、この知らせでは分からない。
- 書き出しの待ち行列（`export_queue`）。
  ページの変更のうち、まだMarkdownファイルに書かれていないものが、ここで待つ。
  `MarkdownExportService::process_whole_queue`が、待っているページをファイルに書く。
  作ったページも、変えたページも、ここに入る。

フロントのコア（Crux）は、ジャーナルへの要求を`ApiOp`として送る。
組み込み版では、`client-shell-embedded`の`Door::call`が、すべての`ApiOp`を受ける。
ダウンロードしたファイルの取り込み（`PetaCore::sync`）は、`Door::call`を通らない。

## Decision

ページが変わったことは、Rustが知り、Rustが同期を始める。
シェルは、どの操作がページを変えるかを決めない。

### 変更を知る

`client-shell-embedded`の`Door::call`は、`ApiOp`を`journal-core`に渡したあと、書き出しの待ち行列を見る。
待っているページが1つでもあれば、アプリでページが変わったと見なし、同期のクレートに知らせる。
待ち行列の数は、`ExportQueueRepository::waiting`で読む。

読むだけの`ApiOp`は、待ち行列にページを足さない。
そのため、`ApiOp`を読むものと書くものに分ける表は作らない。
ダウンロードしたファイルの取り込みは`Door::call`を通らないので、取り込みが書き直したページは、同期を始めない。
取り込みが書き直したページは、同じ回の同期がアップロードする（ADR 054の「同期の流れ」）。

### 待ってから同期する

`client-cloud-sync`の`CloudSync`に、ページが変わったことを受ける`changed`を足す。
`changed`は、待ち時間のタイマーを掛け直す。
時間が来たら、`CloudSync`は、最後の`start`で受けたDropboxの呼び出し（`Cloud`）とフォルダーで、同期を始める。
ユーザーは待たない（`in_front`は`false`）。
待ち時間は3秒で、シェルが`PetaCloudSync`を作るときに渡す。
テストは、短い待ち時間で作る。

まだ`start`を受けていないときと、サインアウトしたあとは、`changed`は何もしない。
`sign_out`は、覚えていたDropboxの呼び出しを捨てる。
同期が動いている間に`changed`を受けたときは、時間が来てから`start`を呼ぶ。
`start`は、動いている同期のあとに、もう1回の同期を入れる（ADR 054の今の振る舞い）。

### 2つをつなぐ

シェルは、`PetaCore`を作ったあとに`PetaCloudSync`を作る。
`PetaCloudSync::new`は、`PetaCore`を受け取り、変更の知らせを`changed`につなぐ。
先に確かめること：BoltFFI 0.32.1で、外に出した構造体（`PetaCore`）を、別の構造体の作り手の引数にできるか。
できないときは、`PetaShell`に`page_changed`を足し、シェルがそれを`PetaCloudSync::changed`に渡す。
この場合も、どの操作がページを変えるかは、Rustが決める。

### フォルダーを作る

`PetaCore::new`が、Markdownフォルダーの中に`docs`、`journals`、`assets`のフォルダーを作る。

### シェルに残るもの

- アプリが前に出たときと、サインインのあとの、ユーザーが待つ同期（`syncInFront`）。
- 「Sync」の操作と、サインアウト。

### シェルから消えるもの

- `Shell`の`onEdit`と、`change`と`send`の使い分け。すべての操作を`send`で送る。
- `JournalModel`の3秒のタイマー（Androidの`afterEdit`、iOSの`afterEdit`）。
- `JournalModel`の、フォルダーを作る処理。
- これらを確かめるシェルのテスト。同じことを、Rustのテストで確かめる。

## Alternatives

### Alternative 1: 変更の知らせ（`pagesUpdated`）で知る

`journal-core`の`JournalEvents::page_changed`を受け、`ChangedBy::Client`のときに同期を始める。
ページの変更をいちばん直接に表す。
しかし、ページを作る操作は知らせを出さないので、TodayとCreateで作ったページが上がらない。
作る操作に知らせを足すと、Webのアウトライナーやほかのクライアントも、ページを読み直すようになる。
シェルの外への影響が大きいので、選ばなかった。

### Alternative 2: `ApiOp`を読むものと書くものに分ける

`Door::call`で、書く`ApiOp`のときに同期を始める。
待ち行列を読まずに済む。
しかし、`ApiOp`は44種類あり（2026-11-17）、足すたびに表を直す必要がある。
シェルの`change`と`send`の取り違えが、Rustの表の取り違えに変わるだけなので、選ばなかった。

### Alternative 3: タイマーをシェルに残す

Rustは変更を知らせるだけにし、3秒の待ちと同期の開始は、今のシェルの`JournalModel`のままにする。
FFIの変更が小さい。
しかし、同じタイマーがKotlinとSwiftに残る。
ADR 054で同期の決まりをRustにまとめたので、きっかけの待ちもRustに置くほうが、まとまりがよい。
選ばなかった。

## Confidence Level

確信度は中程度である。

待ち行列で変更を知る方法は、ページを作る操作も変える操作も、1か所で拾える。
一方で、次の2つは確かめていない。
第一に、ファイルに書けないページが待ち行列に残り続けると、読むだけの操作のたびにも同期が始まる。
今は、書けないページが残る場面を見ていない。
第二に、BoltFFIで2つの構造体をつなげるか。
つなげないときの代わりは、Decisionに書いた。

次の状況になったら、この決定を見直す。

- 待ち行列に残り続けるページが実際に見つかり、同期が多すぎるとき。
  待ち行列の数が増えたときだけ知らせるように変える。
- `journal-core`が、ページを作る操作でも変更の知らせを出すようになったとき。
  Alternative 1のほうが直接になる。
- 同期のほかに、変更を知りたいものがシェルにできたとき。

## Consequences

### 良い結果

- どの操作がページを変えるかを、シェルが決めなくなる。
  Androidの`Shell.answer`のような呼び忘れが、起きなくなる。
- 新しい操作を足しても、同期のために何かを足す必要がない。
- 3秒の待ちとフォルダーを作る処理が、KotlinとSwiftから消える。
- 同期を始める決まりを、Rustのテストで1回だけ確かめる。

### 悪い結果

- 同期が始まるのは、キーを打ったときではなく、コアが本文を保存したあとになる。
  コアの自動保存の待ちの分だけ、アップロードが遅れる。
- `Door::call`のたびに、待ち行列の数をデータベースに尋ねる。
- `CloudSync`が、最後に受けたDropboxの呼び出しを覚える。
  サインインし直したときは、新しい`start`が置き換える。

### 中立的な結果

- `PetaCloudSync::new`の引数が変わる。
  AndroidとiOSのシェルは、同じPRで直す。
- 取り込みが書き直したページは、今と同じく、同じ回の同期がアップロードする。
