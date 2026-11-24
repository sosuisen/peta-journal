# ADR 083: The Core Decides What a Click on a Link Does

## Status

Accepted

## Context

### どのシェルも、リンクのクリックを自分で判断している

項目の文字の中のリンクは、コアの`RichLink`（`client-core/src/rich_text/mod.rs`）で表される。
`RichLink::Page { url }`はアプリのページ、`RichLink::Web { url }`は外のアドレス、`RichLink::File { path }`は`assets`のファイルである。
シェルは、このリンクがクリックされたときに何をするかを、修飾キーを見て自分で決めている。

| シェル | ページへのリンク | 修飾キー | 外へのリンク、`assets`のファイル |
|---|---|---|---|
| Web（`outliner-react`の`OutlineItem.tsx`） | `core.go` | Ctrl/Commandはブラウザに任せて新しいタブ、Shiftは`window.open` | `window.open` |
| macOSの付箋紙（`Shell.follow`） | `open` | Commandは新しい付箋紙、Shiftはブラウザ、ほかのキーはこの付箋紙 | ブラウザ |
| Windowsの付箋紙（`Shell.Follow`） | `Open` | Controlは新しい付箋紙、ほかのキーはすべてブラウザ | ブラウザ |
| iOS、Android、Emacs | 開く | 見ない | ブラウザ |

判断がシェルにあるので、キーの意味がシェルごとに食い違っている。
macOSでShift以外のキーを押すと付箋紙で開くが、WindowsでShift以外のキーを押すとブラウザで開く。
`assets`のファイルのアドレスも、各シェルが`client-native-bridge`の`asset_address`などを呼んで作っている。

### コアには、判断の一部と命令がすでにある

#964で、ページを開く場所の判断の一部がコアに移った。
コアの`keys::Held`（`shift`、`ctrl`、`cmd`）の`open_in()`が、`keys::OpenIn`（`Here`、`Tab`、`Window`）を返す。
Webのサイドバーやカレンダーは、その値を`Event::GoToPage { path, open_in }`に入れて送る。
コアは、`Tab`か`Window`なら`UiOp::OpenWindow { url, new_window }`を命じ、`client-web-bridge`が`window.open`を呼ぶ。
ただし、項目の中のリンクは、この道を通っていない。
`OpenIn`はブラウザのタブとウィンドウだけで、付箋紙の「新しい付箋紙」がない。

`Event::LaunchUrl { url }`は、サーバーにURLをOSで開かせるイベントである。
Leptos版のWebが、独自のスキームのリンク（`obsidian://`など）に使っていた。
今は、どのシェルも送っていない。

### 見つかった経緯

ADR 064のS2を、macOSの付箋紙で試したときに見つかった（#1050、#1063）。
`LaunchUrl`がmacOSのソースに出てこない理由を調べると、`Shell.follow`が判断を持っていた。
ADR 064の言う、コアに任せるべき判断がシェルに残っている例である。

### イベントでもブラウザの動きを止められる

#964で値（`Held`と`OpenIn`）にした理由は、記録に残っていない。
考えられる理由は、ブラウザのCtrl+クリックの既定の動き（新しいタブ）を、クリックのその場で止めるかを決める必要があったことである。
しかし、Webのシェルは、クリックのハンドラーで`preventDefault()`を呼べば、いつでも既定の動きを止められる。
`outliner-react`のコアへの送信は同期である（`core.ts`の`send`は、`update`の答えをその場で受け取る）。
そのため、コアが`UiOp::OpenWindow`を命じると、`window.open`はクリックの処理の中で呼ばれ、ブラウザにブロックされない。
イベントにしても、Webでの問題はない。

## Decision

項目の文字の中のリンクのクリックは、すべてのシェルがコアにイベントで伝え、コアが開き方を決めて命じる。

### シェルが送るイベント

シェルは、リンクがクリックされると、`Event::LinkClicked { link: RichLink, held: Held }`を送る。
`held`は、クリックのときに押されていた修飾キーである。
シェルは、リンクの種類も修飾キーも見ずに、そのまま送る。
Webのシェルは、ブラウザの既定の動きを`preventDefault()`で止めてから送る。
中クリック（`auxclick`）も止め、Ctrlを押したクリックとして送る。
リンクの`href`は残すので、右クリックのメニューの「新しいタブで開く」は使える。

