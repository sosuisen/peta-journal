# PetaJournal Build Guide

## Prerequisites

### 1. Java Development Kit (JDK) 25

### 2. Apache Maven 3.9+

### 3. Node.js 20+

The flatmarkdown library (`com.sosuisha:flatmarkdown`) comes from Maven Central and
bundles its native binaries for Linux x86-64, Windows x86-64 and macOS arm64, so there
is nothing to build by hand.

## Build

### 1. Build Journal Server

```powershell
cd peta-journal/journal-server
mvn clean package
```

This creates a Quarkus application in `target/quarkus-app/`.

To run in development mode:
```powershell
mvn quarkus:dev
```

To run the packaged application:
```cmd
java -jar target\quarkus-app\quarkus-run.jar
```

### 2. Build Frontend

Install dependencies:
```powershell
cd peta-journal/frontend
npm i
```

Run development server:
```powershell
npm start
```

Build for production:
```powershell
npm run build
```

The built files will be in `frontend\dist\`.

### 3. End-to-end tests

The end-to-end tests drive the real app with Playwright: the journal server on port 8187
with a throw-away data directory, and the Angular dev server on port 4300 in front of it.

```powershell
cd peta-journal/journal-server
mvn package -DskipTests
cd ../frontend
npx playwright install chromium   # once
npm run e2e
```

To run the end-to-end tests against the Rust server instead, build it first (see below) and set
`PETA_JOURNAL_SERVER=rust`:

```powershell
cd peta-journal/journal-server-rust
cargo build --release
cd ../frontend
$env:PETA_JOURNAL_SERVER = "rust"; npm run e2e      # PowerShell
PETA_JOURNAL_SERVER=rust npm run e2e               # bash
```

### 4. Server integration tests

`mvn test` in `journal-server` runs the unit tests and the integration tests (`*ApiTest`,
`FolderSyncTest`) together. The integration tests start the server once with its database,
search index, markdown folder and config file under a temporary directory, so they never
touch `~/.peta-journal`.

## Rust server (journal-server-rust)

`journal-server-rust` is a port of the journal server to Rust (axum, rusqlite, tantivy, rmcp).
It serves the same REST API, SSE stream and MCP tools on the same port, reads the same
`journal.db`, `config.json` and markdown folder, and can replace the Java server without
any change to the frontend or the extensions. See `docs/rust-rewrite-plan.md`.

### Prerequisites

- Rust 1.85 or newer (`rustup`), with a C compiler for the bundled SQLite.
- Network access during the first build: the Japanese dictionary (IPADIC) for the search
  index is downloaded by the `lindera-ipadic` crate and compiled into the binary. Set
  `LINDERA_BUILD_DICTIONARY_CACHE_DIR` to a directory to keep the built dictionary between
  clean builds.

### Build and run

```powershell
cd peta-journal/journal-server-rust
cargo build --release
./target/release/journal-server            # same defaults as the Java server (port 8087, ~/.peta-journal)
```

Every setting of `application.properties` has a flag and an environment variable:

| Java property | Flag | Environment variable |
|---|---|---|
| `quarkus.http.port` | `--port` | `PETAJOURNAL_PORT` |
| `petajournal.data.dir` | `--data-dir` | `PETAJOURNAL_DATA_DIR` |
| `petajournal.data.db` | `--db` | `PETAJOURNAL_DATA_DB` |
| `petajournal.index.dir` | `--index-dir` | `PETAJOURNAL_INDEX_DIR` |
| `petajournal.config.dir` | `--config-dir` | `PETAJOURNAL_CONFIG_DIR` |
| `petajournal.export.interval` | `--export-interval` | `PETAJOURNAL_EXPORT_INTERVAL` |
| `petajournal.import.enabled` | `--import-enabled` | `PETAJOURNAL_IMPORT_ENABLED` |
| `petajournal.import.debounce-ms` | `--import-debounce-ms` | `PETAJOURNAL_IMPORT_DEBOUNCE_MS` |
| `petajournal.import.scan-interval` | `--import-scan-interval` | `PETAJOURNAL_IMPORT_SCAN_INTERVAL` |
| `petajournal.import.delete-prompt-cooldown` | `--import-delete-prompt-cooldown` | `PETAJOURNAL_IMPORT_DELETE_PROMPT_COOLDOWN` |
| `quarkus.http.cors.origins` | `--cors-origins` | `PETAJOURNAL_CORS_ORIGINS` |

`RUST_LOG` controls logging (default `info`), e.g. `RUST_LOG=journal_server=debug`.

The search index format is not the same as Lucene's. On the first start the Rust server
replaces a Lucene index in `~/.peta-journal/index` with a tantivy index and rebuilds it from
the database; the database itself is not changed.

### Tests

```powershell
cd peta-journal/journal-server-rust
cargo test
```

`cargo test` runs the unit tests and the API tests (`crates/server/tests/api.rs`, the same
cases as the Java `*ApiTest` and `FolderSyncTest`). The API tests start the server once on a
free port with everything under a temporary directory.

## Development Workflow

1. Start journal server: `cd journal-server && mvn quarkus:dev`
   (or the Rust server: `cd journal-server-rust && cargo run`)
2. Start frontend: `cd frontend && npm start`
3. Access application at `http://localhost:4200`

