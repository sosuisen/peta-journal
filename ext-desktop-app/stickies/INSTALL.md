# Stickies — Build & Run

## Prerequisites

- Visual Studio 2026 (v18) with **.NET desktop development** and **Windows App SDK** workloads
- .NET 10 SDK
- The JavaScript file `vendor\fm-ast2html.min.js` (see the next section)

## Required JavaScript file

Stickies needs one JavaScript file to build and to run:

```
ext-desktop-app\stickies\vendor\fm-ast2html.min.js
```

This file is the browser build of [flatmarkdown-ast2html](https://github.com/petajournal/flatmarkdown-ast2html).
It turns the AST of each item into HTML inside the WebView.
The build puts the file into `Stickies.exe` as an embedded resource.
You do not need to copy the file next to the `.exe`.

The file is not in the Git repository, because `vendor/` is in `.gitignore`.

### Automatic download

The build downloads the file from unpkg when `vendor\fm-ast2html.min.js` does not exist.
The first build needs an internet connection.
The version is the value of `FmAst2HtmlVersion` in `Stickies.csproj`.

### Manual download

Use this way when the build machine has no internet connection.
Replace `<version>` with the value of `FmAst2HtmlVersion` in `Stickies.csproj`.

```powershell
cd ext-desktop-app\stickies
mkdir vendor
curl.exe -L -o vendor\fm-ast2html.min.js https://unpkg.com/flatmarkdown-ast2html@<version>
```

You can also build the file from the source of flatmarkdown-ast2html.
Run `npm install` and `npm run build` there.
Then copy `dist\index.global.js` to `vendor\fm-ast2html.min.js`.

### Changing the version

The build does not download the file again when the file already exists.
After you change `FmAst2HtmlVersion`, delete `vendor\fm-ast2html.min.js` and build again.

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

## Settings

Window state is persisted at `~/.peta-stickies` (JSON). Delete this file to reset all windows.
