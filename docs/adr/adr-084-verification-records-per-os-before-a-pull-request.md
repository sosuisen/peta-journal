# ADR 084: Verification Records per OS Before a Pull Request

## Status

Superseded by ADR 086

## Context

PRの前の検証は、Claude Codeのフック（`.claude/hooks/verify-before-pr.mjs`）が強制する（2026-11-25に開発者が決めた）。
フックは、ブランチで変えたファイルから`scripts/checks.mjs`の`jobsFor`でジョブを選び、それを手元の機械で回す。
CIは、同じ`scripts/checks.mjs`を、Linux、Windows、macOSのランナーで回す。

### 手元の機械では、ほかのOSのジョブが回らない

ジョブの中には、決まったOSでしか回らない段がある。
たとえば`stickies-win`はWindowsで、`stickies-mac`はmacOSでしかビルドできない。
フックは、ほかのOSの段を飛ばして、その段をAIに知らせる。
AIは、知らされた段をPRの本文の「確かめていないこと」と、issueのチェックボックスに書く。

この仕組みには、次の穴がある。

- issueのチェックボックスは、人でもAIでも、回さずに付けられる。
  フックは、チェックが本当に回した結果かを確かめられない。
- チェックボックスは、どのコミットで確かめたかを持たない。
  Macで確かめたあとにコミットを足すと、その確認は古くなるが、チェックは残る。
- 複数のOSのシェルが共有するコード（`client-core`、`client-shell-http`などのRustのクレート、`swift-shared`）を変えたとき、どのOSで確かめるべきかを人が判断している。
  その判断はすでに`scripts/checks.mjs`の`touchedBy`にあり、人の判断は漏れる。

### PRを作ったあとのプッシュは、検証されない

フックが動くのは、`gh pr create`と、PRを作るREST APIの呼び出しだけである。
PRを作ったあとに同じブランチへプッシュしたコミットは、どこでも検証されない。

### CIは手で起こし、料金がかかる

CIは`workflow_dispatch`でだけ動く。
Windowsのランナーの料金はLinuxの2倍、macOSのランナーはおよそ10倍である。
このため、CIを起こすときはOSごとに回すかを選ぶ（入力`linux`、`windows`、`macos`）。
macOSの既定は「回さない」である。

## Decision

各ジョブに、通さなければならないOSを書く。
フックは、ジョブがそのOSで通った記録を、コミットのSHAごとにGitのnotesに残す。
フックは、PRを作るときと、PRのあるブランチへプッシュするときに、必要な記録がすべて揃っているかを確かめる。
揃っていなければ止める。

### ジョブのOS

`scripts/checks.mjs`の各ジョブに、`oses`を書く。
値は、Node.jsの`process.platform`の名前（`linux`、`win32`、`darwin`）の配列か、`['any']`である。

- `['any']`のジョブは、どれか1つのOSで通れば足りる。
  `journal`、`client`、`atlas`、`emacs-e2e`、`todo`、`outliner-react`、`guard`がこれにあたる。
- OSを並べたジョブは、並べたOSのすべてで通る必要がある。
  `stickies-win`と`memoview-win`は`['win32']`、`stickies-mac`は`['darwin']`である。
- `stickies-win`の`PetaBorder`とそのテストはmacOSでも回るが、`stickies-win`の`oses`に`darwin`は入れない。
  macOSでは画面のテスト（`PetaScreens.Tests`）が回らず、アプリ全体のカバレッジを満たせないためである。

ジョブが「あるOSで通った」とは、そのOSの機械で、ジョブのどの段も飛ばされずに通ったことをいう。
ほかのOSの段を飛ばしたジョブは、そのOSで通ったことにならない。

### 記録

記録は、Gitのnotesの`refs/notes/peta-verify`に書く。
notesは、コミットを変えずに、コミットのSHAにテキストを付ける。
1つのコミットのnoteは、1行に1つの記録を持つ。

```
stickies-win win32 pass 2026-11-25T21:21:25Z
client win32 pass 2026-11-25T21:05:10Z
```

1. フックは、ジョブを回す前に`origin`から`refs/notes/peta-verify`を取ってくる。
2. フックは、ジョブがその機械のOSで通ったら、`HEAD`のSHAのnoteに1行を足す。
3. フックは、`refs/notes/peta-verify`を`origin`にプッシュする。
   ほかの機械も記録を足していてプッシュが断られたら、取ってきて`git notes merge -s cat_sort_uniq`で合わせてから、もう一度プッシュする。

記録は、そのSHAのコミットにだけ効く。
コミットを足すと、新しいSHAには記録がないので、ジョブを回し直す。
`docs/`の下だけを変えたコミットは、`jobsFor`がジョブを選ばないので、記録を求めない。

### CIの結果

