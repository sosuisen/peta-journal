# ADR 054: The Sync with Dropbox Is Written Once in Rust

## Status

Accepted

## Context

AndroidとiOSのシェルは、アプリの中のMarkdownフォルダーを、Dropboxのフォルダーと同期する（ADR 030、ADR 031）。
ADR 034は、この同期をAndroidのKotlinのクラスから、同じ名前と同じ分け方でSwiftに書き直すと決めた。
その結果、同じクラスがKotlinとSwiftに1つずつある。
`Cloud`、`CloudRecord`、`MarkdownFolder`、`Downloader`、`Uploader`、`SyncRound`、`CloudSync`と、Dropboxの形に直す`DropboxCloud`と`ContentHash`である。
2026-11-17に数えると、この分のコードがAndroidに約640行、iOSに約660行ある。
テストは、Androidに約1,200行、iOSに約1,100行ある。

同期の決まりを変えるときは、両方を直す必要がある。
ADR 034は、この短所を悪い結果として挙げ、KotlinとSwiftの同期が食い違い始めたら同期をRustに移す（Alternative 2）としていた。
食い違いは、すでに2つある。

- フォルダーの中のファイルを、大文字と小文字を区別せずに探す関数`findWithoutCase`が違う。Swiftの`split("/")`は空の区切りを捨て、Kotlinの`split('/')`は残す。先頭が`/`のパスは、iOSでしか見つからない。
- アプリが知っているDropboxの版の記録（`dropbox-record`）の形式が違う。AndroidはJavaの`Properties`で、iOSはJSONで書く。

2026-11-14に、4つのネイティブのシェルで重複したコードを調べた（`docs/plans.md`の「リファクタ：重複したコードをまとめる」）。
シェルのコードがいちばん減るのは、この同期をまとめる項目だった。

同期のクラスのうち、Dropboxに触れるのは`DropboxFiles`の実装（Androidは`SdkDropboxFiles.kt`、iOSは`SdkDropboxFiles.swift`）だけである。
この層は、DropboxのSDKの`list_folder`、`list_folder/continue`、`download`、`upload`を呼び、答えをSDKの型から`Listing`と`Entry`に直す。
残りのクラスは、`Cloud`のインタフェースを通してDropboxを見る。
サインインとアクセストークンの保存と更新は、各OSのSDKがする（ADR 033、ADR 034）。

AndroidとiOSのアプリは、すでに同じRustのクレート`client-shell-embedded`を使う。
このクレートは、Cruxのコアとjournal-coreをまとめ、BoltFFIでKotlinとSwiftの入口を作る。
BoltFFIは、Rustのトレイトを外の言語に実装させることができる。
`client-shell-embedded`の`PetaShell`が、その例である。

## Decision

同期の決まりを、Rustの新しいクレート`client-cloud-sync`に1回だけ書く。
AndroidとiOSは、`client-shell-embedded`を通して、このクレートを使う。
DropboxのSDKを呼ぶ層、サインイン、画面は、シェルに残す。

### Rustに書くもの

`client-cloud-sync`には、今の次のクラスが持つ決まりを移す。

- `CloudRecord`：アプリが知っているDropboxの各ファイルの版（`rev`）とcontent hashの記録
- `MarkdownFolder`：アプリのMarkdownフォルダーへの書き込みと、大文字と小文字を区別しない探し方
- `Downloader`：Dropboxの変更を、アプリのMarkdownフォルダーへ持ってくる手順
- `Uploader`：アプリで変わったファイルを、Dropboxへ上げる手順
- `SyncRound`：ダウンロード、コアへの取り込み、アップロードの順と、Dropboxがアップロードを断ったときのやり直し
- `CloudSync`：同期を1つずつ走らせることと、同期の状態（`SignedOut`、`Idle`、`Downloading`、`Uploading`、`Failed`）
- `DropboxCloud`と`ContentHash`：Dropboxの答えを`Change`に直すこと、カーソルを忘れられたときの読み直し、Dropboxのcontent hashの計算

決まりは、ADR 030とADR 031のままである。
同期で何をダウンロードし、何を消し、何をアップロードするかは変えない。

### シェルに残すもの

シェルは、BoltFFIで外に出したトレイト`DropboxFiles`を実装する。
メソッドは、今の`DropboxFiles`と同じく4つである。

1. `list(folder)`：フォルダーの中身の最初の部分を返す。
2. `list_after(cursor)`：カーソルの次の部分を返す。
3. `download(rev, to)`：ある版のファイルを、Rustが指定したパスに書く。
4. `upload(path, from, over, modified)`：Rustが指定したパスのファイルを、Dropboxに上げる。

