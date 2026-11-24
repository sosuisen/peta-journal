# MemoView — Build & Run

## Prerequisites

- Visual Studio 2026 (v18) with **.NET desktop development** and **Windows App SDK** workloads
- .NET 10 SDK
- The JavaScript file `vendor\fm-ast2html-<version>.js` (see the next section)

## Required JavaScript file

MemoView needs one JavaScript file to build and to run:

```
client\apps\webview-app\memoview-win\memoview\vendor\fm-ast2html-<version>.js
```

`<version>` is the value of `FmAst2HtmlVersion` in `MemoView.csproj`, for example `fm-ast2html-0.10.0.js`.

This file is the npm package [flatmarkdown-ast2html](https://www.npmjs.com/package/flatmarkdown-ast2html).
It turns the AST of each item into HTML inside the WebView.
The package is the Rust crate `flatmarkdown-ast2html` built as WebAssembly, so MemoView shows the same HTML as the other frontends (ADR 023).
The WebAssembly code is inside the JavaScript file. There is no second file.

The build puts the file into `MemoView.exe` as an embedded resource.
You do not need to copy the file next to the `.exe`.

The file is not in the Git repository, because `vendor/` is in `.gitignore`.

### Automatic download

The build downloads the file from unpkg when it does not exist in `vendor\`.
The first build needs an internet connection.

### Manual download

Use this way when the build machine has no internet connection.
Replace `<version>` with the value of `FmAst2HtmlVersion` in `MemoView.csproj`.

```powershell
cd client\apps\webview-app\memoview-win\memoview
mkdir vendor
curl.exe -L -o vendor\fm-ast2html-<version>.js https://unpkg.com/flatmarkdown-ast2html@<version>/dist/index.global.js
```

You can also build the file from the source.
The source is the folder `flatmarkdown-ast2html-wasm` of the [flatmarkdown](https://github.com/petajournal/flatmarkdown) repository.
Run `npm run build` there. Its README lists the tools that the build needs.
Then copy `dist\index.global.js` to `vendor\fm-ast2html-<version>.js`.

### Changing the version

Change `FmAst2HtmlVersion` in `MemoView.csproj` and build again.
The version is part of the file name, so the build downloads the new file.
You can delete the old file in `vendor\`.
## Build

**Important:** Use `msbuild` from Developer PowerShell for VS 18 (not `dotnet build`), because WinUI3's PRI generation requires VS build tooling.

Open **Developer PowerShell for VS 18**, then:

```powershell
cd client\apps\webview-app\memoview-win\memoview
msbuild MemoView.csproj -restore -p:Configuration=Debug -p:Platform=x64 -verbosity:minimal
```

Output: `bin\x64\Debug\net10.0-windows10.0.22621.0\MemoView.exe`

Two scripts in `client\apps\webview-app\memoview-win` do this build from any PowerShell. They
find `msbuild` with `vswhere` when it is not on the path:

```powershell
.\debug.ps1      # the Debug build; ends a running MemoView and starts the new one
.\release.ps1    # the Release build
```

`debug.sh` and `release.sh` do the same, for Git Bash.

## Run

1. Start the journal server first. MemoView connects to `http://localhost:8087` (REST API and SSE). See `BUILD.md` at the root of the repository.
2. Run `bin\x64\Debug\net10.0-windows10.0.22621.0\MemoView.exe`

## Test

The tests are in `client\apps\webview-app\memoview-win\memoview-tests`. They need only the .NET 10 SDK.

```powershell
cd client\apps\webview-app\memoview-win\memoview-tests
dotnet test
```

Run `dotnet test` in this folder, not at the root of the repository.
The `global.json` in this folder selects the test runner, and the SDK finds it only from the current folder upward.

To measure the coverage of the lines too, run this in the same folder:

```powershell
dotnet tool restore
dotnet build -t:Coverage
```

It runs the tests with coverlet.console, a local tool of `memoview-win`, and prints a table. It writes `coverage\coverage.cobertura.xml`.
It fails when less than 90% of the lines of the app files are covered, and the CI runs it.
A line no test reaches is either removed, or given a test of the behaviour that needs it.
Only the files of the tests are left out. `MemoView.Tests.csproj` tells why the tool is coverlet.console.

The tests cover the code that does not use WinUI: the rows of the items, the state of a task, the addresses of links and images, the page document, the messages from the page, and the settings file.
They do not start the app. The window has no tests.
The script inside the WebView (`Web\page.js`) does not run in the tests. They only check that it sends the messages the app reads.

## Format

The formatter is [CSharpier](https://csharpier.com). It formats the C#, XAML and project files under `client/apps/webview-app/memoview-win`.
It is a local tool. Its version is in `client/apps/webview-app/memoview-win\dotnet-tools.json`.

```powershell
cd client/apps/webview-app/memoview-win
dotnet tool restore        # once, to install the tool
dotnet csharpier format .  # format the files
dotnet csharpier check .   # only check, as the CI does
```

Format after every change. The CI fails when a file is not formatted.

## Lint

The linter is the set of analyzers in the .NET SDK. There is nothing to install.
The analyzers run in every build, of the app and of the tests. Their findings are warnings of the build.

The settings are in `client/apps/webview-app/memoview-win\Directory.Build.props`.

- `AnalysisLevel` is `latest-recommended`: the rules that Microsoft recommends.
- The null safety warnings of the compiler (CS86xx) are errors. They stop the build.
- All other findings are warnings. They do not stop the build, so read the output of the build.

The tests turn off one rule, CA1707, in `client\apps\webview-app\memoview-win\memoview-tests\.editorconfig`. The file tells why.

## Settings

Window state is persisted at `~/.peta-stickies` (JSON). Delete this file to reset all windows.

The close button of a sticky removes the sticky from this file. It does not come back at the next start.
When the last sticky is closed with the button, the app ends, and the next start opens one new sticky.

A sticky that closes in another way stays in the file and comes back at the next start.
Examples are Alt+F4, "Close window" on the taskbar, and the end of the Windows session.
