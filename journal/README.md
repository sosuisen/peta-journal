# journal

The PetaJournal journal server in Rust: the same REST API, SSE stream and MCP tools as the
Java server in `../journal-server`, on the same port (8087), over the same `journal.db`,
`config.json` and markdown folder. It also serves the web frontend, the `dist/` folder of
`../client/apps/web-app/outliner`, so the app opens at `http://localhost:8087`.

```
cargo build --release
./target/release/journal-server --help
cargo test
```

`journal-server/debug.ps1` builds the debug build and starts it (`cargo run`), and
`journal-server/release.ps1` makes the release build. Both have an `.sh` twin.

| Crate | What it is |
|---|---|
| `hmtid` | HMTID (human-readable monotonic timestamp identifier), a port of the `hmtid` npm package |
| `journal-core` | `journal-core`, the library under the server (ADR 029): `domain/` (models, fractional index, hashtags), `application/` (page, item, journal and search use cases, extensions), `infrastructure/` (SQLite, tantivy search, markdown export/import/reconcile), `dto` (the answers as JSON). It does not depend on tokio or axum. Its `extensions` feature, on by default, runs the scripts of the extension functions with QuickJS; a build without it runs none |
| `property-names` | The type of a property, taken from the suffix of its key name (ADR 047). The server and the clients share it, so they read a key name alike |
| `journal-api-types` | The bodies of the requests to the journal and the messages of its event stream. The journal and the frontend core write and read them with the same types, so the two cannot name a field differently. It depends on serde alone, as the frontend core is also built for wasm |
| `journal-server` | The server over `journal-core`: `presentation/` (REST, SSE, MCP, the files of the web frontend), the folder watcher, the periodic tasks, and what hands over to the OS (folder dialog, launcher) |

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

## Writing the page files again

The server writes the markdown file of a page when the page changes. A file therefore keeps
the shape of the server that wrote it last. The files from before the server wrote the
modified date of each item have none. A database that is built from such files gives the
items other dates.

The `write-pages` command writes the file of every page again. Stop the server first.

```
journal-server write-pages
```

It takes in what changed in the folder first. Then it writes each file that does not say what
its page says, and tells how many it wrote. The database stays as it is. The files of the
atlases get no dates of items: a device builds an atlas from its own pages.

A written file gets everything this server writes, not the dates alone. In an old file a link
to a page is `[[Java/Thread]]` and the tags of an item are `tag="book"`. This server writes
`[Java/Thread](Thread.md)` and `tags="book"`.

A device that has the same notes through a synced folder takes the written files in. A few
items do not come back from their file exactly as they are in the database, such as a body
that ends with a line break. That device gives those items the date of that day.

## Moving the asset files to ID names

Uploads are saved under a name made of an HMTID (ADR-022). The files that were saved before
that keep their old names, such as `image-3.png`. The `asset-names` command moves them, and
the links to them, to ID names. It is run by hand, once.

Stop the server first. The command opens the database and the search index as the server
does, and the server holds the search index.

```
journal-server asset-names plan   --file ~/.peta-journal/asset-names.json
journal-server asset-names apply  --file ~/.peta-journal/asset-names.json
journal-server asset-names revert --file ~/.peta-journal/asset-names.json
```

- `plan` writes the table of old and new names to the file. It changes nothing else. Read the
  table before the next step.
- `apply` renames the files, points the links in the items to the new names, and writes the
  markdown files of those pages. The items and the pages keep their modified dates.
- `revert` does the same in the other direction, with the same table. Keep the file.

The time in a new name is the modified time of the file, or the time the file was made when
that is older. The text of a link stays as it is.

The options of the server, such as `--config-dir` and `--data-dir`, come before
`asset-names`. Try the command on a copy of the notes first: give `--data-dir`, `--index-dir`
and `--config-dir` of the copy, with a `config.json` whose `markdownDir` is the copied folder.

A device that has the same notes through a synced folder gets the new links with the markdown
files. Its server takes them in as edits made by hand. The items with a link then get the date
of that day as their modified date, on every device.

To keep the modified dates on that device too, run `apply` there with the same table before
the folder brings the files: stop the server and pause the sync first. Both devices then
write the same files.

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
this tree. `cargo cov` measures `journal-core`, `journal-server`, `property-names` and `journal-api-types` together,
as the tests of the server run most of the core. The mutants of `property-names` time out when
the whole workspace is tested, as the server tests run first and take longer than the limit;
run them with `--test-package property-names`. The files no test can run (the entry
point, the native folder dialog, the launcher) are left out of both; `.cargo/config.toml` and
`.cargo/mutants.toml` say which and why.

Settings: see the "Rust server" section of `../BUILD.md`.
