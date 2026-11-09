# ADR 012: fm-ast Published as the flatmarkdown-ast Crate

## Status

Accepted

## Context

`fm-ast`は、Flat MarkdownのASTをRustの型として読むクレートである。`flatmarkdown`クレートが作るJSONを、`serde`で`Node`の木にする。本体は約480行で、依存は`serde`と`serde_json`だけである。

今は、PetaJournalのリポジトリの中（`frontend-rust/crates/fm-ast`）にあり、crates.ioには公開していない。使っているのは、同じワークスペースの`core`、`fm-leptos`、`shell-leptos`の3つである。どれもパスで参照している。

ASTに関わるものは、ほかはすでにPetaJournalの外にある。ASTを作る`flatmarkdown`クレートは、独立したリポジトリにあり、crates.ioに公開している。ASTの仕様（`SPEC_AST.md`）もそこにある。TypeScriptの型である`flatmarkdown-ast`も、独立したnpmパッケージである。Rustの型だけが、PetaJournalの中に残っている。

2026年11月、項目のツリーを別の形で描く仕組みを設計している。項目に`view`プロパティを付けると、その子孫をviewのクレートが描く。最初のviewは年表を描く`timeline`クレートである。`timeline`は独立したリポジトリに置き、PetaJournalとAstroのサイトの両方で使う。PetaJournalは、viewのクレートにCargoで依存しない。

`timeline`は、MarkdownのテキストではなくASTを入力として受け取る。PetaJournalは、項目ごとのASTをすでにDBに保存しているからである。そのため`timeline`は、Markdownのパーサーを持たずに済む。その代わり、ASTを読む型が要る。

`timeline`が今の`fm-ast`を使う方法は限られる。PetaJournalのリポジトリをGitで参照すると、`timeline`をcrates.ioに公開できない。`timeline`の中に型を写すと、ASTの仕様が変わるたびに2か所を直すことになる。

## Decision

`fm-ast`を、`flatmarkdown-ast`という名前の独立したクレートとして、crates.ioに公開する。名前は、同じASTのTypeScriptの型であるnpmパッケージ`flatmarkdown-ast`にそろえる。

クレートは、`flatmarkdown`のリポジトリ（`petajournal/flatmarkdown`）に置く。このリポジトリをCargoのワークスペースにし、`flatmarkdown`と`flatmarkdown-ast`の2つのクレートを持たせる。ASTを作るコード、ASTの仕様（`SPEC_AST.md`）、ASTを読む型が、1つのリポジトリに並ぶ。仕様を変えるときは、1つのPRで3つをそろえて直す。

2つのクレートの版は、常に同じにする。片方しか変えていないときも、両方の版を上げて公開する。同じ版の`flatmarkdown`が作ったASTは、同じ版の`flatmarkdown-ast`で必ず読める。`flatmarkdown-ast`の往復のテストは、ワークスペースの中の`flatmarkdown`をパスで参照する。

PetaJournalは、`frontend-rust/crates/fm-ast`を消し、crates.ioの`flatmarkdown-ast`に依存する。`core`、`fm-leptos`、`shell-leptos`の`use fm_ast::`は、`use flatmarkdown_ast::`に書き換える。`timeline`などのviewのクレートも、同じクレートに依存する。

TypeScriptの型のリポジトリ`petajournal/flatmarkdown-ast`は、`petajournal/flatmarkdown-ast-ts`に名前を変える。Rustのクレートと同じ名前のリポジトリが、TypeScriptのコードを持つことになるからである。手元のフォルダの名前と、`package.json`の`repository`のURLも合わせて直す。npmのパッケージの名前は、`flatmarkdown-ast`のまま変えない。使う側は、npmの名前で参照しているので、影響を受けない。2つのリポジトリのREADMEには、相手への案内を書く。

リポジトリの一番上の`Cargo.toml`は、ワークスペースの定義だけを持つ。2つのクレートは、`flatmarkdown/`と`flatmarkdown-ast/`のフォルダにそれぞれ置く。既存の`flatmarkdown`の`src/`と`tests/`は、`flatmarkdown/`の下へ移す。版は、一番上の`Cargo.toml`に1回だけ書き、2つのクレートがそれを参照する。リポジトリの直下で`cargo test`を実行すると、両方のクレートを試す。`SPEC_AST.md`は、リポジトリの直下に残す。

## Alternatives

### Alternative 1: PetaJournalのリポジトリをGitで参照する

`timeline`の`Cargo.toml`から、PetaJournalのリポジトリの`fm-ast`をGitの依存として指す。

クレートを動かさずに済む。しかし、Gitの依存を持つクレートは、crates.ioに公開できない。`timeline`が、PetaJournalのリポジトリ全体の取得にも縛られる。そのため採らなかった。

### Alternative 2: viewのクレートが、必要な型だけを自分で持つ

`timeline`の中に、年表が使うノードだけの小さな型を書く。

依存が増えない。しかし、同じASTの型が2か所にできる。ASTの仕様が変わるたびに、両方を直す必要がある。viewのクレートが増えると、写しも増える。そのため採らなかった。

### Alternative 3: `flatmarkdown`クレートの中に型を入れる

新しいクレートを作らず、`flatmarkdown`にASTの型を足す。

公開するクレートが1つで済む。しかし、ASTを読むだけの側も、Markdownのパーサーである`comrak`に依存することになる。`timeline`がASTを受け取る狙いは、パーサーを持たないことである。機能フラグで切り分けることはできるが、使う側が毎回指定する必要がある。そのため採らなかった。