ファイルの中身は境界を越えず、パスだけが越える。
大きな画像を、メモリーに読まずに済むためである。

失敗は、種類を分けてRustへ返す。
種類は、Dropboxがカーソルを忘れた（`CursorForgotten`）、Dropboxに別の版がある（`UploadConflict`）、通信ができない（`Offline`）、そのほか（`Other`）の4つである。
通信ができないかどうかは、シェルがSDKの例外から判定する。
Androidは、例外とその原因の中に`IOException`があるかで見る。
iOSは、SDKの`urlSessionError`かで見る。
2026-11-13に、iPhoneで通信を切って同期したとき、この種類のエラーが出た。
SDKの生のエラーの文字は、ログにだけ書き、画面には出さない。

サインイン、サインアウト、アクセストークンは、今のままSDKに任せる。
サインアウトのとき、シェルはRustに知らせる。
Rustは、カーソルのファイル`dropbox-cursor`を消し、状態を`SignedOut`にする。
記録のファイル`dropbox-record`は消さない。

### 同期の流れ

1. シェルは、`PetaCore`に同期を始めるよう頼む。前に出たとき、後ろに回ったとき、「Sync」、編集の3秒後である。
2. Rustは、同期のスレッドを1つ作る。同期が走っている間に頼まれたら、次の1回を予約する。今の`CloudSync`と同じである。
3. Rustは、シェルの`DropboxFiles`を通して、Dropboxの変更をアプリのMarkdownフォルダーへダウンロードする。
4. Rustは、シェルにコアへの取り込みを頼み、それが終わるのを待つ。
5. Rustは、journal-coreがMarkdownフォルダーに書いたファイルのうち、記録と違うものを、シェルの`DropboxFiles`を通してアップロードする。
6. Rustは、状態が変わるたびに、シェルのコールバックに新しい状態を渡す。シェルは、それをメインスレッドへ移して画面に出す。

手順4をシェルに頼むのは、シェルがコアの呼び出しを1本の作業スレッドに並べているためである（Androidの`Shell.kt`の`worker`）。
Rustの同期のスレッドから`PetaCore::sync`を直接呼ぶと、編集のイベントと順番が入れ替わる。
シェルは、取り込みのあとに画面を描き直し、最初の画面をまだ開いていなければ開く。
これも今の`syncAndWait`のままである。

### ファイルとその移行

アプリの中のファイルの置き場所は、今のままにする。
カーソルは`dropbox-cursor`、記録は`dropbox-record`、Markdownフォルダーは、Androidが`filesDir`の下の`markdown`、iOSが`JournalModel`の`root`の下の`markdown`である。

記録の形式は、今のiOSのJSONに1つにする。
JSONのキーは、ファイルのパスを小文字にしたものである。
値は、Swiftの`CloudFile`と同じく、`path`、`contentHash`、`remote`の3つのキーを持つ。
iOSのアプリは、今の`dropbox-record`をそのまま新しい版で読める。

Androidの今の`dropbox-record`は、Javaの`Properties`の形式で書かれている。
Rustは、この形式を読まない。
新しい版のAndroidのアプリは、読めない`dropbox-record`を、記録がないものとして扱う。
記録がないと、最初の同期で次のことが起きる（ADR 031）。

1. Rustは、`dropbox-cursor`を使わず、Dropboxのフォルダーの中身をすべて読み直す。
2. Rustは、アプリのMarkdownフォルダーのファイルを、すべてDropboxの写しと見なす。
3. Dropboxと中身が違うファイルは、Dropboxの版で上書きされる。Dropboxにないファイルは消される。
4. Rustは、Dropboxの版で記録を作り、JSONで`dropbox-record`に書く。2回目の同期からは、記録を使う。

手順3で、まだアップロードしていない編集と、アプリで作ってまだアップロードしていないページが失われる。
そこで、Androidのアプリを新しい版に更新する前に、古い版で同期を済ませる。
同期が終わり、そのあとに編集していなければ、手順3で失われるものはない。
中身が同じファイルはダウンロードしないので、手順1から4は、content hashを比べる時間しかかからない。
このアプリを入れているのは開発者の端末だけなので、この手順で足りる。

### 食い違いの直し方

パスは、空の区切りを捨てて扱う。
今のiOSの振る舞いである。
先頭が`/`のパスも、Androidで見つかるようになる。

### パスの文字の正規化