### コアが決める開き方

コアは、次の表で開き方を決める。

| リンク | 修飾キー | 開き方 |
|---|---|---|
| ページ | なし | この画面で開く |
| ページ | CtrlかCommand（Shiftなし） | アプリの新しい画面で開く |
| ページ | Shift | Web版のそのページを、ブラウザで開く |
| 外へのリンク | どれでも | ブラウザで開く |
| `assets`のファイル | どれでも | journalサーバーの`/assets/...`を、ブラウザで開く |

ShiftとCtrlが両方押されていれば、Shiftを採る。
Altなど、表にないキーは、押されていないものとして扱う。
これで、macOSとWindowsの付箋紙の食い違いがなくなる。
Windowsの付箋紙で、Altだけを押したクリックは、ブラウザでなく、この付箋紙で開くようになる。

### コアが命じること

コアは、開き方に応じて、次のどれかをする。

- この画面で開く：今の`Open`や`go`と同じく、ページを開き、履歴に残す（ADR 035）。
- アプリの新しい画面で開く：`UiOp::OpenNewView { url }`を命じる。Webのシェルは新しいタブで、付箋紙は新しい付箋紙で開く。
- ブラウザで開く：`UiOp::OpenInBrowser { address }`を命じる。Webのシェルは新しいウィンドウで、ネイティブのシェルはOSの既定のブラウザで開く。

`address`は、ブラウザがそのまま開ける完全なアドレスである。
ページと`assets`のファイルのアドレスには、journalサーバーのアドレスが要る。
コアはサーバーのアドレスを持たないので、ブリッジ（`client-native-bridge`、`client-web-bridge`）が、シェルに渡す前に完全なアドレスにする。
ブリッジには、`page_address`と`asset_address`がすでにある。

### 今ある道の置き換え

- `UiOp::OpenWindow`は、`OpenNewView`と`OpenInBrowser`に置き換える。
- `keys::OpenIn`は、`Here`、`NewView`、`Browser`に変える。`Held::open_in()`は、上の表のページの行に合わせる。
- `Event::GoToPage { path, open_in }`は残す。サイドバーやカレンダーのクリックは、これまでどおりこのイベントで送り、コアは同じ命令を出す。
- `EditorKey::OpenLink`からコアが作る`Event::OpenLinkAt`も、修飾キーなしのクリックと同じ表で開く。
- `Event::LaunchUrl`は消す。独自のスキームのリンクは、外へのリンクとして`OpenInBrowser`で開く。サーバーの`launch_url`のAPIは、この決定では変えない。

### 新しい画面を持たないシェル

iOS、Android、Emacsには、アプリの新しい画面がない。
これらのシェルは、修飾キーのない`Held`を送る。
そのため、ページへのリンクで`OpenNewView`が命じられることはない。
外へのリンクと`assets`のファイルでは、`OpenInBrowser`を受けて開く。

## Alternatives

### Alternative 1: コアに値を返す関数を置く

`keys`に、`RichLink`と`Held`から開き方を返す関数を置き、シェルはその答えに従って開く。
#964の`Held::open_in()`と同じ形である。
シェルの変更は小さい。
しかし、コアを通るのは関数の呼び出しだけで、イベントは通らない。
ADR 064のS2のようなチェックでは、判断がコアを通ったかが見えない。
また、答えを受けたあとの「どう開くか」の分岐が、各シェルに残る。
イベントでもWebのブラウザの動きを止められると分かったので、選ばなかった。

### Alternative 2: 今のまま、各シェルが判断する

変更はない。
しかし、キーの意味がシェルごとに食い違ったまま残る。
新しいシェルを作るたびに、同じ判断を書き直すことになる。

### Alternative 3: シェルが自分のできることをコアに伝える

起動のときに、シェルが「新しい画面を持つか」などをコアに伝え、コアはそれを見て命令を選ぶ。
新しい画面のないシェルでも、修飾キーを送れるようになる。
しかし、今そうしたいシェルはない。
修飾キーを送らないシェルには`OpenNewView`が来ないので、今は要らない。

