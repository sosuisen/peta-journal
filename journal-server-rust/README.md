# journal-server-rust

The PetaJournal journal server in Rust: the same REST API, SSE stream and MCP tools as the
Java server in `../journal-server`, on the same port (8087), over the same `journal.db`,
`config.json` and markdown folder. It also serves the web frontend, the `dist/` folder of
`../frontend-rust/crates/shell-leptos`, so the app opens at `http://localhost:8087`.

```
cargo build --release
./target/release/journal-server --help
cargo test
```

| Crate | What it is |
|---|---|
| `crates/hmtid` | HMTID (human-readable monotonic timestamp identifier), a port of the `hmtid` npm package |
| `crates/server` | The server: `domain/` (models, fractional index, hashtags), `application/` (page, item, journal and search use cases), `infrastructure/` (SQLite, tantivy search, markdown export/import/reconcile, folder watcher), `presentation/` (REST, SSE, MCP, the files of the web frontend) |

## Client ID

Every POST, PUT and DELETE under `/api/` must carry an `X-Client-Id` header. The server answers
400 Bad Request without it. See
[ADR 015](../docs/adr/adr-015-client-id-on-every-change-request.md).

- A client makes a new UUID when it starts and sends it with every change.
- The server sends the ID back as `clientId` in the `pagesUpdated` event of that change.
- A client reloads a page when the `clientId` of the event is not its own.

A client that only reads needs no ID. GET requests pass without the header, and that includes
the event stream at `/api/events`.

The ID has one use: a client tells its own changes from the changes of the other clients. A
client that only reads makes no change. So none of the events it gets is about its own change,
and it has nothing to tell apart. It can reload on every `pagesUpdated` event.

The form at `/api/sync/pending-deletes` needs no ID either.

The MCP tools at `/mcp` need no ID. An MCP client is never one of the clients that show pages.
A change by an MCP tool sends a `pagesUpdated` event with `"source": "mcp"` and no `clientId`.
Every client reloads the page on that event.

```
curl -X POST http://localhost:8087/api/journals/2026-11-10 \
  -H "X-Client-Id: 123e4567-e89b-42d3-a456-426614174000"
```

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
git diff --relative main...HEAD > /tmp/pr.diff
python -I scripts/mutants-parallel.py --in-diff /tmp/pr.diff
```

The script runs cargo-mutants in 3 shards at once, each in a git worktree of its own under
`target/mutants-shards/` of the main checkout. The worktrees and their builds stay there, so
the next run builds only what changed. Commit the changes first: the shards test the commit at
HEAD. `--jobs N` sets the number of shards, and the other arguments go to cargo mutants. A
shard's output is in `target/mutants-shards/shard<k>.log`.

The script passes `--in-place` to cargo mutants. It is needed because `peta-atlas` is a path
dependency outside this workspace, which the copy cargo-mutants normally works in cannot reach.
For a few mutants, `cargo mutants --in-diff /tmp/pr.diff --in-place` runs them one by one in
this tree. The files no test can run (the entry
point, the native folder dialog, the launcher) are left out of both; `.cargo/config.toml` and
`.cargo/mutants.toml` say which and why.

Settings: see the "Rust server" section of `../BUILD.md`. Design and background:
`../docs/rust-rewrite-plan.md`.
