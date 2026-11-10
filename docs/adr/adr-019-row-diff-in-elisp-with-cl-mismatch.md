# ADR 019: Row Diff in Elisp with cl-mismatch

## Status

Accepted

## Context

Emacs版（`shell-emacs`）は、シェルが送る行の並びをバッファに表示する。Elispの`peta-journal--apply-rows`が、古い行と新しい行を比べ、変わった行だけを書き直す。この比較は、自前のコードで書いていた。

Emacs版のEnterは、項目が多いページで遅い（ADR 018）。差分計算をRust側へ移す案と、`ewoc.el`で描き直す案があった。

先に速さを測り、遅くなければ変えないと決めていた。測ると、Enterの時間のうち`apply-rows`は、1000項目で5%、2000項目で3%だけだった。差分計算を動かしても、遅さは解決しない。

## Decision

Emacs版の行の差分計算は、Elisp側に置いたまま、組み込みの`cl-mismatch`で行う。

`cl-mismatch`は、自前の比較の18行を置き換えられ、比較は約3倍速い。

## Alternatives

### Alternative 1: 差分計算をRust側へ移す

Elispのコードは減る。しかし、`apply-rows`はEnterの時間の3%から5%だけである。移しても遅さは解決しないので、選ばなかった。

### Alternative 2: `ewoc.el`で描き直す

Emacsに組み込みの仕組みで、行を管理できる。しかし、差分の計算は別に要る。IDからノードを探す対応表も増える。行数は減らないと判断したので、選ばなかった。試してはいない。

### Alternative 3: `replace-region-contents`を使う

組み込みの関数だけで、差分の適用まで済む。しかし、文字の差分だけを見るので、字下げやIDだけが変わった行を更新しない。5000項目で1851msかかり、今の11.5msより大幅に遅い。そのため、選ばなかった。

## Confidence Level

確信度は高い。速さを測って決めた。ただし、`ewoc.el`は試していない。

次のときに見直す。

- `apply-rows`が、操作の時間の大きな部分を占めるようになったとき。

## Consequences

良い結果は次のとおりである。

- 自前の比較の18行がなくなる。
- 比較は約3倍速くなる。

悪い結果は次のとおりである。

- Enterの遅さは解決しない。原因はほかにある。

中立的な結果は次のとおりである。

- 差分計算はElisp側に残る。シェルは、これまでどおり行の並び全体を送る。