2026-11-17追記：同期は、パスを比べるたびに、NFCにしてから小文字にする（`client-cloud-sync`の`fold`）。
比べるのは、Dropboxが返したパス、アプリのMarkdownフォルダーにある実際の名前、`dropbox-record`のキーである。
この追記は、iPhoneの実機で起きた失敗から決めた。
Rustに移した同期は、はじめパスを`to_lowercase`だけで比べていた。
iPhoneのMarkdownフォルダーには、ADR 054より前のSwiftの同期が作った、濁点を分けた名前（NFD）のフォルダーがあった。
Dropboxは、同じフォルダーを合成済みの名前（NFC）で返した。
Rustは、フォルダーがないと判断して`create_dir`で作ろうとし、APFSは同じ名前とみなして`File exists (os error 17)`を返した。
起動のたびの同期が、ページを1つも取らずに失敗した。
Swiftの同期では起きなかった。
Swiftの文字列の比較が、NFCとNFDを同じ文字とみなすためである。

ADR 007は、ページの名前を入口で一度だけNFCにし、比べるたびには正規化しないと決めた（Alternative 2を退けた）。
同期では、比べるたびに正規化する。
アプリのMarkdownフォルダーの名前は、ADR 007のとおり付け替えないので、NFDのまま残ることがある。
そのため、入口で一度そろえるだけでは足りない。
正規化はすべて`fold`の1つの関数を通るので、ADR 007が心配した、正規化を忘れる箇所は、この関数に集まる。
スマホのコアがページを取り込むときは、`journal-core`の`relativize`がADR 007のとおりNFCにする。
アップロードするパスは、アプリのMarkdownフォルダーでの綴りのままである。
Dropboxがその形をどう扱うかは、確かめていない。
Swiftの同期も同じだった。

### 移す順番

1. `client-cloud-sync`を作り、今のKotlinとSwiftの同期のテストをRustに移す。偽の`DropboxFiles`で確かめる。シェルは変えない。
2. `client-shell-embedded`から`client-cloud-sync`を外に出す。
3. iOSのシェルを切り替え、Swiftの同期のクラスとテストを消す。実機で確かめる。
4. Androidのシェルを切り替え、Kotlinの同期のクラスとテストを消す。

iOSを先にするのは、この開発用のMacで、`swift test`、シミュレーター、iPhoneの実機まで確かめられるためである。
同期の失敗を画面に出す文字を、SDKの生の文字から分けて出す課題（`docs/plans.md`の「iOSのシェル」）も、手順3で片づく。

## Alternatives

### Alternative 1: Dropboxとのやり取りも、Rustで書く

RustがDropboxのHTTP APIを直接呼ぶ。
シェルに残るのは、サインインだけになる。
シェルのコードは、この決定よりさらに約200行減る。
「通信ができないか」の見分けも、Rustの1か所になる。
Rustの`dropbox-sdk`クレート（`dropbox/dropbox-sdk-rust`、2026-11-17の時点で0.21.0）を使えば、APIを呼ぶコードも自分では書かない。
このクレートは、短命のアクセストークンが切れると、リフレッシュトークンで新しいトークンを取って呼び直す。
Dropboxのリフレッシュトークンは、更新のたびには変わらない（Dropboxのフォーラムでの職員の回答による）。
そのため、シェルは、OSのSDKが保存したリフレッシュトークンを同期のたびにRustへ渡すだけでよく、更新したトークンを書き戻す必要はない。
短所は2つある。
1つ目は、このクレートに公式のサポートがないことである。
READMEは「This SDK is not yet official」と書き、バグが直るとは限らないとしている。
同期の通信の中心を、この依存に置くことになる。
2つ目は、HTTPのクライアント（`ureq`）とTLS（`rustls`）がコアに加わることである。
各OSのSDKは、OSの通信の仕組みを使い、OSのプロキシの設定と証明書に従う。
`ureq`と`rustls`は、それとは別に振る舞う。
減る行数と、エラーの見分けの一本化に対して、この2つは大きすぎる。
選ばなかった。

### Alternative 2: 決まりだけをRustに置き、手順はシェルに残す

何をダウンロードし、何をアップロードするかの判断だけを、Rustの純粋な関数にする。
スレッド、進み具合、やり直しは、KotlinとSwiftに残す。
境界の形は単純になる。
短所は、`Downloader`、`Uploader`、`SyncRound`、`CloudSync`の大半が、両方のシェルに残ることである。
食い違いの1つ（`findWithoutCase`）は、ファイルを扱う手順の側にあり、この案では残る。
減る行数が少ない。
選ばなかった。