CIで通ったジョブも、記録として数える。
フックは、`gh api`で、`HEAD`のSHAで回ったワークフロー`CI`の実行を引く。
`conclusion`が`success`のジョブを、`ci.yml`の`runs-on`のOSで、`scripts/checks.mjs`の同じ名前のジョブが通った記録とみなす。
Macを持たないときは、`macos=true`でCIを起こせば、`darwin`の記録になる。

### 止める場所

フックは、`Bash`と`PowerShell`のツールの、次のコマンドの前に動く。

- `gh pr create`と、PRを作るREST APIの呼び出し（今と同じ）
- `git push`のうち、プッシュするブランチに開いたPRがあるもの。
  フックは`gh pr list --head <branch> --state open`でPRを探す。

### 記録が足りないとき

フックは、その機械のOSで回せるジョブを回して記録を残したあと、`jobsFor`が選んだジョブの`oses`ごとに記録を確かめる。
1つでも足りなければ、コマンドを止める。
止めるときは、足りないジョブとOSとSHAを示す。

フックは、PRの本文の`Refs #N`のissueに、`## OSごとの検証（フックが書く）`という節を書く。
節には、足りないジョブとOSごとに、`- [ ] stickies-mac on darwin at 1a2b3c4d`の形のチェックボックスを置く。
記録が揃ったら、フックがチェックを付ける。
人もAIも、この節を手で書き換えない。
`gh pr create`のときはまだPRがないので、フックはコマンドの`--body`か`--body-file`から`Refs #N`を読む。

## Alternatives

### Alternative 1: 手で付けるチェックボックス（今のやり方）

ほかのOSの段を知らせ、人かAIがissueにチェックボックスを置いて付ける。
仕組みはいらない。
しかし、回さずにチェックを付けられ、どのコミットで確かめたかも残らない。
スキルに書いただけの決まりと同じく、飛ばせる。

### Alternative 2: CIの結果だけを証拠にする

偽れない証拠はCIの結果だけである。
しかし、プッシュのたびにCIが要り、Windowsは2倍、macOSはおよそ10倍の料金がかかる。
手元で通したことが、まったく数えられない。

### Alternative 3: notesに加えて、PRごとに必ずCIを回す

notesで足りるとしつつ、PRには必ずCIを1回回し、notesとCIの結果を照らし合わせる。
偽りを見つけられるが、料金はAlternative 2に近づく。
今は開発者が1人であり、わざと偽る危険は小さいと判断した。

### Alternative 4: `gh pr create`だけで止める

今と同じく、PRを作るときだけ確かめる。
仕組みは小さい。
しかし、PRを作ったあとのプッシュが検証されない穴が残る。

### Alternative 5: 記録が足りなくても止めずに知らせる

PRの本文とissueに残すだけにする。
しかし、知らせを読み落とすと、確かめていないコミットがPRに入る。
飛ばせる決まりは決まりにならない。

## Confidence Level

中程度。

Gitのnotesを使った記録の残し方と、GitHubの`gh api`からCIの結果を引く方法は、どちらも実際には試していない。
実装の前に、小さなリポジトリで次を確かめる。

- notesのプッシュと、ほかの機械のnotesとの合わせ方（`cat_sort_uniq`）
- `gh api`で、SHAごとにCIのジョブの結果を引けること

次のときは、この決定を見直す。

- notesが、回さずに書かれたことが分かったとき。
  Alternative 3に移る。
- 開発者が増え、notesにプッシュできる人が増えたとき。
- GitHubがnotesを保存しなくなったとき。
- macOSのランナーの料金が下がったとき。
  Alternative 2が現実的になる。

## Consequences

### 良い結果

- ほかのOSのジョブを、黙って飛ばしてPRに入れることがなくなる。
- 記録はコミットのSHAに付くので、古い確認が新しいコミットの確認として数えられない。
- どのOSで確かめるべきかを、人ではなく`scripts/checks.mjs`が決める。
- PRを作ったあとのプッシュも、PRを作るときと同じく検証される。
- issueの節はフックが書くので、残りの作業が手書きの誤りなく見える。

### 悪い結果

- 複数のOSのシェルが共有するクレートを変えると、すべてのOSの記録が要る。
  Windowsで作業していても、Macで回すか、`macos=true`でCIを回すまで、PRを作れずプッシュもできない。
- PRのあるブランチへのプッシュのたびに、フックがジョブを回す。
  通ったジョブはSHAごとに覚えるので、同じコミットでは回し直さない。
- notesを`origin`にプッシュできない機械（オフライン、権限なし）では、記録がほかの機械から見えない。
- notesは、わざとなら偽れる。
  この危険は受け入れる。

### 中立的な結果

- GitHubの画面には、notesは表示されない。
  人が見るのは、フックが書くissueの節である。
- `scripts/checks.mjs`のジョブに`oses`が増え、`scripts/verification.test.mjs`がその値を見張る。
- 手元でほかのOSの段を飛ばす決まりは残る。
  飛ばしたジョブは、そのOSで通った記録にならないので、別の機械かCIで通す必要がある。