### Alternative 4: 新しいリポジトリを作る

`flatmarkdown-ast`だけのリポジトリを作る。npmの`flatmarkdown-ast`と同じ形である。

リポジトリごとの役割は単純になる。しかし、ASTの仕様を変えるときに、作る側と読む側の変更が2つのリポジトリに分かれる。2つの版をそろえる決まりも、守りにくくなる。そのため採らなかった。

### Alternative 5: `fm-ast`の名前のまま公開する

PetaJournalの中の`use fm_ast::`を書き換えずに済む。TypeScriptのリポジトリと名前が重なることもない。しかし、crates.ioで見たときに、何のASTかが名前から分からない。npmのパッケージとの対応も見えない。そのため採らなかった。

### Alternative 6: 2つのクレートの版を別々にする

`flatmarkdown-ast`を0.1.0から始め、独立して版を上げる。

型だけを直したときに、`flatmarkdown`の版を上げずに済む。しかし、どの版の`flatmarkdown`のASTを、どの版の`flatmarkdown-ast`で読めるかを、別に管理する必要がある。そのため採らなかった。

### Alternative 7: viewのクレートがMarkdownのテキストを受け取る

`storymap`クレートと同じく、`timeline`がMarkdownの文書を受け取り、自分で解析する。この場合、ASTの型を公開する必要がない。

しかし、`timeline`がMarkdownのパーサーを持つことになり、wasmが大きくなる。wikiリンクやハッシュタグの解釈も、PetaJournalと別になる。PetaJournalは、項目ごとのASTをすでに保存している。そのため採らなかった。

## Confidence Level

確信度は高い。`fm-ast`は小さく、依存も`serde`と`serde_json`だけである。ASTを作るクレートとTypeScriptの型は、すでにPetaJournalの外で公開している。Rustの型を同じ扱いにするだけなので、新しい種類の仕組みは増えない。この決定は、後から変えることもできる。crates.ioに出した版は消せないが、PetaJournalがパスの参照に戻すことは、いつでもできる。

確信が中くらいなのは、2つのクレートの版を常に同じにする決まりである。型の小さな直しのたびに、`flatmarkdown`の版も上がる。公開の回数が増えて負担になるなら、版を別々にすることを見直す。

次の場合は、この決定そのものを見直す。

- viewのクレートがASTを受け取る設計をやめた場合。PetaJournalの外に、型を使う側がなくなる。
- ASTの仕様を頻繁に変える時期が来た場合。変更のたびにcrates.ioへの公開を待つことになり、PetaJournalの開発が遅くなる。その間は、PetaJournalがGitやパスで参照する形に一時的に戻すことを考える。

## Consequences

### 良い結果

viewのクレートは、crates.ioの`flatmarkdown-ast`に依存できる。`timeline`自身も、crates.ioに公開できる。PetaJournalの外でASTを読むプログラムは、型を自分で書かずに済む。

ASTを作るコード、仕様、読む型が、1つのリポジトリに並ぶ。仕様の変更は、1つのPRで3つにそろえて入る。往復のテストは、常に同じ版の`flatmarkdown`が作ったASTで動く。

ASTに関わる3つの公開物の名前がそろう。作る側が`flatmarkdown`、Rustの型がcrates.ioの`flatmarkdown-ast`、TypeScriptの型がnpmの`flatmarkdown-ast`である。

### 悪い結果

PetaJournalでASTの型を直す手順が長くなる。今は、同じワークスペースのファイルを直すだけである。これからは、`flatmarkdown`のリポジトリで直し、crates.ioに公開し、PetaJournalの依存の版を上げる。手元で試す間は、`[patch.crates-io]`でパスを指すことができる。

`Node`の型が、公開したAPIになる。ノードの種類を足すと、`match`ですべての種類を並べている利用者のコードが、コンパイルできなくなる。これは、利用者が新しい種類に気づくための仕組みとして残し、`#[non_exhaustive]`は付けない。その代わり、種類を足すときは、互換性のない変更として版を上げる。たとえば0.4から0.5に上げる。

公開の仕事が増える。2つのクレートを、毎回そろえて公開する。片方しか変えていないときも、両方の版が上がる。

移行の作業が、3つのリポジトリにまたがる。`flatmarkdown`では、ファイルを1段下のフォルダへ移し、READMEとCIのパスを直す。TypeScriptのリポジトリでは、名前を変え、`package.json`のURLを直す。PetaJournalでは、`fm-ast`を消し、`use`を書き換える。

### 中立的な結果

PetaJournalの中で、2つのクレートの版をそろえる必要がある。サーバーは`flatmarkdown`でASTを作り、フロントエンドは`flatmarkdown-ast`で読む。版を上げるときは、両方を同時に上げる。

`frontend-rust`のワークスペースから、`crates/fm-ast`がなくなる。`core`、`fm-leptos`、`shell-leptos`の3つは、`use flatmarkdown_ast::`に変わる。

TypeScriptのリポジトリの古いURLは、GitHubが新しいURLへ転送する。既存のリンクと、手元の`git remote`は、そのまま動く。

このADRは、型の置き場所だけを決める。viewの仕組みそのもの、つまり`view`プロパティ、wasmの実行環境、viewのクレートへの入力の形は、別のADRで決める。
