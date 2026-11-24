# Todo (todo-web)

A web app that lists the tasks of PetaJournal and moves each one to its next state. It is an
extension web app (ADR 009). React, Vite and TypeScript make it.

## Use it from the journal server

The journal server serves the extension web apps in `apps/` of its config folder (ADR 073).
The config folder is `~/.peta-journal` unless `--config-dir` names another one.

1. Build the app. The files go to `dist/`.

   ```
   ./release.ps1      # or ./release.sh
   ```

2. Copy what is in `dist/` to `apps/todo/` of the config folder. Copy the files, not the
   `dist/` folder itself.

   ```powershell
   New-Item -ItemType Directory -Force ~/.peta-journal/apps/todo
   Copy-Item -Recurse -Force dist/* ~/.peta-journal/apps/todo/
   ```

   ```sh
   mkdir -p ~/.peta-journal/apps/todo
   cp -R dist/. ~/.peta-journal/apps/todo/
   ```

3. Open `http://localhost:8087/apps/todo/`.

`release-all.ps1` (or `release-all.sh`) at the root of the repository does these steps too.

The journal server needs no restart. It reads the files on each request. After a change to
the app, build it and copy the files again.

The app calls the API at the origin of its page. To call another journal server, add
`?host=` and `?port=` to the URL, such as `?port=9000`.

## Develop

`./debug.ps1` (or `./debug.sh`) starts the dev server of Vite at `http://localhost:5180`. The
journal server has to run already. Open `http://localhost:5180/?port=8087`. The page is not
served by the journal server here, so `?port=` sends the app to it. The journal server lets
`http://localhost:5180` call it by default (`--cors-origins`).

`npm test` runs the tests, `npm run lint` the lint, and `npm run format` the formatter.

`npm run coverage` runs the tests and measures their coverage. It prints a table per file and
writes a browsable report to `coverage/index.html`. It fails when less than 90% of the lines are
covered, and the CI runs it. The settings are in `vitest.config.ts`.
