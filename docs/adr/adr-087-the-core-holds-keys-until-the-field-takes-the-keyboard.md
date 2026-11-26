# ADR 087: The Core Holds Keys Until the Field Takes the Keyboard

## Status

Accepted

## Context

Enterで項目を分けると、編集は新しい項目へ移る。
新しい項目の入力欄がキーボードを取るまでに、利用者は次の文字を打つことがある。
その文字は、まだキーボードを持っている古い欄に届く。
古い欄にそのまま入れると、文字は分けた前の項目に入ってしまう。

### コアの`KeyRelay`

コアには、この間の文字を預かる`KeyRelay`（`client-core/src/outline/mod.rs`）がある。
コアは、編集を移し始めたときに`hold_until_focus`で預かり始め、ビューモデルの`keyRelayHolding`を真にする。
シェルは、`keyRelayHolding`が真の間に打たれたキーを、`Event::KeyHeld`でコアに送る。
コアは、`focus_editor`（`client-core/src/ops/edit.rs`）で`UiOp::FocusEditor`を出すときに、預かったキーを`replay_keys`として渡し、預かるのをやめる。
シェルは、`replay_keys`を、カーソルを置いた欄に打ち直す。
Macの付箋紙（stickies-mac）とWebのアウトライナー（outliner-react）は、この形で使っている。

### iOSの預かり

iOSのアウトライナー（outliner-ios）では、SwiftUIが新しい行を描き、その入力欄がキーボードを取るまでに時間がかかる。
コアが`FocusEditor`を出した時点では、新しい欄はまだキーボードを持っていない。
コアはもう預かっていないので、この間の文字を`KeyHeld`で送っても捨てられる。
そこでiOSは、`KeyHeld`を使わず、`App/ItemEditor.swift`の`Typist.held`で文字を預かっている。
`Typist.held`は、`keyRelayHolding`が真の間に加えて、編集中の項目がその欄の項目でない間（`moving`）も預かる。
`KeyboardKeeper`が持っていた文字も、`Typist.held`に戻している。

同じ決まりがコアとiOSのシェルの2か所にあり、片方だけが変わると振る舞いが食い違う。
#1075でiOSにshell-checkのS2を入れたとき、iOSが`KeyHeld`を送っていないことから、この重なりが見つかった（#1163）。

## Decision

コアは、`FocusEditor`を出したあとも、シェルがその注文に答えるまでキーを預かり続ける。
シェルは、`FocusEditor`に、カーソルを置いた欄が実際にキーボードを取ったときに答える。

手順は次のとおりである。

1. コアは、編集を移し始めると`hold_until_focus`で預かり始める（今と同じ）。
2. コアは、`focus_editor`で、それまでに預かったキーを`replay_keys`に入れて`FocusEditor`を出す。
   このとき、預かるのはやめない。
   `keyRelayHolding`は真のままである。
3. シェルは、`keyRelayHolding`が真の間に打たれたキーを、`KeyHeld`でコアに送り続ける。
4. シェルは、`FocusEditor`の欄がキーボードを取ったときに、その注文に答える。
5. 答えが来たとき、その注文のあとに預かったキーがなければ、コアは預かるのをやめる。
   預かったキーがあれば、コアは同じ項目へもう一度`FocusEditor`を出し、そのキーを`replay_keys`で渡す。
   コアは、この注文の答えまで預かり続け、答えが来たら手順5をくり返す。

手順5で、キーを渡す注文のあとも預かり続ける理由は次のとおりである。
欄は、注文がシェルに届く前にキーボードを取ることがある。
iOSでは、注文の前の描き直しで、新しい欄がキーボードを取った。
その間に打たれたキーは、コアが注文を作ったあとに届き、2回目の注文で戻る。
2回目の注文のあとに預かるのをやめると、2回目の注文が届くまでに打たれたキーは、そのまま欄に入る。
そのキーが、預けたキーを追い越す（2026-11-26に、iOSの画面テストで見つかった。#1163）。

答えを待つ間に、利用者が次のEnterを押すことがある。
コアは、どの`FocusEditor`の答えかを見分け、古い注文の答えでは預かるのをやめない。
古い答えで止めると、あとの移動のために預かったキーが、前の項目へ出てしまうためである。

欄がいつまでもキーボードを取らないこともある。
たとえば、新しい項目が画面の外に出て、iOSが行を描かないときである。
コアは時間を区切らない。
編集が終わったとき、または次の移動が始まったときに、預かったキーの扱いが決まる。
時間で区切ると、遅い機械で打った文字が消えるためである。

シェルごとの答えの時期は次のとおりである。

- outliner-ios：新しい項目の欄がキーボードを取ったとき。
  `Typist.held`、`KeyboardKeeper`の預かり、`moving`の分岐を消し、預かる文字は`KeyHeld`でコアに送る。
- stickies-mac、stickies-win、outliner-android、outliner-react：今と同じく、描き直してカーソルを置いた直後。
  これらのシェルでは、描き直した時点で欄がフォーカスを持っているためである。

## Alternatives

### Alternative 1: コアの`KeyHeld`は移動中だけに使い、そのあとはiOSが預かる

iOSは、`keyRelayHolding`が真の間は`KeyHeld`でコアに送り、`FocusEditor`のあと欄がキーボードを取るまでは`Typist.held`で預かる。
コアは変わらない。
重なりは減るが、預かる決まりがコアとiOSの2か所に残る。
#1163の目的である、決まりを1か所にまとめることが果たせない。

### Alternative 2: 今のまま残す

iOSの預かりを、iOSのキーボードの事情によるものとして残し、shell-checkの除外の理由を書き換える。
変更はいちばん少ない。
2か所の決まりが食い違う危険は残る。

### Alternative 3: 時間で預かるのをやめる

`FocusEditor`のあと、数秒たっても答えがなければ、コアは預かるのをやめてキーを捨てる。
欄がキーボードを取らないときに、`keyRelayHolding`が真のまま残らない。
遅い機械や、重いページでは、利用者が打った文字が消える。
文字を失わないことを優先し、選ばなかった。

## Confidence Level

中程度である。
iOSで、欄がキーボードを取った時点をシェルが確実に知れることを前提にしている。
SwiftUIの入力欄が`becomeFirstResponder`を知らせない場面が見つかれば、答えの時期を見直す。
また、ほかのシェルで、描き直した直後に欄がフォーカスを持たない場面が見つかれば、そのシェルも答えの時期を遅らせる。

## Consequences

### 良い結果

- 移動の間のキーを預かる決まりが、コアの`KeyRelay`の1か所にまとまる。
- iOSの`ItemEditor.swift`から、`Typist.held`と`moving`の分岐がなくなり、短くなる。
- iOSが`KeyHeld`を送るようになり、shell-checkの`rules/outliner-ios.toml`から`KeyHeld`の除外を外せる。

### 悪い結果

- `keyRelayHolding`が真の間が、今より長くなる。
  `keyRelayHolding`を見て振る舞いを変えるシェルは、その間も古い欄のキーを`KeyHeld`で送る。
- `FocusEditor`の答えの意味が「描き直した」から「欄がキーボードを取った」に変わる。
  新しいシェルを作るときは、この意味で答える必要がある。

### 中立的な結果

- 同じ項目への`FocusEditor`が、続けて出ることがある。
  答えまでにキーを預かるたびに、もう1回出る。
  シェルがキーを受け取る道は`replay_keys`のままで、新しい`UiOp`は増えない。
- outliner-androidは、まだキーを預かっていない（#1146）。
  Androidが預かるときは、この決まりに従って`KeyHeld`を送る。
