# ADR 071: Paste Images in Native Shells Through a Temporary File

## Status

Accepted

## Context

ネイティブのシェルでは、画像を貼り付けられない（#765）。
まずAndroidで貼り付けられるようにしたい。

`client-core`は、貼り付けた画像を`u64`の番号で受ける（`Event::PasteImage { item_id, file, cursor }`）。
コアは、リンクを入れる位置を`ImageUpload`に覚えてから、`ApiOp::UploadAsset { file }`を出す（`client/libs/client-core/src/ops/paste.rs`の`paste_image`）。
番号を実際のファイルに結び付けるのは、シェルである。
Crux（コアの土台）では、イベントとエフェクトはシリアライズできる値でなければならない。
Webの画像はブラウザの`File`オブジェクトで、シリアライズできない。
このため、`File`はシェルに残し、コアには番号だけを渡している（`client/libs/client-core/src/effects.rs`の`UploadAsset`）。

シェルごとの今の扱いは次のとおりである。

- Webのアウトライナーは、番号から`File`を引き、自分でアップロードする（`client/apps/web-app/outliner/src/effects/api.rs`）。
- Emacsのアウトライナーは、番号からパスへの表（`image_files`）を持ち、パスをホストに渡す（`client/apps/emacs/outliner/src/shell.rs`）。
- AndroidとiOSは、`client-native-bridge`を通してアプリの中のジャーナル（`client-api-embedded`）を使う（ADR 024）。
  `client-api-embedded`は、`ApiOp::UploadAsset`にエラーを返す。

Androidの本文フォルダは、アプリ専用のフォルダ`filesDir/markdown`である。
Rustから、パスで直接書ける。

ファイル名を付けて`assets/`に画像を保存する処理`save`は、`journal-server`の`presentation/rest/assets.rs`にある。
名前の付け方（`saved_name`）とIDの生成（`generate_asset_id`）は、すでに`journal-core`にある。

Androidは、Dropboxでほかの端末と本文フォルダを同期する（ADR 031）。
`client-cloud-sync`の`MarkdownFolder::written`は、`.md`のファイルと`settings/properties.json`だけをアップロードの対象にする。
画像は、クラウドから来るだけである。
このため、Androidで貼った画像は、ほかの端末に届かない。
Androidの`markdown/assets/`の画像は、同期のダウンロードで消されはしない。
まだアップロードしていないファイルは、`Downloader`の`is_unsent`で守られるためである。

## Decision

### 渡し方

Androidのシェル（Kotlin）は、貼り付けた画像を一時ファイルに書き、そのパスを`client-native-bridge`に渡す。
`client-core`の番号（`file: u64`）は変えない。
コアや、WebとEmacsのシェルに手を入れずに済むためである。

手順は次のとおりである。

1. Kotlinは、クリップボードなどから受けた`content://`のURIから、画像のバイト列を読む。
   Kotlinは、そのバイト列を`cacheDir`の一時ファイルに書く。
2. Kotlinは、`NativeShell`の新しいメソッドを1回呼ぶ。
   渡すのは、一時ファイルのパス、項目のID、カーソルの位置である。
3. `client-native-bridge`は、番号からパスへの表を持つ。
   ブリッジは、パスに番号を割り当て、`Event::PasteImage`をコアに送る。
4. コアが`ApiOp::UploadAsset { file }`を出すと、ブリッジは番号をパスに戻す。
   ブリッジは、`client-api-embedded`を通して、そのファイルを本文フォルダの`assets/`に保存する。
5. ブリッジは、保存が成功しても失敗しても、一時ファイルを消し、表から番号を外す。

番号からパスへの表は、Kotlinではなく`client-native-bridge`に置く。
iOSのシェルも、同じメソッドを呼ぶだけで済むためである。
画像のバイト列は、bincodeでFFIを越えない。
数MBのバイト列を、イベントとして往復させないためである。

### 保存処理の置き場所

ファイル名を付けて`assets/`に保存する処理`save`を、`journal-server`から`journal-core`へ移す。
`journal-server`のRESTとMCP、`client-api-embedded`の3つが、同じ`save`を呼ぶ。
同じ操作は同じ処理で行うためである（ADR 060）。
`client-api-embedded`は、`ApiOp::UploadAsset`にエラーを返すのをやめ、`save`の結果を`ApiOutput::Uploaded`で返す。

### Dropboxへのアップロード

`client-cloud-sync`の`MarkdownFolder::written`に、`assets/`の下のファイルを加える。
Androidで貼った画像を、Dropboxにアップロードするためである。
これで、ほかの端末でもリンク切れにならない。

## Alternatives

### Alternative 1: バイト列をそのままFFIで渡す

Kotlinが画像のバイト列を、bincodeでブリッジに渡す。
一時ファイルは要らない。
しかし、数MBのバイト列がFFIを越え、ブリッジの表がバイト列を持つことになる。

### Alternative 2: Kotlinが`assets/`に保存し、ファイル名だけを渡す

Kotlinが画像に名前を付けて`assets/`に保存し、ファイル名だけをコアに渡す。
Rustの変更が最も少ない。
しかし、ファイル名の付け方をKotlinとSwiftの両方で書くことになる。
同じ決まりをシェルごとに書かないという方針（`docs/issue-labels.md`の`dedup`）に反する。

### Alternative 3: コアの番号を、番号かパスのどちらかを表す型に変える

ネイティブのシェルは、パスを直接コアに渡す。
ブリッジの表は要らない。
しかし、`client-core`と、WebとEmacsのシェルを変える必要がある。
番号の表は小さいので、選ばなかった。

### Alternative 4: Kotlinが一時ファイルを消す

アップロードが終わったことを、ブリッジからKotlinに知らせる。
Kotlinはそれを受けて一時ファイルを消す。
しかし、知らせる仕組みが新しく要る。
一時ファイルを読むのはブリッジなので、ブリッジが消すほうが簡単である。

## Confidence Level

高い。
コアの番号の仕組みは、WebとEmacsで動いている。
Androidの本文フォルダは普通のフォルダなので、Rustからパスで書ける。

次の場合は見直す。

- iOSで、本文フォルダにRustからパスで書けなかった場合。
- 一時ファイルを`cacheDir`に置くと、保存の前にOSに消される場合があると分かった場合。
- Dropboxに画像をアップロードする量が、同期の時間を大きく延ばした場合。

## Consequences

### 良い結果

- Androidで画像を貼り付けられる。
- iOSは、同じ`NativeShell`のメソッドを呼ぶだけで、同じことができる。
- `client-core`と、WebとEmacsのシェルは変わらない。
- 画像の保存処理が、`journal-core`の1か所にまとまる。
- 貼った画像が、Dropboxを通してほかの端末に届く。

### 悪い結果

- `client-native-bridge`が、番号からパスへの表という状態を持つ。
- 一時ファイルの書き込みと読み直しの分、画像1枚につきディスクへの書き込みが1回増える。

### 中立的な結果

- `journal-server`の`presentation/rest/assets.rs`は、`journal-core`の`save`を呼ぶだけになる。
- Dropboxの同期は、`assets/`の下のファイルも、内容が変わったかをハッシュで確かめる。
