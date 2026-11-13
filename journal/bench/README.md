# Round-trip benchmark

`roundtrip.mjs` sends the requests the frontend makes while editing and measures, per request,
the time until the response arrives (when the frontend takes the saved state) and until the
matching SSE `pagesUpdated` event arrives. It needs a running server and Node 20+.

```
node bench/roundtrip.mjs http://localhost:8087 200          # Java server
node bench/roundtrip.mjs http://localhost:8197 200 --json   # Rust server, JSON output
```

The page under test has 30 items. Each operation runs 20 warm-up iterations, then the given
number of measured iterations (default 200). Run the benchmark twice against the Java server
and read the second run; the JIT needs the first one.

## Results (2026-10-16)

Same machine (4 vCPU, Linux container), one server at a time, second run of each,
200 iterations, milliseconds. Java: Quarkus 3.31, JDK 25. Rust: release build.

| Operation (frontend action) | Java p50 | Java p95 | Rust p50 | Rust p95 | SSE Java p50 | SSE Rust p50 |
|---|---|---|---|---|---|---|
| `PUT /api/items/{id}` body (auto-save while typing) | 28.4 | 41.1 | 1.0 | 1.6 | 27.0 | 0.8 |
| `POST /api/items` + `DELETE` (Enter, then remove) | 45.2 | 51.7 | 2.0 | 4.0 | - | - |
| `POST /api/items/shift-level` (Tab) | 16.6 | 23.5 | 0.8 | 1.4 | 15.0 | 0.7 |
| `POST /api/items/move` (Alt+Shift+Down) | 16.1 | 22.3 | 1.1 | 1.4 | 15.1 | 0.9 |
| `POST /api/items/{id}/cycle-task` (Ctrl+Enter) | 11.6 | 18.0 | 0.7 | 1.0 | 10.7 | 0.6 |
| `GET /api/pages/{id}` (reload, 30 items) | 2.8 | 4.6 | 1.0 | 1.3 | - | - |
| `GET /api/search/combined` | 3.1 | 6.6 | 2.7 | 3.4 | - | - |
| `GET /api/search/pages` (`[[` popup) | 1.4 | 2.5 | 0.5 | 0.8 | - | - |

The Rust numbers are with the search index committed in the background and `TCP_NODELAY`
on the server socket. Before those two changes the body auto-save took 29 ms on the Rust
server too. Each of the two costs that on its own, as measured by switching them one at a time:

| Rust server variant | `PUT` body p50 |
|---|---|
| index commit on every save, Nagle's algorithm on | 29.4 |
| index commit in the background, Nagle's algorithm on | 29.4 |
| index commit on every save, `TCP_NODELAY` | 29.9 |
| index commit in the background, `TCP_NODELAY` | 1.0 |

- The commit of the tantivy index costs about 26 ms (`examples/commit_bench.rs`: 0.01 ms to
  queue the document, 27 ms to commit and reload). A save now only queues its document; a
  background task commits every second, a search commits first when anything is pending (so a
  word typed a moment ago is still found), `flush()` commits on demand, and the server commits
  at shutdown.
- Without `TCP_NODELAY`, a response written in two pieces waits for the client's delayed ACK
  on a keep-alive connection; Node's `fetch` (and the browser) hit that, `curl` with a fresh
  connection did not.

The SSE event arrives about 0.2 ms before the HTTP response, so the frontend's state
confirmation is bounded by the request itself. The SQLite work of an item save is around
0.1 ms (`examples/save_bench.rs`).

```
cargo run --release --example commit_bench     # queue vs commit of one item
cargo run --release --example save_bench       # the steps of an item body save
```
