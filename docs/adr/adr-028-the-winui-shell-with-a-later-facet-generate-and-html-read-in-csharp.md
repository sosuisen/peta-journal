# ADR 028: The WinUI Shell Uses a Later facet_generate and Reads the HTML of Items in C#

## Status

Accepted

## Context

SwiftUIのシェル（`frontend-swiftui`、アプリ名はPetaStickies）と同じ機能のアプリを、Windows向けにWinUIで作る。置き場所は`frontend-winui`である。作り方もSwiftUIのシェルと同じにする。つまり、Cruxのコアを`frontend-rust/crates/shell-native`で包み、境界をbincodeのバイト列で越え（ADR 024）、項目は`ast_to_html`のHTMLから描く（ADR 025）。

SwiftUIのシェルは、2つの生成物から作られている。1つは、境界を越える型である。Cruxが持ち込む`facet_generate`が、Rustの型からSwiftの型を作る。もう1つは、コアを呼ぶ入口である。BoltFFIが、`ffi.rs`からライブラリとSwiftのコードを作る。

WinUIのシェルで同じ作り方をするには、3つのことを決める必要があった。2026-11-11に、それぞれを確かめた。

第一に、C#の型の生成である。Crux 0.20.0が持ち込む`facet_generate`は0.19.1である。この版が作るC#は、このプロジェクトの型ではコンパイルが通らなかった。エラーは2種類である。

- 名前の衝突が9か所あった。バリアントの中のフィールド名が、バリアントの名前と同じときに起きる（CS8866、CS8910）。
- `Option<enum>`の読み書きが4か所で通らなかった（CS1503、CS0452）。

どちらも、上流で直っている（`facet_generate`のissue #131、#174、#193）。0.22.3で試すと、後処理なしでコンパイルが通った。生成される型は77個で、Rustの型の数と一致した。0.19.1と0.22.3は、どちらも`facet =0.46.5`を使う。同じクレートに両方を入れても、ぶつからない。

第二に、BoltFFIである。BoltFFI 0.32.1は、C#の出力を持つ（`boltffi pack csharp`）。動的ライブラリ（`cdylib`）が要る。`win-x64`のDLLと、それを呼ぶC#のプロジェクトができる。

第三に、HTMLの描き方である。ADR 025は、WinUIのシェルがHTMLを何で描くかを決めていない。macOSには、HTMLを装飾付きのテキストにする機能がある。Windowsには、同じ機能がない。WinUIの`RichTextBlock`は、装飾付きのテキストを描けるが、HTMLは読めない。

ギアは5速である。ウィンドウの見た目と操作は、SwiftUIのシェルにそろえる。

## Decision

WinUIのシェルを、`frontend-winui`に作る。アプリ名は、SwiftUIのシェルと同じPetaStickiesである。

### 型の生成

C#の型は、`facet_generate` 0.22.3を直接呼んで作る。Cruxが持ち込む0.19.1は、C#には使わない。`shell-native`の`Cargo.toml`に、`facet_generate = "=0.22.3"`を足す。`facet_typegen`フィーチャーのときだけ入る。

`codegen.rs`の`write_csharp_types`が、境界を越える型を自分で登録し、0.22.3のC#の出力を呼ぶ。Swiftの型は、今までどおりCruxの`TypeRegistry`を通して作る。

生成されたC#に、後処理はしない。コアの型の名前も、生成のためには変えない。型がRustと一致することは、生成器が保証する。後処理や名前の変更を挟むと、その保証が弱くなる。

Cruxが`facet_generate`を0.22以降に上げたら、C#もCruxの`TypeRegistry`を通す形に戻す。

### コアの入口

BoltFFIのC#の出力を使う。`boltffi.toml`に`[targets.csharp]`を足す。`shell-native`のクレートの種類に`cdylib`を足す。作るのは`win-x64`だけである。

`frontend-winui/generate.ps1`が、型の生成と`boltffi pack csharp --release`を続けて行う。生成物は、リポジトリに入れない。

### HTMLの描き方

項目のHTMLは、C#で読む。HTMLの解析には、AngleSharpを使う。読んだ結果は、段落と、スタイルを持つ文字のかたまりの並びである（`ShownText`）。画面は、これを`RichTextBlock`に描く。WebView2は使わない。

知らないタグは、中身の文字を出す。書かれた内容が、もれなく何らかの形で出ることを守る（ADR 025）。本文の中のチェックボックス、区切り線、画像は、文字に置き換える。SwiftUIのシェルと同じである。

`ShownText`は、WinUIに依存しない。テストは、ウィンドウなしで動く。

### 分け方

SwiftUIのシェルと同じに分ける。

- `PetaBorder`は、手書きのC#のライブラリである。WinUIに依存しない。`Border`、`Shell`、キーの意味、`ShownText`、付箋紙の保存を持つ。`dotnet test`でテストする。
- `PetaStickies`は、WinUIのアプリである。ウィンドウとビューだけを持つ。アウトラインについては何も決めず、`Shell`のビューモデルを描く。

アプリは、パッケージにしない。Windows App SDKは、`.exe`の横に置く。`dotnet build`でビルドでき、Visual Studioは要らない。

### SwiftUIのシェルと違うところ

Windowsに合わせて、次の点を変える。

