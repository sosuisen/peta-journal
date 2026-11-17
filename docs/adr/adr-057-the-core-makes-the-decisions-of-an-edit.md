# ADR 057: The Core Makes the Decisions of an Edit

## Status

Accepted

## Context

`client-core`（Crux）の上で、6つのシェルが動く。
Webのアウトライナー（`web-app/outliner`）、Emacs版（`emacs/outliner`）、Android、iOS、macOSの付箋紙（`stickies-mac`）、Windowsの付箋紙（`stickies-win`）である。
コードを共通にする目的は、シェルごとに書いている判断とそのテストを減らすことである。
ただし、性能が大きく落ちるか、アーキテクチャが歪むなら、共通にしない。

6つのシェルは、項目の見せ方で2つに分かれる。

- リッチテキストのシェル：Web版はASTからHTMLを作り（ADR 041）、ほかの4つは`RichText`を描く（ADR 042）。
  項目は飾りを付けて見せ、触れると、その項目だけがMarkdownの編集欄に切り替わる。
- Emacs版：いつもMarkdownのまま見せる。
  Emacsのポイントが、そのままMarkdownの位置である。

項目を編集するときの判断のうち、次の4つが、シェルに分かれて書かれている。

### 1. 括弧を閉じる条件

開き括弧を打つと、閉じる側を足す（ADR 050の`auto_close`）。
閉じる処理は`client_core::util::auto_close`の1つだが、いつ呼ぶかはシェルが決めている。

- ネイティブの4つ：本文が前より長くなったときに、`closed_as_typed`（`client-native-bridge`）を呼び、その結果を`BodyChanged`で送る。
  FFIを2回越える。
- Web版：ブラウザの`input`イベントの`inputType`が`insertText`のときに、ページの中で`auto_close`を呼ぶ（`item_editor/text.rs`の`close_typed`）。
- Emacs版：`post-self-insert-hook`で、`[`、`]`、`` ` ``、`~`、`>`を打ったときと、`<`で始まる行のときだけ、`typed`を送って返事を待つ。
  `outliner-emacs`が`auto_close`を呼ぶ。
  キーごとに返事を待つと打つのが遅くなるため、絞っている（`peta-journal--close-typed`）。

Androidの文字欄（Composeの`onValueChange`）は、変わったあとの文字だけを知らせる。
そのため、ネイティブのシェルは、打ったのか貼り付けたのかを区別できない。

### 2. 編集を始めるイベント

項目に触れて編集を始めるイベントは、3つある。

| イベント | 送るシェル | 位置の渡し方 |
|---|---|---|
| `EditRequested { cursor }` | Emacs版。リッチテキストのシェルでは、見えている文字の位置が決まらないとき | Markdownの位置 |
| `ShownTextTouched { letter }` | ネイティブの4つ | 見えている文字の何文字目か |
| `ShownTextTouchedAfter { before }` | Web版 | 触れた位置より前の、見えている文字 |

リッチテキストのシェルは、見えている文字の位置が決まらないときに、`EditRequested { cursor: 0 }`を送る。
コアは、カーソルを先頭に置く。
位置が決まらない場面と、そのあとのカーソルは、シェルごとに違う。

| シェル | 位置が決まらない場面 | カーソル |
|---|---|---|
| AndroidとiOS | 空の項目に触れたとき、画像だけの項目に触れたとき、選んだ項目でツールバーの「キーボードを出す」を押したとき。文字のない所に触れても、いちばん近い文字の位置にするので、この場面にはならない（`Touches.letter`） | 末尾。シェルがフォーカスの要求を`touched`に覚え、画面が末尾に置き直す |
| Web版 | ブラウザが、クリックした点に文字の位置を返さないとき（`dom::text_before_point`が`None`）。項目の余白、空の項目、画像だけの項目などである | 先頭 |
| macOSとWindowsの付箋紙 | Markdownの編集欄が、クリックやコアの指示でカーソルを得たとき（`gotCursor`、`GotCursor`） | 編集欄がすでに置いた所。シェルは、コアの先頭への要求を無視する（`cursorRequest`） |

Emacs版は、カーソルがいつもMarkdownの中にあるので、位置が決まらない場面はない。
Emacs版は、行の先頭にいるときに、本当の位置として`cursor: 0`を送る。
空の項目では、先頭と末尾は同じである。
差が出るのは、画像だけの項目（本文は`![](...)`）と、「キーボードを出す」で書き足すときである。

### 3. 前の編集を終える

別の項目に移ったときに前の項目の編集を終える道が、コアの中に2本ある。

- `edit_requested`：前の項目に打った文字があれば保存する（handover）。
  保存が非同期で走る間、`EditFinished`を無視する（`edit_handovers`）。
  それ以外の後始末はしない。
- `finish_edit`：保存に加えて、Wikiリンクの候補を閉じ、閉じていないブロックのある本文を保存し、「編集中」の印（`editing_item_ids`）を外す。

どちらを通るかは、シェルが送る順で決まる。
Android、iOS、Emacs版は、`EditRequested`の前に`EditFinished`を送る。
Web版と2つの付箋紙は、編集欄からフォーカスが外れたときに`EditFinished`を送る。
「編集中」の印を外すのは、シェルの`EditFinished`だけである。
印が残ると、同期で届いたページの読み直しが止まる。

### 4. 選択中に項目に触れたとき

コアは、選択中の`EditRequested`を無視する。

- Web版：マウスを押すと`EditRequested`を送るが、コアが無視する。
  続くクリックで`SelectionClickOutside`を送り、選択が解けるだけである。
  編集を始めるには、もう一度クリックする。
- AndroidとiOS：シェルが選択を解き（`unselect`）、続けて編集を始める。
- macOSとWindowsの付箋紙：編集欄がカーソルを得ると、シェルが選択を解き（`leaveSelection`、`LeaveSelection`）、続けて編集を始める。
- ネイティブの4つは、コアを2回呼び、画面を2回描く。
- Emacs版：選択はキーで操作する。
  選択中は、ポイントが動いても編集を始めない。

## Decision

4つの判断を、`client-core`に置く。
シェルは、何が起きたかをコアに知らせ、コアが返したビューモデルを映すだけにする。
置き場所は、ネイティブ用の層（`client-native-bridge`）ではなく`client-core`にする。
Web版とEmacs版も、同じ判断を使うためである。

### 編集を始めるイベント

`EditRequested`、`ShownTextTouched`、`ShownTextTouchedAfter`を、1つの`EditRequested`にまとめる。
位置は、種類で渡す。

```rust
EditRequested { item_id: String, place: EditPlace }

