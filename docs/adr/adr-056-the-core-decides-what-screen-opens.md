# ADR 056: The Core Decides What Screen Opens

## Status

Accepted

## Context

AndroidとiOSのアプリは、Rustのコア（`client-shell-embedded`の`PetaCore`）で、画面を開く（ADR 029、ADR 031）。
しかし、どの画面を開くかの判断と、その結果の状態は、まだ両方のシェルの`Shell`（Androidは`Shell.kt`、iOSは`Shell.swift`）にある。
同じ判断を、KotlinとSwiftで2回書いている。

シェルの`Shell`が持つ状態は、次の3つである。

- `missing`：リンクが指すページがなく、何も開かなかったときの、そのページの名前。
  画面は、そのページを作るかを尋ねる（ADR 031）。
- `missingFile`：リンクが指す`assets`のファイルが、まだダウンロードされていないときの、そのパス。
- `unopened`：サインインの前で、今日のジャーナルを作らず、何も開いていないこと。
  最初のダウンロードが今日のジャーナルを持ってくるかもしれないためである。

シェルの`Shell`が持つ判断は、次のとおりである。

- `restore`：前回開いた画面（`PetaCore::last_screen`）を開く。
  それがないか、そのページが消えていれば、今日のジャーナルを作って開く。
  作らないように言われたら、何も開かず、`unopened`にする。
- `follow`はページを作らずに開き、`make`は`missing`のページを作って開き、`today`は今日のジャーナルを作って開く。
  どれも、`PetaCore::open`か`PetaCore::open_or_make`の答えを`missing`にする。
- `back`と`forward`は、`missing`と`missingFile`を消してから、コアにイベントを送る。
- コアが`Navigate`の要求を出したら、そのURLを作って開く。
- ダウンロードを取り込んだあと、`unopened`なら`restore`を呼ぶ（`syncAndWait`）。
- `image`と`attached`は、Markdownフォルダーの中のファイルを、大文字と小文字を区別せずに探す。
  `attached`は、見つからなければ`missingFile`にする。

ファイルの探し方は、Rustの同期とシェルで違う。
`client-cloud-sync`の`find_without_case`は、大文字と小文字に加えて、文字の正規化（NFCとNFD）も問わずに比べる（ADR 054の「パスの文字の正規化」）。
シェルの`findWithoutCase`は、大文字と小文字だけを問わない。
そのため、ファイル名の濁点が分かれた形（NFD）で書かれた画像は、シェルでは見つからないことがある。

## Decision

画面を開く判断と、その結果の状態を、`client-shell-embedded`の`PetaCore`に移す。
シェルは、ユーザーの操作を`PetaCore`に伝え、`PetaCore`が返す状態を画面に映すだけにする。

### 状態

`PetaCore`は、`missing`、`missing_file`、`unopened`を持つ。
これを、BoltFFIのレコード`PetaScreen`として、`PetaCore::screen`で返す。
3つは、`client-core`のビューモデル（`ViewModel`）には入れない。
`ViewModel`はWebのアウトライナーと共通で、Webには、このどれも要らないためである。

### 操作

`PetaCore`に、次の操作を足す。
どれも、操作のあとの`PetaScreen`を返す。

| 操作 | すること |
|---|---|
| `follow(url)` | ページを作らずに開く。ないページなら、何も開かず、`missing`にその名前を入れる |
| `make()` | `missing`のページを作って開く |
| `today()` | 今日のジャーナルを作って開く |
| `restore(make)` | 前回の画面を開く。開けず`make`なら、今日のジャーナルを作って開く。開けず`make`でなければ、`unopened`にする |
| `open_first_screen()` | `unopened`のときだけ、`restore(true)`をする。ダウンロードを取り込んだあとに、シェルが呼ぶ |
| `back()`、`forward()` | `missing`と`missing_file`を消し、コアの履歴をたどる。今日の日付は、Rustの`SystemClock`で決める |
| `navigated(url)` | コアの`Navigate`の要求に従い、ページを作って開く |
| `image(path)` | `assets`のファイルのパスを返す。ないときと、フォルダーのときは、何も返さない |
| `attached(path)` | `image`と同じものを返す。なければ、`missing_file`に`path`を入れる |

ページを開くと、`missing_file`は消え、`unopened`は解ける。
`image`と`attached`は、`client-cloud-sync`の`find_without_case`で探す。
そのため、NFDのファイル名の画像も見つかる。

今の`PetaCore::open`、`open_or_make`、`last_screen`は、シェルからは呼ばなくなるので、外に出さない。

### シェルに残るもの

- 状態を、画面が読める形（Composeの`mutableStateOf`、SwiftUIの`@Published`）で持つこと。
  シェルは、`PetaCore`の答えの`PetaScreen`を、そのまま写す。
- コアの`Navigate`の要求を受けて、`navigated`を呼ぶこと。
- ファイルを開くこと（Androidの`Intent`、iOSの`QuickLook`）。

### テスト

`follow`、`make`、`restore`などの判断を確かめるテストは、`client-shell-embedded`のRustのテストにする。
シェルには、`PetaScreen`が画面に届くことを確かめるテストを残す。

## Alternatives

### Alternative 1: 状態を`ViewModel`に入れる

`missing`、`missing_file`、`unopened`を、`client-core`の`ViewModel`のフィールドにする。
シェルは、ビューモデルを読むだけで、状態も読める。
しかし、`client-core`はWebのアウトライナーと共通で、Webには要らないフィールドが増える。
Webのアウトライナーは、ないページへのリンクに触れると、そのページを作って開く。
選ばなかった。

### Alternative 2: 判断だけを移し、状態はシェルに残す

`PetaCore`は、どのURLを開くかだけを返し、シェルが状態を持つ。
FFIの変更が小さい。
しかし、`missing`と`unopened`の更新の決まりが、両方のシェルに残る。
選ばなかった。

## Confidence Level

確信度は高い。

判断は、今の両方のシェルの振る舞いを、そのままRustに移す。
新しい振る舞いは、NFDのファイル名の画像が見つかることだけである。

次の状況になったら、この決定を見直す。

- Webのアウトライナーも、ないページへのリンクでページを作らないようにするとき。
  状態を`ViewModel`に入れる案（Alternative 1）が、両方に役立つ。

## Consequences

### 良い結果

- 画面を開く判断を、1回だけ書く。
- `findWithoutCase`がシェルから消え、ファイルの探し方が、Rustの同期と同じになる。
  NFDのファイル名の画像が、シェルでも見つかる。
- 判断のテストを、Rustで1回だけ書く。

### 悪い結果

- `PetaCore`の外に出す操作が9つ増える。
- `PetaScreen`を返すたびに、FFIを越えて3つの値を写す。

### 中立的な結果

- シェルの`Shell`の`follow`、`make`、`today`、`restore`、`back`、`forward`、`image`、`attached`は残る。
  中身は、`PetaCore`を呼び、`PetaScreen`を写すだけになる。
