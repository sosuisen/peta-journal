# Stickies — Build & Run

## Prerequisites

- Visual Studio 2026 (v18) with **.NET desktop development** and **Windows App SDK** workloads
- .NET 10 SDK
- The JavaScript file `vendor\fm-ast2html-<version>.js` (see the next section)

## Required JavaScript file

Stickies needs one JavaScript file to build and to run:

```
ext-desktop-app\stickies\vendor\fm-ast2html-<version>.js
```

`<version>` is the value of `FmAst2HtmlVersion` in `Stickies.csproj`, for example `fm-ast2html-0.6.0.js`.

This file is the npm package [flatmarkdown-ast2html](https://www.npmjs.com/package/flatmarkdown-ast2html).
It turns the AST of each item into HTML inside the WebView.
The package is the Rust crate `flatmarkdown-ast2html` built as WebAssembly, so Stickies shows the same HTML as the other frontends (ADR 023).
The WebAssembly code is inside the JavaScript file. There is no second file.

The build puts the file into `Stickies.exe` as an embedded resource.
You do not need to copy the file next to the `.exe`.

The file is not in the Git repository, because `vendor/` is in `.gitignore`.

### Automatic download

The build downloads the file from unpkg when it does not exist in `vendor\`.
The first build needs an internet connection.

### Manual download

Use this way when the build machine has no internet connection.
Replace `<version>` with the value of `FmAst2HtmlVersion` in `Stickies.csproj`.

```powershell
cd ext-desktop-app\stickies
mkdir vendor
curl.exe -L -o vendor\fm-ast2html-<version>.js https://unpkg.com/flatmarkdown-ast2html@<version>/dist/index.global.js
```

You can also build the file from the source.
The source is the folder `flatmarkdown-ast2html-wasm` of the [flatmarkdown](https://github.com/petajournal/flatmarkdown) repository.
Run `npm run build` there. Its README lists the tools that the build needs.
Then copy `dist\index.global.js` to `vendor\fm-ast2html-<version>.js`.

### Changing the version

Change `FmAst2HtmlVersion` in `Stickies.csproj` and build again.
The version is part of the file name, so the build downloads the new file.
You can delete the old file in `vendor\`.
## Build

**Important:** Use `msbuild` from Developer PowerShell for VS 18 (not `dotnet build`), because WinUI3's PRI generation requires VS build tooling.

Open **Developer PowerShell for VS 18**, then:

```powershell
cd ext-desktop-app\stickies
msbuild Stickies.csproj -restore -p:Configuration=Debug -p:Platform=x64 -verbosity:minimal
```

Output: `bin\x64\Debug\net10.0-windows10.0.22621.0\Stickies.exe`

## Run

1. Start the journal server first. Stickies connects to `http://localhost:8087` (REST API and SSE). See `BUILD.md` at the root of the repository.
2. Run `bin\x64\Debug\net10.0-windows10.0.22621.0\Stickies.exe`

## Test

The tests are in `ext-desktop-app\stickies-tests`. They need only the .NET 10 SDK.

```powershell
cd ext-desktop-app\stickies-tests
dotnet test
```

Run `dotnet test` in this folder, not at the root of the repository.
The `global.json` in this folder selects the test runner, and the SDK finds it only from the current folder upward.

The tests cover the code that does not use WinUI: the rows of the items, the state of a task, the addresses of links and images, the page document, the messages from the page, and the settings file.
They do not start the app. The window has no tests.
The script inside the WebView (`Web\page.js`) does not run in the tests. They only check that it sends the messages the app reads.

## Settings

Window state is persisted at `~/.peta-stickies` (JSON). Delete this file to reset all windows.

Closing a sticky removes it from this file. It does not come back at the next start.
When the last sticky is closed, the app ends, and the next start opens one new sticky.
