# ADR 067: The Windows Screen Tests Start Their App on an STA Thread

## Status

Accepted

## Context

### ADR 059の起動の決定

Windowsの付箋紙（`stickies-win`）の画面のテストは、`PetaScreens.Tests`にある（ADR 059）。
ADR 059は、テストのアプリの起動を、別ホスト型でMSTestに任せると決めた。
`PetaScreens.Tests`の`TestApp.cs`に`[assembly: WinUITestTarget(typeof(TestApp))]`を書き、最初の`[UITestMethod]`の前に、MSTestが`TestApp`を起動する形である。

### MSTestが作るスレッドはMTAである

MSTestの`UITestMethodAttribute`は、`new Thread`でスレッドを作り、その中で`Application.Start`を呼ぶ。
MSTestは、このスレッドに`SetApartmentState`を呼ばない。
そのため、テストのアプリのUIスレッドは、MTAになる。

MTAのスレッドでは、Windowsのクリップボードが使えない。
`Clipboard.SetContent`と`Clipboard.GetContent`は、`0x800401F0`（CoInitializeが呼ばれていない）で失敗する。
UIスレッドの上で`OleInitialize`を呼んでも、`0x80010106`（RPC_E_CHANGED_MODE）が返る。
スレッドのアパートメントは、作ったあとでは変えられないためである。

本物のアプリ（`PetaStickies`）は、`[STAThread]`の`Main`から動く。
本物のアプリでは、クリップボードは使える。
テストだけが、本物のアプリと違う条件で動いていた。

### 画面のテストがクリップボードを確かめられなかった

`StickyWindow`と`RowView`は、クリップボードの失敗を「ほかのアプリがクリップボードを持っている」として握りつぶす。
そのため、画面のテストでは、Ctrl+CもCtrl+Vも、何も起こさなかった。
コピーと貼り付けは、`PetaBorder.Tests`の`ShellTests`で、クリップボードを差し替えて確かめていた。
`ShellTests`は、キーからWindowsのクリップボードまでの道を通らない。

この件は、issue #827で調べた。

## Decision

`PetaScreens.Tests`は、`[WinUITestTarget]`を使わない。
テストのアプリは、テストのコードが自分で、STAのスレッドの上で起動する。

起動は、`TestApp.cs`の`TestAppStart`クラスにある`[AssemblyInitialize]`のメソッドで行う。
手順は次のとおりである。

1. `new Thread`でスレッドを作り、`SetApartmentState(ApartmentState.STA)`を呼ぶ。
   スレッドは`IsBackground = true`にする。
   `Application.Start`はテストのあいだ戻らないので、テストが終わったらプロセスを終えられるようにするためである。
2. そのスレッドの上で`Application.Start`を呼ぶ。
   コールバックの中で、`DispatcherQueueSynchronizationContext`を設定し、`TestApp`を作る。
3. そのスレッドの`DispatcherQueue`を、`UITestMethodAttribute.DispatcherQueue`に入れる。
   MSTestは、この値があれば、自分ではアプリを起動しない。
   `[UITestMethod]`は、このスレッドで動く。

この決定は、ADR 059の「アプリの起動は、別ホスト型（`[assembly: WinUITestTarget(typeof(...))]`）でMSTestに任せる」を置き換える。
ADR 059のほかの決定は変えない。
画面のコードを`PetaScreens`に置くこと、キーとクリックを`SendInput`で送ること、`PetaBorder.Tests`を残すことは、そのままである。

画面のテストは、Windowsの本物のクリップボードを使う。
Ctrl+C、Ctrl+V、Ctrl+Shift+V、親と子の貼り戻しは、`OutlineScreenTests`で確かめる。

## Alternatives

### Alternative 1: `[WinUITestTarget]`のままにし、クリップボードは`ShellTests`で確かめる

ADR 059の形を変えずに済む。
しかし、キーからWindowsのクリップボードまでの道を確かめられない。
実際、クリップボードを使う画面のテストを足したところで、2つの不具合が見つかった。
Ctrl+Shift+Vで、Windowsの改行（`\r\n`）の`\r`が項目に入っていた。
付箋紙を閉じるときにエディタがカーソルを受け取ると、片づけたコアを呼んで、アプリが落ちていた。
どちらも、`ShellTests`では見つからなかった。

### Alternative 2: アプリの中で、クリップボードの呼び出しだけをSTAのスレッドに回す

テストのアプリの起動は、MSTestに任せたままにできる。
しかし、テストのためだけに、本物のアプリのコードにスレッドの切り替えが入る。
本物のアプリはSTAで動くので、その切り替えは本物のアプリでは要らない。

### Alternative 3: MSTestの修正を待つ

起動のコードを、このリポジトリで持たずに済む。
しかし、いつ直るか分からない。
直るまで、クリップボードの画面のテストを書けない。

## Confidence Level

確信度は高い。
2026-11-19に、`PetaScreens.Tests`の27件が、STAのスレッドの上ですべて通った。

次のどちらかが起きたら、見直す。

- MSTestが、STAのスレッドでアプリを起動するようになった。
  そのときは、`[WinUITestTarget]`に戻して、起動のコードを消せる。
- MSTestから`UITestMethodAttribute.DispatcherQueue`がなくなった。
  そのときは、この起動の形が使えなくなる。

## Consequences

### 良い結果

- 画面のテストが、本物のアプリと同じSTAのスレッドで動く。
- 画面のテストで、Windowsの本物のクリップボードを使える。
  キーからクリップボードまでを、通して確かめられる。

### 悪い結果

- 起動のコード（約40行）を、MSTestではなく、このリポジトリで持つ。
  MSTestの起動の改善（起動の失敗を知らせる処理など）は、自動では入らない。
- 画面のテストは、開発者のクリップボードを書き換える。
  テストを回す前にクリップボードにあったものは、消える。

### 中立的な結果

- コピーと貼り付けの同じシナリオが、`ShellTests`と`OutlineScreenTests`の両方にある。
  `ShellTests`のほうを残すかは、別に決める。
