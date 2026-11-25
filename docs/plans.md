2026-11-23 ギア5速：テストもコードもAIが書く。人間は仕様書やADRを作成するだけで、コードレビューはしない
2026-10-31 ギア5速：テストもコードもAIが書く。人間は仕様書やADRを作成するだけで、コードレビューはしない

# plans

## #1075 S2をoutliner-emacs（Rust）に広げる

- [x] Rustでは、`Event`の種類は`Event::PasteText`として書かれる
- [x] Rustで、`SseEvent::Open`のような`Event`で終わる別の名前の中の同じ名前や、長い名前は、種類に数えない
- [x] Rustのシェルでは、`.rs`のファイルが読まれる
- [ ] `rules/outliner-emacs.toml`を置き、`main`で何も出ない