- キーは、Webのフロントエンドと同じである。Commandの代わりに、Ctrlを使う。
- Ctrl+Qで、付箋紙を残したままアプリを終える。macOSのCommand+Qに当たる。Windowsには、アプリを終える共通の操作がないためである。閉じた付箋紙は戻らない、という決まりは変えない。
- 付箋紙は、最大化できない。タイトルバーのダブルクリックを、巻き上げに使うためである。
- 外のリンクは、`http`、`https`、`mailto`だけを開く。

## Alternatives

### Alternative 1: 生成されたC#を、後処理で直す

0.19.1が作るC#を、`codegen.rs`の中で文字列として書き換え、コンパイルが通るようにする。

長所は、依存が増えないことである。生成器の版は、Cruxと同じ1つのままである。

短所は、型の保証が弱くなることである。書き換えは、生成器の出力の形に頼る。型が増えると、新しい衝突が出て、書き換えを足すことになる。書き換えを誤ると、コンパイルは通り、バイト列が合わなくなる。

一度はこの形で実装した。上流で直っていると分かったので、やめた。

### Alternative 2: コアの型の名前を変える

衝突するフィールドの名前を、Rustの側で変える。

長所は、名前の衝突がなくなることである。

短所は2つある。`Option<enum>`のエラーは、名前を変えても直らない。また、C#の生成器の都合で、コアの名前が決まることになる。Web、Emacs、SwiftUIのシェルにも、変更が及ぶ。

問題の半分しか直らないので、選ばなかった。

### Alternative 3: C#の型を手で書く

境界を越える型と、bincodeの読み書きを、C#で手書きする。

長所は、生成器に頼らないことである。

短所は、型が77個あることである。Rustの型を変えるたびに、C#も手で直す。直し忘れると、コンパイルは通り、バイト列が合わなくなる。型を生成するというADR 024の狙いに反する。

選ばなかった。

### Alternative 4: HTMLをWebView2で描く

項目のHTMLを、WebView2で見せる。

長所は、どのHTMLも、そのまま出ることである。表も、箱の装飾も出る。

短所は、ADR 025のAlternative 3と同じである。項目ごとにWebViewを置くと重い。ページ全体を1つにすると、Webを埋め込む形に戻る。編集欄はネイティブなので、表示と編集のつなぎも難しい。

選ばなかった。

### Alternative 5: ASTをC#で直接読んで描く

ADR 025のAlternative 1と同じである。「どの種類も出る」という保証を、C#の側で作り直すことになる。ADR 025に従い、選ばなかった。

## Confidence Level

確信度は、中程度である。

確信が高いのは、型の生成である。0.22.3のC#は、後処理なしでコンパイルが通った。Rustが書いたバイト列をC#で読み、C#が書いたバイト列をRustで読むテストが通っている。本物のジャーナルサーバーに対して`Shell`を動かすテストも通っている。

確信が中程度なのは、HTMLをC#で読むことである。`ast_to_html`が出すHTMLの形は決まっているので、読むコードは小さい。ただし、タグごとのスタイルは、C#で書いた。HTMLブロックには、書かれたHTMLがそのまま入る。そこで見た目が崩れるものが、あるかもしれない。中身の文字は、どのタグでも出る。

確信が低いのは、ウィンドウである。ウィンドウのテストは、まだない。手で確かめたのは、一時的なサーバーでの基本の操作だけである。入力メソッドでの日本語の入力は、確かめていない。

次のときに、この決定を見直す。

- Cruxが`facet_generate`を0.22以降に上げたとき。C#の生成を、Cruxの`TypeRegistry`を通す形に戻す。
- `facet_generate`の2つの版が、`facet`の版でぶつかるようになったとき。
- SwiftUIのシェルが、macOSの変換をやめて、HTMLを自分で読むことにしたとき（ADR 025のAlternative 2）。HTMLを読むコードを、Rustの側に1つだけ置けないかを考える。
- `RichTextBlock`での描画が、長いページで遅いと感じられたとき。

## Consequences

### 良い結果

WindowsとmacOSで、同じアプリが動く。コア、境界、HTMLを作る関数は、同じものを使う。

C#の型は、Rustの型から作られる。後処理がないので、型の一致は生成器が保証する。

HTMLを読むコードは、WinUIに依存しない。ウィンドウなしでテストできる。Windowsの変換の速さや安定さに、左右されない。SwiftUIのシェルと違い、変換を待つ仕組みが要らない。

アプリは`dotnet build`だけでビルドできる。

### 悪い結果

`shell-native`が、`facet_generate`の2つの版に依存する。`facet_typegen`フィーチャーのビルドが、その分だけ長くなる。Cruxが版を上げるまで、この状態が続く。

HTMLの見た目を決めるコードが、SwiftUIのシェル（CSS）とWinUIのシェル（C#）に分かれる。見た目は、シェルごとに違ってよいと決めている（ADR 025）が、直すところは2つになる。

ウィンドウのテストがない。SwiftUIのシェルにある、画面をウィンドウに置くテストに当たるものを、まだ書いていない。

`shell-native`のクレートの種類に`cdylib`が加わる。macOS向けのビルドでも、使わない動的ライブラリが作られる。

### 中立的な結果

BoltFFIの版は、SwiftUIのシェルと同じ0.32.1である。`boltffi.toml`に、C#の出力先が加わる。

`frontend-swiftui/generate.sh`は、`codegen swift <folder>`を呼ぶ形に変わる。`codegen`が、作る言語を引数で受けるようになったためである。

付箋紙の保存先は、`%APPDATA%\PetaStickies\stickies.json`である。形は、SwiftUIのシェルのファイルと同じではない。位置の数え方が、OSによって違うためである。
