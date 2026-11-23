# Web版のシェルの試作（#971）

ADR 079の「試作」の節に従って、アウトラインの画面をSolidとReactで試作したものである。
2つの試作を比べるためだけのもので、どのアプリにも入らない。

## 中身

| フォルダー | 中身 |
|---|---|
| `bridge/` | wasmとTSの境界のクレート（`web-prototype-bridge`）。`Event`をJSONで受け取り、`Changes`をJSONかJSの値で返す。APIの要求、SSE、タイマーはwasmに残す |
| `shared/` | 2つの試作が共有するTS。境界の型、コアの包み、キーの変換、計測 |
| `react/` | Reactの試作（React 19、`@tanstack/react-virtual`） |
| `solid/` | Solidの試作（Solid 2、`@tanstack/virtual-core`） |
| `bench/` | ヘッドレスのChromeで、2つの試作を測るスクリプト |
| `seed.mjs` | 使い捨てのサーバーに、1000項目のページ（`proto-1000`）を作るスクリプト |

描き方のほかは、2つの試作で同じにした。
コアとのやり取りと見た目は、`shared/`にある。

## 動かし方

1. wasmを作る。

   ```
   cd bridge
   wasm-pack build --target web --out-dir ../pkg --release
   ```

2. 使い捨てのjournal-serverを立てる。
   `--throwaway-dir`を使うので、開発者のノートには触れない。

   ```
   journal-server --throwaway-dir <空のフォルダー> --port 18971
   ```

3. 1000項目のページを作る。

   ```
   node seed.mjs http://127.0.0.1:18971
   ```

4. 2つの試作の開発サーバーを立てる。
   それぞれの`npm install`を先に行う。

   ```
   cd react && npx vite    # http://localhost:5181/pages/proto-1000
   cd solid && npx vite    # http://localhost:5182/pages/proto-1000
   ```

URLのクエリで、当て方を選べる。

- `?carry=json`（既定）か`?carry=value`: `Changes`をJSONの文字列で運ぶか、`serde-wasm-bindgen`のJSの値で運ぶか。
- `?apply=store`（既定）か`?apply=signal`: Solidだけ。行をストアに`splice`で当てるか、新しい配列をsignalに入れるか。

## 測り方

```
cd bench
npm install
node measure.mjs 5
```

ヘッドレスのChromeで、次のことを測る。

- ページを読み込んでから、行が画面に出るまでの時間。
- `Reset`で全部の行を運ぶ時間（JSONとJSの値）。
- 編集欄でキーを打ってから、画面が直るまでの時間。
- 400項目の子を持つ親を、たたむ・広げるときに、画面が直るまでの時間。
- 検索欄に打ったとき、結果が出て、打った文字とフォーカスが消えないか。

IMEは、ヘッドレスでは確かめられない。
開発者が、ブラウザで日本語を打って確かめる。

## IMEの確かめ方（開発者が行う）

2つの試作で、次のことを確かめる。

1. 項目を押して編集欄を開き、日本語を変換しながら打つ。変換中の文字が消えないか、変換を確定したときに文字が重ならないか。
2. 変換中にEnterを押す。変換が確定するだけで、新しい項目ができないか。
3. 検索欄で日本語を変換しながら打つ。結果が出るたびに、変換中の文字が消えないか。
