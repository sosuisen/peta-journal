# Conventional Commits のスコープ

コミットメッセージと PR のタイトルの `(<scope>)` には、この一覧から選んだスコープを書く。

- スコープは、変更したディレクトリで決める。
- 関係するスコープが2つ以上あるときは、カンマで区切る（例: `test(stickies-mac,outliner-ios): ...`）。ただし、コミットを分けられるなら分ける。
- 次のときは、スコープを省略する。
  - 変更が4つ以上のスコープにまたがるとき
  - どのスコープにも当てはまらないとき（`docs/`、`.github/`、ルートの README など）
- 新しいスコープが必要と考えられる場合は、ユーザーに尋ねること。

## アプリ（`client/apps/`）

| scope | ディレクトリ |
|-------|-------------|
| `outliner-react` | `client/apps/web-app/outliner-react` |
| `todo-web` | `client/apps/web-app/todo` |
| `outliner-emacs` | `client/apps/emacs/outliner` |
| `outliner-ios` | `client/apps/native-app/outliner-ios` |
| `outliner-android` | `client/apps/native-app/outliner-android` |
| `stickies-mac` | `client/apps/native-app/stickies-mac` |
| `stickies-win` | `client/apps/native-app/stickies-win` |
| `swift-shared` | `client/apps/native-app/swift-shared`（macOSとiOSのSwiftのシェルが共有するパッケージ） |
| `memoview-win` | `client/apps/webview-app/memoview-win` |

## クライアントのライブラリ（`client/libs/`）

| scope | ディレクトリ |
|-------|-------------|
| `client-core` | `client/libs/client-core` |
| `client-api-embedded` | `client/libs/client-api-embedded` |
| `client-api-http` | `client/libs/client-api-http` |
| `client-shell-embedded` | `client/libs/client-shell-embedded` |
| `client-shell-http` | `client/libs/client-shell-http` |
| `client-native-bridge` | `client/libs/client-native-bridge` |
| `client-stickies` | `client/libs/client-stickies` |
| `client-libs` | 上記以外の`client/libs/`以下のディレクトリー |
| `client-tools` | `client/tools/`以下のディレクトリー |


## ジャーナル（`journal/`）

| scope | ディレクトリ |
|-------|-------------|
| `journal-server` | `journal/journal-server`（MCP を含む） |
| `journal-core` | `journal/journal-core` |
| `journal` | 上記以外の`journal/`以下のディレクトリー |

## そのほか

| scope | ディレクトリ |
|-------|-------------|
| `atlas` | `atlas` |
| `auto-template` | `ext-functions/auto-template` |
