# ADR 059: The Windows of the Windows Stickies Are Tested in a WinUI Test App

## Status

Accepted

## Context

Windowsの付箋紙（`stickies-win`）には、画面とウィンドウのテストがない。
テストは`PetaBorder.Tests`（xUnit）の66件で、`Shell`とコアの境界までを確かめる。
画面のコードは、アプリ本体の`PetaStickies`（exe）にある。
`RowView`、`StickyWindow`、`SearchBox`、`StickyDesk`、`Native.cs`の`Input`で、約2,400行である。
テストのプロジェクトは、このコードを参照していない。
そのため、キーを`PreviewKeyDown`で受けてコアに渡す処理、リンクのクリック、選択、検索窓、付箋紙の復元などは、手で確かめるしかない。

macOSの付箋紙（`stickies-mac`）は、画面を`PetaScreens`というライブラリに分けている。
テスト（130件）は、SwiftUIの画面を本物の`NSWindow`に置き、AppKitと同じ形でキーやクリックを渡す。
`OutlineScreenTests`の40件と`StickyDeskTests`の29件が、Windowsにはないテストである。

WinUI 3の画面は、XAMLのアプリのプロセスの中で、UIスレッドからしか作れない。
ふつうの`dotnet test`のプロジェクトでは動かない。
MicrosoftはMSTestの`[UITestMethod]`を用意している。
MSTest 4.5から、パッケージなしのWinUIアプリも、`dotnet test`で動かせる。

2026-11-18に、捨てる前提の試作で次のことを確かめた。

- `MSTest.Sdk` 4.5.3のテストアプリが、.NET 11、Windows App SDK 2.6.5、自己完結型の組み合わせで、`dotnet test`から動く。
- `[UITestMethod]`のテストが、ウィンドウに`TextBox`を置き、`Loaded`を`await`で待てる。
- テスト用の`Application`は、コードだけで書ける。
  ただし、`IXamlMetadataProvider`を実装し、`XamlControlsXamlMetaDataProvider`に任せる必要がある。
  任せないと、`XamlControlsResources`を読むところで`0xC000027B`で落ちる。
  `Resources`に触れるのは`OnLaunched`の中にする。
  コンストラクタで触れると、`0x8000FFFF`で落ちる。
- `KeyRoutedEventArgs`には公開のコンストラクタがない。
  テストからキーイベントを作って渡すことはできない。
- `SendInput`で送ったShift+Enterは、`TextBox`の`PreviewKeyDown`に届く。
  `InputKeyboardSource.GetKeyStateForCurrentThread`も、Shiftが押されていると答える。
  3回続けて流し、毎回通った。

## Decision

画面のコードを、WinUIのクラスライブラリ`PetaScreens`に移す。
移すのは`RowView`、`StickyWindow`、`SearchBox`、`StickyDesk`と、`Native.cs`の`Native`と`Input`である。
アプリ本体の`PetaStickies`には、`App.xaml`と起動の処理だけを残す。
名前はmacOSの`PetaScreens`に合わせる（ADR 040は`Peta`で始まる部品の名前を変えない）。

画面のテストは、新しいプロジェクト`PetaScreens.Tests`に書く。
`PetaScreens.Tests`は、`MSTest.Sdk`のパッケージなしのWinUIテストアプリである。
UIスレッドで動くテストには`[UITestMethod]`を付ける。
アプリの起動は、別ホスト型（`[assembly: WinUITestTarget(typeof(...))]`）でMSTestに任せる。
テスト用の`Application`はコードだけで書き、`IXamlMetadataProvider`を実装する。

`PetaBorder.Tests`（xUnit）は、そのまま残す。
テストのサーバーを起動する`JournalServer.cs`は、`PetaBorder.Tests`に置いたまま、`PetaScreens.Tests`からファイルのリンクで使う。

キーは、`SendInput`で本物のキーとしてOSから送る。
アプリと同じく、`PreviewKeyDown`から`Input.ChordOf`、`Shell.Pressed`までを通すためである。
テストはウィンドウを前面に出してから送る。

`PetaScreens`のクラスは`internal`のままにし、`InternalsVisibleTo`で`PetaScreens.Tests`にだけ見せる。
`PetaStickies`が起動に使う`StickyDesk`などだけを`public`にする。

`PetaScreens.Tests`はCIで動かさない。
`PetaBorder.Tests`と同じく、手元で`dotnet test`を流す。

## Alternatives

### Alternative 1: 1つのMSTestのWinUIテストアプリにまとめる

`PetaBorder.Tests`もMSTestに書き換え、画面のテストと1つのプロジェクトにする。
macOSと同じく、テストのプロジェクトが1つになる。
しかし、66件の書き換えが要る。
境界のテストまで、ウィンドウを開くアプリの中で動くことになる。
このため選ばなかった。

### Alternative 2: キーの処理を関数に分け、テストから呼ぶ

`PreviewKeyDown`の中身を「キーと押されている修飾キー」を受け取る関数に分け、テストはその関数を呼ぶ。
キーボードやマウスに触っても落ちず、画面がロックされていても動く。
しかし、`PreviewKeyDown`とのつなぎと、修飾キーの読み取りは確かめられない。
テストは手元で流すので、`SendInput`の制約は受け入れられる。
このため選ばなかった。

### Alternative 3: UI Automationで外から操作する

アプリのexeを起動し、FlaUIやAppiumで別のプロセスから操作する。
アプリのコードを動かさずに済む。
しかし、遅く、不安定になりやすい。
AppiumのWindowsドライバーは、開発が止まったWinAppDriverに頼っている。
このため選ばなかった。

### Alternative 4: 移したクラスをすべて`public`にする

`InternalsVisibleTo`が要らなくなる。
しかし、`PetaScreens`の外に見せる型が増える。
このため選ばなかった。

## Confidence Level

中程度である。
試作で、テストアプリの起動、UIスレッドでのテスト、`SendInput`のキーは確かめた。
本物の`StickyWindow`をテストの中で開くことと、クリックやドラッグを送ることは、まだ確かめていない。

次のときは見直す。

- `SendInput`のテストが、手元でもたびたび落ちるとき。
  Alternative 2に移る。
- xUnitがWinUIのUIスレッドでテストを動かせるようになったとき。
  テストのプロジェクトを1つにまとめられる。
- テストをCIで動かすことになったとき。
  CIのエージェントに、ログインした画面があるとは限らない。

## Consequences

- キー、リンク、選択、検索窓、付箋紙の復元など、今は手で確かめている画面の振る舞いを、テストで確かめられる。
- 画面のテストは、macOSの`OutlineScreenTests`と`StickyDeskTests`を手本に書ける。
- テストの道具が、xUnitとMSTestの2種類になる。
  `PetaBorder.Tests`と`PetaScreens.Tests`で、`Assert`の書き方が違う。
- 画面のテストを流す間は、ウィンドウが前面に出る。
  その間にキーボードやマウスを触ると、テストが落ちる。
  画面がロックされていると動かない。
- 画面のコードの場所が、`PetaStickies`から`PetaScreens`に変わる。
  READMEと`docs/architecture.md`の記述も合わせて直す。
