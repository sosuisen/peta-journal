2026-11-23 ギア5速：テストもコードもAIが書く。人間は仕様書やADRを作成するだけで、コードレビューはしない
2026-10-31 ギア5速：テストもコードもAIが書く。人間は仕様書やADRを作成するだけで、コードレビューはしない

# plans

## #1075 S2をoutliner-react（TypeScript）に広げる

- [x] TypeScriptでは、`Event`の種類は`kind: 'PasteText'`として書かれる。複数行のオブジェクトでも同じである
- [x] TypeScriptで、長い名前や、`kind`でないキーの同じ名前は、種類に数えない
- [x] TypeScriptのシェルでは、`.ts`と`.tsx`のファイルが読まれ、テストのファイル（`.test.ts`、`.test.tsx`）は読まれない
- [x] ルールの`skipped`に書いたフォルダは読まれない（`src/border`と`src/wasm`の生成物、テストの手助けの`src/test`）
- [ ] `rules/outliner-react.toml`を置き、`main`で何も出ない