### Alternative 3: このままKotlinとSwiftに置き、食い違いだけを直す

`findWithoutCase`と記録の形式を、両方で同じにする。
変更は最も小さい。
短所は、次に決まりを変えるときも、両方を直すことである。
今回の2つの食い違いも、そうして生まれた。
選ばなかった。

### Alternative 4: Androidの`Properties`の形式を、Rustが読み替える

Rustは、`dropbox-record`をJSONとして読めなければ、`Properties`の形式として読み、次に保存するときにJSONで書く。
Androidのアプリは、更新の前に同期を済ませなくても、上げていない編集を失わない。
短所は、1回の移行のためだけに、読み替えのコードとテストを持つことである。
両方のOSが新しい版になったあとで、それを消す仕事も残る。
このアプリを入れているのは開発者の端末だけで、更新の前に同期を済ませれば、失うものはない。
選ばなかった。

## Confidence Level

確信度は中程度である。

決まりそのものには、確信がある。
決まりはADR 030とADR 031のままで、今のKotlinとSwiftのテストを、そのままRustのテストに移せる。
Rustのテストは、偽の`DropboxFiles`とテンポラリーのフォルダーで、どのOSでも回る。

確信が下がる理由は2つある。

- BoltFFI 0.32.1の外のトレイトから、Rustへ`Result`のエラーを返せるかを、まだ動かして確かめていない。BoltFFIのマクロのソースには、コールバックのエラーを扱うコードがある。返せなければ、成否を表すenumのレコードで返す。
- 1回の同期の中で、Rustのスレッドから、シェルの`DropboxFiles`とコールバックを何度も呼ぶ。ダウンロードは8本のスレッドで同時に走る。KotlinとSwiftの側の実装が、どのスレッドから呼ばれても動くことを、実機で確かめる必要がある。今の`SdkDropboxFiles`は、すでにメインでないスレッドから呼ばれている。

次のどれかが起きたら、この決定を見直す。

- BoltFFIの外のトレイトで、ダウンロードの速さが今より目に見えて落ちる。
- DropboxのほかにOneDriveなどのクラウドを足す。そのときは、`DropboxFiles`をクラウドごとのトレイトに分けるかを考える。
- SDKがトークンの更新をやめる、またはSDKが使えなくなる。そのときは、Alternative 1を考える。
- Rustの`dropbox-sdk`クレートが公式になる。そのときは、Alternative 1を考える。

## Consequences

良い結果は、次のとおりである。

- 同期の決まりが1か所になる。決まりを変えるときは、Rustの1か所を直せば、両方のOSに届く。
- 同期のテストが1つになる。Macでも、Windowsでも、`cargo test`で回る。シミュレーターもエミュレーターも要らない。
- `cargo-mutants`のミューテーションテストが、同期の決まりに効くようになる。今はKotlinにもSwiftにも、ミューテーションテストの道具がない。
- `findWithoutCase`と記録の形式の食い違いがなくなる。
- 同期の失敗が、両方のOSで同じ種類に分かれる。iOSの画面から、SDKの生のエラーの文字が消える。
- 両方のOSで、コードが約1,300行、テストが約2,200行減る。

悪い結果は、次のとおりである。

- Rustに、コードが約600行、テストが約1,000行増える。
- `client-shell-embedded`の入口が広がる。BoltFFIで生成するKotlinとSwiftのコードも増える。
- 同期の不具合を調べるときに、RustとKotlin、またはRustとSwiftの境界をまたぐ。ログは、シェルの`DropboxFiles`の層とRustの両方に出る。
- 移す間は、片方のOSだけがRustの同期を使う期間がある。その間に決まりを変えるときは、まだ移していない側も直す。

中立的な結果は、次のとおりである。

- サインインの流れは、今のまま、AndroidとiOSで違う（ADR 034の悪い結果）。
- Androidのアプリを新しい版にする前に、古い版で同期を済ませる必要がある。済ませずに更新すると、上げていない編集が失われる（Decisionの「ファイルとその移行」）。
- 新しい版のAndroidのアプリの最初の同期は、`dropbox-cursor`を使わず、Dropboxのフォルダーの中身をすべて読み直す。
- WindowsとmacOSのシェルは、ジャーナルサーバーを通すので、Dropboxの同期を持たない。このクレートを使わない。
- ADR 034のAlternative 2は、この決定で選ばれたことになる。ADR 034の「同期のコードが、KotlinとSwiftの2つになる」という悪い結果は、この決定で解消する。
