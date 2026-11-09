<img alt="PetaJournal" src="https://github.com/sosuisen/peta-journal/blob/main/images/peta-journal.png" width=60 height=60 align="left"> 

# PetaJournal

<br>

English | [日本語](README.ja.md) (how to start the app, in Japanese)

A headless, offline-first outliner on top of local Markdown files.


## Features

PetaJournal is an open environment that loosely connects simple outliner, note-taking, task management, and knowledge tools around a local journal server.

- Offline-first
- Read and write as Markdown files
- Local API server architecture
  - MCP built-in
  - Journal server
  - Extension functions
- Create independent apps using public APIs
- Cross-app communication via the journal server
- No data storage on central remote servers; distributed multi-device sync
- End-to-end encryption

## Markdown folder sync

The journal server keeps the database and the markdown folder in sync in both directions.

- **DB → folder**: every change is written to `{markdownDir}/docs/...` or `journals/...` a few seconds later.
- **Folder → DB**: the server watches the folder and merges edited files back into the database.
  Items are matched by the `<!-- id -->` comments and keep the `<!-- idx=... -->` written by the
  other device, so unchanged items are left alone and every device holds the same ids and order values.
  When both sides changed, the newer `modified` wins and the older version is kept as
  `name.conflict-<timestamp>.md`.
- **Deleted files are never deleted from the database automatically.** When files disappear,
  a confirmation page opens in your browser (`http://localhost:8087/api/sync/pending-deletes`).
  Only the pages you tick and submit are deleted; you can also write them back to the folder.
  The page keeps reopening (every 5 minutes by default) until nothing is left undecided.

Notes for Dropbox users:

- Mark the folder as **Available offline**. Online-only files work but load slowly.
- Dropbox "conflicted copy" files are ignored. Merge them by hand if needed.
- Folders hidden by selective sync show up on the confirmation page as missing files.
  Just leave them unticked.

Step-by-step journeys (in Japanese): [docs/folder-sync-journeys.md](docs/folder-sync-journeys.md).

The import settings are flags of the journal server (`--import-enabled`, `--import-debounce-ms`,
`--import-scan-interval`, `--import-delete-prompt-cooldown`); see [BUILD.md](BUILD.md).

## Build

### Windows ARM64

Install "C++ Clang Compiler for Windows" and "MSBuild support for LLVM(clang-cl) toolset" by Visual Studio Installer.

Execute `cargo` command on Developer PowerShell for VS.