## Confidence Level

中程度である。

イベントと命令の形は、Webで`GoToPage`と`OpenWindow`がすでに動いているので、確信がある。
キーの表は、macOSとWindowsの付箋紙の今の振る舞いと、Webの振る舞いから決めた。
macOSでは、Control+クリックはOSが右クリックとして扱うので、付箋紙には来ない。
Windowsでは、Shift以外のキーでブラウザで開いていた振る舞いがなくなる。
使ってみて、別のキーの意味が要ると分かったら、表を見直す。

ブリッジでアドレスを完全にする形は、まだ試していない。
ブリッジがシェルへの命令を書き換えるのが難しければ、コアがサーバーのアドレスを持つ形を考え直す。

## Consequences

### 良い結果

- リンクのクリックの判断が、コアの1か所になる。キーの意味が、すべてのシェルでそろう。
- ADR 064のS2で、`LinkClicked`が全シェルのソースに出てくるかを確かめられる。
- 付箋紙の`Shell.follow`と`Shell.Follow`の分岐と、そのテストが消える。判断のテストは、コアの1か所に書く。
- `assets`のファイルのアドレスを、シェルが作らなくなる。

### 悪い結果

- `Event`と`UiOp`が変わるので、BoltFFIとbincodeから生成される型が、すべてのシェルで変わる。全シェルを同時に直す必要がある。
- Webのシェルは、リンクのクリックのたびに既定の動きを止める。ブラウザに任せていたCtrl+クリックの新しいタブを、自分で`window.open`で開くことになる。
- iOSとAndroidのシェルも、外へのリンクを、自分で開く代わりに`OpenInBrowser`を受けて開くよう直す必要がある。

### 中立的な結果

- `features.md`の、付箋紙のリンクのクリックの行（macOSとWindows）を書き直す。
- `client/tools/shell-check/rules/stickies-mac.toml`は、`LaunchUrl`が消えるので、そのまま`missing`が出なくなる。`LinkClicked`は、macOSのソースに出てくる。
- 実装は#1063で行う。

## 追記: スマホのシェルとEmacsへの適用（2026-11-25）

iOS、Android、Emacsのシェルも、リンクのクリックを`LinkClicked`で送るようにした（#1063、#1090）。その中で、この決定になかった2つのことを決めた。

### ないページへのリンクは、スマホでもページを作る

ADR 031は、スマホでは、まだないページへのリンクでページを作らないと決めていた。触り間違いで、空のページがDropboxへ上がるのを防ぐためである。画面は「No page yet」と知らせ、「Make it」で作った。

この決定をやめる。ほかのシェル（Web、付箋紙、Emacs）は、ないページへのリンクで、そのページを作って開く。コアが`Navigate`で開かせ、ページを開くときにサーバーが作るためである。スマホだけが違う振る舞いを持つ理由はない。スマホのシェルも、コアの`Navigate`をそのまま受け、ないページを作って開く。「No page yet」と「Make it」はなくした。

### シェルは、コアの命令を自分の環境に合わせて読み替えてよい

コアは、`assets`のファイルを、journalサーバーの`/assets/...`のアドレスで`OpenInBrowser`に命じる。スマホのアプリにはサーバーがないので、このアドレスはブラウザで開けない。

シェルは、コアの命令の意図を変えない範囲で、自分の環境に合わせて読み替えてよい。スマホのシェルは、`/`で始まるアドレスの`OpenInBrowser`を、端末の中の`assets`のファイルを開くことと読み替える。アドレスの復号とファイルの探索は、`client-shell-embedded`の`file_at`が行う。

Emacsのシェルは、`/`で始まるアドレスの前に、Emacsが知るjournalサーバーのアドレスを付ける。ネイティブのブリッジ（`client-native-bridge`）がしていることを、Emacsでは`peta-journal.el`がする。

開き方の判断（どのリンクを、どのキーで、どう開くか）は、読み替えの対象にしない。判断はコアの1か所に置くという、この決定の目的を崩さないためである。