enum EditPlace {
    /// The place in the Markdown itself (Emacs, the Markdown editor of a sticky)
    Markdown(usize),
    /// The letters of the shown text before the touch (Android, iOS, macOS, Windows)
    ShownLetter(usize),
    /// The shown text before the touch (web)
    AfterShownText(String),
    /// No place in the text is known
    Unplaced,
}
```

- `Unplaced`は、シェルが本文の中の位置を知らずに編集を始めることを表す。
  空の項目、画像だけの項目、「キーボードを出す」、Webで文字の位置が返らないクリックである。
  コアは、カーソルを本文の末尾に置く。
  書き足すときに、続けて打てるためである。
- AndroidとiOSの`touched`は、なくなる。
  シェルは、コアのフォーカスの要求どおりにカーソルを置く。
- macOSとWindowsの付箋紙は、編集欄がカーソルを得たときに、そのカーソルの位置を`Markdown`で送る。
  コアの要求が、編集欄のカーソルと同じ所になる。
  そのため、コアの要求を無視する処理（`cursorRequest`）はなくなる。
- Emacs版は、JSONの`edit_requested`を変えない。
  `outliner-emacs`が、`EditPlace::Markdown(cursor)`に直して送る。
  `Markdown(0)`は、本当の先頭である。
- Web版は、文字の位置が返らないクリックで`Unplaced`を送る。
  項目の余白や画像だけの項目をクリックしたときのカーソルは、今の先頭から末尾に変わる。

### 前の編集を終える

`EditRequested`を受けたコアは、ほかに「編集中」の項目があれば、`finish_edit`と同じ手順で終える。
Wikiリンクの候補を閉じ、打った文字や閉じていないブロックのある本文を保存し、「編集中」の印を外す。
保存が非同期で走る間に届く`EditFinished`は、今と同じくhandoverで無視する。
保存のあとに届く`EditFinished`は、その項目がもう編集中でないので、何もしない。

- Android、iOS、EmacsのLispは、`EditRequested`の前に`EditFinished`を送らなくなる。
- Web版と2つの付箋紙は、フォーカスが外れたときの`EditFinished`を残す。
  ほかの項目に移らずに編集を終える場合（ページの外のクリックなど）に要るためである。

### 選択中に項目に触れたとき

選択中に`EditRequested`を受けたコアは、選択を解き、その項目の編集を始める。
1回の更新で行う。

- AndroidとiOSの`unselect`と、付箋紙の`leaveSelection`の呼び出しは、なくなる。
- Web版では、1回のクリックで編集に入れるようになる。
  クリックに続いて届く`SelectionClickOutside`は、もう選択中でないので何もしない。
- Emacs版は、選択中に`edit_requested`を送らないので、変わらない。

### 括弧を閉じる

文字を打ったことを知らせるイベントを足す。

```rust
Typed { item_id: String, body: String, cursor: usize, now_ms: u64 }
```

`cursor`は、ほかのイベントと同じく文字の数で数える。
コアは、コアが持つその項目の本文より`body`が短ければ、閉じない。
文字を消して開き括弧の後ろに戻ったときに、閉じる側が消すたびに戻らないためである。
短くなければ、`auto_close`で閉じる側を足す。
足したら、その本文を`BodyChanged`と同じく保存の予定に入れ、カーソルの位置をフォーカスの要求で返す。
シェルは、返った本文とカーソルを編集欄に入れる。

| シェル | `Typed`を送るとき |
|---|---|
| ネイティブの4つ | 本文が変わるたびに、`BodyChanged`の代わりに送る |
| Web版 | `inputType`が`insertText`のときに、`BodyChanged`の代わりに送る |
| Emacs版 | 今の`typed`と同じく、絞った文字のときだけ送る。`body_changed`はそのまま送る |

Emacs版では、`body_changed`が先に届くので、`Typed`の`body`はコアの本文と同じ長さになり、閉じる。
`closed_as_typed`（`client-native-bridge`、`client-shell-embedded`、`client-shell-http`）は、なくなる。

閉じる処理は、`BodyChanged`には入れない。
Emacs版は、`body_changed`を返事を待たずに送っている。
閉じた本文が遅れて届くと、そのあとに打った文字とぶつかる。

### テスト

4つの判断を確かめるテストは、`client-core`のテストにする。
シェルには、イベントが届き、返ったビューモデルが画面に出ることを確かめるテストを、判断ごとに多くても1つ残す。

## Alternatives

### Alternative 1: ネイティブ用の層（`client-native-bridge`）に置く

`NativeShell`に`edit`と`typed`を足し、ネイティブの4つだけで判断を共通にする。
Web版とEmacs版は、今のままである。
`client-core`のイベントを変えないので、変更は小さい。
しかし、判断がネイティブ用の層とWeb版、Emacs版の3か所に残る。
前の編集を終える道が2本あることも、残る。
選ばなかった。

### Alternative 2: 括弧を`BodyChanged`の中で閉じる

イベントを足さずに、`BodyChanged`を受けたコアが閉じる。
しかし、Emacs版で、閉じた本文が遅れて届き、そのあとに打った文字とぶつかる。
避けるには、Emacs版がキーごとに返事を待つ必要があり、打つのが遅くなる。
選ばなかった。

### Alternative 3: 位置のない編集の始まりを、別のイベントにする

`EditRequested { cursor }`を残し、リッチテキストのシェルだけが送る`UnplacedEditRequested`を足す。
しかし、編集を始めるイベントが4つになり、どのシェルがどれを送るかの知識が増える。
選ばなかった。

### Alternative 4: 選択中に触れたときの振る舞いを、シェルに残す

Web版は選択を解くだけ、ネイティブの4つは解いて編集する、という今の振る舞いを残す。
ネイティブの4つの`unselect`と`leaveSelection`も残る。
判断が一か所にならない。
選ばなかった。

## Confidence Level

確信度は中程度から高い。

括弧と編集を始めるイベントは、今の判断をそのまま移すので、確信度は高い。
前の編集を終える手順と選択中のクリックは、Web版の編集の流れ（handover、フォーカスの順序、選択のドラッグ）に関わる。
E2Eテストで確かめる前提で、確信度は中程度である。

次の状況になったら、この決定を見直す。

- Web版で、選択中にマウスを押して動かす操作（選択を広げるドラッグ）と、1回のクリックで編集に入る振る舞いがぶつかるとき。
- ネイティブのシェルで、打ったのか貼り付けたのかを区別できるようになり、貼り付けた括弧を閉じないようにしたいとき。
- `Typed`の往復が、ネイティブのシェルの打つ速さに目に見えて響くとき。

## Consequences

### 良い結果

- 編集の判断を、`client-core`に1回だけ書く。
  6つのシェルで同じ振る舞いになる。
- 「編集中」の印を、コアが外す。
  シェルが`EditFinished`を送り忘れても、印が残らず、ページの読み直しが止まらない。
- 編集を始めるイベントが1つになる。
  AndroidとiOSの`touched`、付箋紙の`cursorRequest`、ネイティブの4つの`unselect`と`leaveSelection`の呼び出し、`closed_as_typed`がなくなる。
- シェルのテストが減る。
  括弧の2件（iOS、macOS、Windows、Androidの各2件）と、触れた位置、`touched`、選択中のタップのテストが、`client-core`のテストに移る。
- ネイティブのシェルで、文字を打つたびにFFIを越える回数が、2回から1回になる。
  選択中に触れたときに、コアを呼ぶ回数と画面を描く回数も、2回から1回になる。
- Web版で、選択中の項目を1回のクリックで編集できる。

### 悪い結果

- `client-core`のイベントが変わるので、6つのシェルをすべて直す。
  Windowsの付箋紙は、macOSではビルドできない。
- Web版の振る舞いが2つ変わる。
  項目の余白や画像だけの項目をクリックするとカーソルが末尾に行くことと、選択中のクリックで編集に入ることである。
  WebのE2Eテストを直し、すべて回して確かめる。
- `edit_requested`が、`finish_edit`の手順も持つので、少し大きくなる。

### 中立的な結果

- 貼り付けた括弧は、Web版とEmacs版では閉じず、ネイティブの4つでは閉じる。
  OSの文字欄の知らせ方の違いで、今と同じである。
- Emacs版のLispは、`EditFinished`を先に送る処理が消えるだけで、JSONは変わらない。
- Web版と2つの付箋紙の、フォーカスが外れたときの`EditFinished`は残る。
