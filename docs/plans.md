2026-10-31 ギア5速：テストもコードもAIが書く。人間は仕様書やADRを作成するだけで、コードレビューはしない

# plans

## #765 ネイティブのシェルで画像を貼り付ける（ADR 071）

- [ ] `save`と`save_file_at`を`journal-core`の`Journal`へ移す（リファクタリング。サーバーの既存のテストで守る）
- [ ] `client-api-embedded`は、パスのファイルを本文フォルダの`assets/`に保存し、保存した名前と元の名前を答える
- [ ] `client-native-bridge`は、パスで貼った画像を番号に変えてコアに渡し、項目に画像のリンクが入る
- [ ] `client-native-bridge`は、保存が終わると、一時ファイルを消す
- [ ] `client-native-bridge`は、保存に失敗しても、一時ファイルを消す
- [ ] `client-cloud-sync`は、`assets/`の画像もアップロードする
- [ ] Androidのシェルは、貼った画像を一時ファイルに書き、ブリッジに渡す
