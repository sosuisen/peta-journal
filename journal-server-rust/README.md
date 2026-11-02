# journal-server-rust

The PetaJournal journal server in Rust: the same REST API, SSE stream and MCP tools as the
Java server in `../journal-server`, on the same port (8087), over the same `journal.db`,
`config.json` and markdown folder.

```
cargo build --release
./target/release/journal-server --help
cargo test
```

| Crate | What it is |
|---|---|
| `crates/hmtid` | HMTID (human-readable monotonic timestamp identifier), a port of the `hmtid` npm package |
| `crates/server` | The server: `domain/` (models, fractional index, hashtags), `application/` (page, item, journal and search use cases), `infrastructure/` (SQLite, tantivy search, markdown export/import/reconcile, folder watcher), `presentation/` (REST, SSE, MCP) |

## Coverage and mutation testing

Setup, once:

```
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo install cargo-mutants --locked
```

After a change, read the coverage for the lines no test reaches (the numbers are not a target):

```
cargo cov            # a table per file
cargo cov --html     # target/llvm-cov/html/index.html, line by line
```

Before a PR, run the mutation test on the changed code. Every mutant runs the test suite of the
crate, and the workspace has about 3600 mutants, so a full run takes many hours:

```
git diff main...HEAD > /tmp/pr.diff
cargo mutants --in-diff /tmp/pr.diff --in-place
```

`--in-place` is needed because `peta-atlas` is a path dependency outside this workspace, which
the copy cargo-mutants normally works in cannot reach. The files no test can run (the entry
point, the native folder dialog, the launcher) are left out of both; `.cargo/config.toml` and
`.cargo/mutants.toml` say which and why.

Settings: see the "Rust server" section of `../BUILD.md`. Design and background:
`../docs/rust-rewrite-plan.md`.
