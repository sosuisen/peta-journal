# outliner-react

The web shell of PetaJournal in React (ADR 079, ADR 080). The core runs in wasm
(`client/libs/client-web-bridge`). This shell draws the rows and the panels that the core
sends, and does the operations of the screen that the core asks for.

## Build

The journal server serves `dist/` of this folder at `http://localhost:8087` by default
(`journal/.cargo/config.toml`). It needs Node.js, and
[wasm-pack](https://rustwasm.github.io/wasm-pack/) for the wasm of the core:

```
cargo install wasm-pack
./release.ps1    # npm install, then npm run build (on macOS and Linux: release.sh)
```

`npm run build` builds the wasm and the types of the border first. The server need not
restart: reload the browser after the build.

## Commands

```
npm install
npm run border   # builds the wasm and the types of the border from the Rust code
npm run dev      # runs `border` first, then the Vite dev server
npm test         # the tests, in jsdom
npm run lint
npm run format
```

The tests draw the components in jsdom with Testing Library. A fake wasm
(`src/test/shell.ts`) stands in for the bridge. The real `Core` of `src/core.ts` runs on it,
so the tests also read and write the JSON of the border.

## Coverage and mutation testing

After a change, read the coverage for the lines no test reaches:

```
npm run coverage   # a table per file, and coverage/index.html line by line
```

`npm run coverage` fails when less than 90% of the lines are covered, and the CI runs it. A line
no test reaches is either removed, or given a test of the behaviour that needs it.

Before a PR, run the mutation testing of Stryker on the files the PR changed:

```
npm run mutants -- --mutate src/screen.ts,src/core.ts
npm run mutants -- --mutate "src/OutlineItem.tsx:1-345"   # a part of a large file
```

The report is in `reports/mutation/index.html`. The whole app takes about 4 minutes.

These files are left out of both:

- `src/wasm/` and `src/border/`: they are made from the Rust code.
- `src/test/` and the tests.
- `src/layout.ts`: it measures where the browser lays out text. jsdom lays nothing out, so no
  test can check it. It is checked in a real browser. Agreed on 2026-11-23.
- `src/main.tsx`: the entry point, which loads the wasm and puts the app in `#root`.

The reasons are also in `vitest.config.ts` and `stryker.config.mjs`.

Some mutants are left alive on purpose:

- the text of an error or a log message
- a React `key`
- the dependency list of an effect whose values never change
- a font size or another value of the layout
- a mutant that gives the same result, as setting the value of a textarea to the value it has
