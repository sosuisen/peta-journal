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
| `PUT /api/items/{id}` body (auto-save while typing) | 28.4 | 41.1 | 29.2 | 42.7 | 27.0 | 28.9 |
| `POST /api/items` + `DELETE` (Enter, then remove) | 45.2 | 51.7 | 32.5 | 38.8 | - | - |
| `POST /api/items/shift-level` (Tab) | 16.6 | 23.5 | 1.2 | 3.0 | 15.0 | 0.9 |
| `POST /api/items/move` (Alt+Shift+Down) | 16.1 | 22.3 | 1.0 | 1.7 | 15.1 | 0.8 |
| `POST /api/items/{id}/cycle-task` (Ctrl+Enter) | 11.6 | 18.0 | 0.6 | 1.0 | 10.7 | 0.5 |
| `GET /api/pages/{id}` (reload, 30 items) | 2.8 | 4.6 | 1.0 | 1.9 | - | - |
| `GET /api/search/combined` | 3.1 | 6.6 | 2.6 | 3.8 | - | - |
| `GET /api/search/pages` (`[[` popup) | 1.4 | 2.5 | 0.5 | 0.7 | - | - |

Reading: the two operations that write the search index (body update, item creation) cost
about 30 ms on both servers, because both commit the search index on every write. On the Rust
server that commit is the whole cost: `examples/commit_bench.rs` measures one `index_item`
(delete, add, commit, reader reload) at about 26 ms, against 0.03 ms for parsing the body;
the SQLite work of an item save is around 1 ms, as the other operations show. Every other
edit is 10 to 15 ms faster on the Rust server, which comes down to about 1 ms. The SSE event
arrives about 1 ms before the HTTP response on both servers, so the frontend's state
confirmation is bounded by the request itself.

To make the body auto-save faster, the index commit would have to leave the request path
(for example one commit per second in the background, as the Java server already does for
`flush()` after folder imports). That changes when a just-typed word becomes searchable, so
it is left as a decision rather than done here.

```
cargo run --release --example commit_bench     # the breakdown of one item save
```
