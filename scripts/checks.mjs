// The checks of each CI job, in one place. The CI runs them with `node scripts/checks.mjs run <job>`,
// after the job has installed its tools. The hook before a pull request (.claude/hooks) runs the
// same checks on a developer's machine, for the jobs that the changed files touch.
//
// Usage:
//   node scripts/checks.mjs run <job>          runs the checks of the job
//   node scripts/checks.mjs run <job> --result also prints the result as one line of JSON
//   node scripts/checks.mjs jobs <file>...     prints the jobs that the files touch
//   node scripts/checks.mjs log                prints the log that the hook writes for HEAD
//
// Only a step for another OS (`os`) is skipped on a developer's machine, and reported. In the CI
// (CI=true), it fails the job, so that no check is skipped there without a sign. Run by hand on
// Windows, a step for macOS runs on the Mac of the developer when there is one (scripts/mac.mjs).
// Everything else that a step needs can be set up on any machine: a program given by an
// environment variable (`needs`), a program on the PATH, a file (`programs`). Without it, the step
// fails everywhere.
//
// The hook skips more (ADR 086): it uses no other machine, and it skips a step that touches the
// screen (`screen: true`), as the developer could not use the PC meanwhile. The required `ci`
// check of the CI runs them all.

import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { arch } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { RESULT, macOf } from './mac.mjs';

export const ROOT = resolve(fileURLToPath(import.meta.url), '..', '..');

const EXE = process.platform === 'win32' ? '.exe' : '';

// Rust workspaces, by the workspaces they use through path dependencies
const JOURNAL = ['journal/'];
const ATLAS = ['atlas/', ...JOURNAL];
const CLIENT_RUST = ['client/', ...ATLAS];
// Files that change every job
const ALL = ['rust-toolchain.toml', '.github/workflows/ci.yml', 'scripts/checks.mjs'];

// The coverage of the aarch64 Rust of Windows does not work: llvm-profdata takes no .profraw that
// it writes ("malformed instrumentation profile data: symbol name is empty", Rust 1.100.0, even for
// a crate of one function). The x86_64 Rust of the same version measures under emulation, so an
// ARM64 PC measures with it. A RUSTUP_TOOLCHAIN that is set already is kept
function covEnv() {
  if (process.platform !== 'win32' || arch() !== 'arm64' || process.env.RUSTUP_TOOLCHAIN) return undefined;
  const channel = readFileSync(join(ROOT, 'rust-toolchain.toml'), 'utf8').match(/^channel = "([^"]+)"/m)?.[1];
  if (!channel) throw new Error('rust-toolchain.toml has no channel');
  return { RUSTUP_TOOLCHAIN: `${channel}-x86_64-pc-windows-msvc` };
}

const rust = (args) => [
  { run: 'cargo fmt --check' },
  { run: `cargo clippy --all-targets ${args}` },
  { run: `cargo test ${args}` },
  // Fails below 90% of the lines. The `cov` alias in .cargo/config.toml holds the crates and the floor
  { run: 'cargo cov --summary-only', env: covEnv() },
];

const emacs = '"$PETA_EMACS"';

export const JOBS = {
  // The settings of the checks themselves: the coverage floors, the files left out, the jobs of the
  // CI and the hook (scripts/verification.test.mjs). Any file can change them, and the run is short
  guard: {
    dir: '.',
    touchedBy: [''],
    steps: [{ run: 'node --test "scripts/*.test.mjs"' }],
  },
  journal: {
    dir: 'journal',
    touchedBy: JOURNAL,
    steps: [
      ...rust('--workspace'),
      // The apps on a phone build the core without the extensions, a path the server never takes (ADR 058)
      { run: 'cargo test -p journal-core --no-default-features' },
    ],
  },
  atlas: {
    dir: 'atlas',
    touchedBy: ATLAS,
    steps: rust('--all-features'),
  },
  client: {
    dir: 'client',
    touchedBy: CLIENT_RUST,
    // The web apps have jobs of their own
    notTouchedBy: ['client/apps/web-app/', 'client/apps/webview-app/'],
    steps: [
      ...rust('--workspace'),
      // What only a native shell needs is behind features: the generation of its types, the
      // examples of the values of its border, and the entrance for another language
      {
        run: 'cargo clippy -p client-native-bridge -p client-shell-http --features client-native-bridge/facet_typegen,client-shell-http/ffi --all-targets',
      },
      {
        run: 'cargo test -p client-native-bridge -p client-shell-http --features client-native-bridge/facet_typegen,client-shell-http/ffi --test codegen --test border_examples --test ffi',
      },
      // The browser side of client-web-bridge (src/web.rs) is built for wasm only
      { run: 'cargo clippy --target wasm32-unknown-unknown -p client-web-bridge --lib' },
      // The Emacs mode. These tests need no process. The E2E tests are in the emacs-e2e job
      {
        cwd: 'client/apps/emacs/outliner',
        needs: ['PETA_EMACS'],
        run: `${emacs} -Q --batch -L lisp -l lisp/test/peta-journal-test.el -f ert-run-tests-batch-and-exit`,
      },
      {
        cwd: 'client/apps/emacs/outliner',
        needs: ['PETA_EMACS', 'ELISP_FORMATTER'],
        run: `${emacs} -Q --batch -l "$ELISP_FORMATTER" -f elisp-formatter-check lisp`,
      },
    ],
  },
  // The E2E tests of the Emacs mode press real keys, with the real shell and a real server
  'emacs-e2e': {
    dir: '.',
    touchedBy: CLIENT_RUST,
    notTouchedBy: ['client/apps/web-app/', 'client/apps/webview-app/'],
    steps: [
      { cwd: 'journal', run: 'cargo build -p journal-server' },
      { cwd: 'client', run: 'cargo build -p outliner-emacs' },
      // The tests skip themselves when a program is missing, and the job would pass with no test run
      {
        cwd: 'client/apps/emacs/outliner',
        needs: ['PETA_EMACS'],
        programs: [`journal/target/debug/journal-server${EXE}`, `client/target/debug/outliner-emacs${EXE}`],
        env: { PETA_E2E_SERVER: join(ROOT, 'journal', 'target', 'debug', `journal-server${EXE}`) },
        run: `${emacs} -Q --batch -L lisp -L lisp/test -l lisp/test/peta-journal-e2e.el -f ert-run-tests-batch-and-exit`,
      },
    ],
  },
  todo: {
    dir: 'client/apps/web-app/todo',
    touchedBy: ['client/apps/web-app/todo/'],
    steps: [
      { run: 'npm ci' },
      { run: 'npm run format:check' },
      { run: 'npm run lint' },
      // Runs the tests, and fails below 90% of the lines covered (vitest.config.ts)
      { run: 'npm run coverage' },
      { run: 'npm run build' },
    ],
  },
  // The web shell in React. Its core is the wasm of client-web-bridge. The tests, the lint and the
  // types need that wasm and the types of the border, which `npm run border` makes from the Rust code.
  // The e2e tests with Playwright run on a developer's machine only.
  'outliner-react': {
    dir: 'client/apps/web-app/outliner-react',
    touchedBy: ['client/apps/web-app/outliner-react/', ...CLIENT_RUST],
    notTouchedBy: ['client/apps/web-app/todo/', 'client/apps/webview-app/', 'client/apps/emacs/'],
    steps: [
      { run: 'npm ci' },
      { run: 'npm run border' },
      { run: 'npm run format:check' },
      { run: 'npm run lint' },
      // Runs the tests, and fails below 90% of the lines covered (vitest.config.ts)
      { run: 'npm run coverage' },
      // `--ignore-scripts`: the step above built the border, and `prebuild` would build it again
      { run: 'npm run build --ignore-scripts' },
    ],
  },
  'memoview-win': {
    dir: 'client/apps/webview-app/memoview-win',
    touchedBy: ['client/apps/webview-app/memoview-win/'],
    steps: [
      // CSharpier is a local tool of memoview-win (dotnet-tools.json)
      { run: 'dotnet tool restore' },
      { run: 'dotnet csharpier check .' },
      // Runs the tests, and fails below 90% of the lines covered (the Coverage target of
      // MemoView.Tests.csproj). Why in this folder: its global.json selects the test runner, and
      // the SDK finds it only from the current folder upward. The analyzers run in both builds
      { cwd: 'client/apps/webview-app/memoview-win/memoview-tests', run: 'dotnet build -t:Coverage' },
      // The tests compile only the files without WinUI. The build shows what breaks in the rest.
      // WinUI builds only on Windows, with the MSBuild of Visual Studio
      {
        os: 'win32',
        needs: ['MSBUILD'],
        run: '"$MSBUILD" memoview/MemoView.csproj -restore -p:Platform=x64 -nologo -v:m',
      },
    ],
  },
  // The sticky notes on Windows. PetaBorder, the C# side of the border, is tested without WinUI.
  // The tests of the windows (PetaScreens.Tests) press real keys. They run in the coverage step,
  // on the Windows runner of the CI too. Every step is for Windows. PetaBorder and its tests could
  // run on a Mac, but the developer decided not to run them there (#1085): the app runs on Windows
  // only.
  'stickies-win': {
    dir: 'client/apps/native-app/stickies-win',
    touchedBy: ['client/apps/native-app/stickies-win/', ...CLIENT_RUST],
    notTouchedBy: [
      'client/apps/web-app/',
      'client/apps/webview-app/',
      'client/apps/emacs/',
      'client/apps/native-app/outliner-android/',
      'client/apps/native-app/outliner-ios/',
      'client/apps/native-app/stickies-mac/',
      'client/apps/native-app/swift-shared/',
    ],
    steps: [
      // generated/: the C# types of the border and the core as a library, from the Rust code
      { os: 'win32', run: 'pwsh -NoProfile -File generate.ps1' },
      // Some tests of PetaBorder talk to a real journal server
      { os: 'win32', cwd: 'journal', run: 'cargo build -p journal-server' },
      // CSharpier is a local tool of stickies-win (dotnet-tools.json)
      { os: 'win32', run: 'dotnet tool restore' },
      { os: 'win32', run: 'dotnet csharpier check .' },
      // Runs the tests, and fails below 90% of the lines covered (the Coverage target of
      // PetaBorder.Tests.csproj). The analyzers run in the build. The tests of the windows press
      // real keys, so the hook leaves this step to the CI (ADR 086)
      {
        os: 'win32',
        screen: true,
        cwd: 'client/apps/native-app/stickies-win/PetaBorder.Tests',
        programs: [`journal/target/debug/journal-server${EXE}`],
        env: { PETA_E2E_SERVER: join(ROOT, 'journal', 'target', 'debug', `journal-server${EXE}`) },
        run: 'dotnet build -t:Coverage',
      },
      // The app itself. Its entry point is left out of the coverage, and no test builds it, so
      // only this step shows what breaks in it. WinUI builds only on Windows
      { os: 'win32', run: 'dotnet build PetaStickies -nologo -v:q' },
    ],
  },
  // The sticky notes on macOS, in SwiftUI. Everything here needs Xcode, so every step is for macOS:
  // elsewhere the hook skips them and says so. The tests that put the views in windows run here too
  'stickies-mac': {
    dir: 'client/apps/native-app/stickies-mac',
    touchedBy: ['client/apps/native-app/stickies-mac/', 'client/apps/native-app/swift-shared/', ...CLIENT_RUST],
    notTouchedBy: [
      'client/apps/web-app/',
      'client/apps/webview-app/',
      'client/apps/emacs/',
      'client/apps/native-app/outliner-android/',
      'client/apps/native-app/outliner-ios/',
      'client/apps/native-app/stickies-win/',
    ],
    steps: [
      // generated/ and ../swift-shared/generated/: the core as a library and the types of the
      // border, from the Rust code
      { os: 'darwin', run: './generate.sh' },
      // swift-format checks the Swift of both Apple shells and of swift-shared at once
      { os: 'darwin', cwd: 'client/apps/native-app', run: './swift-format.sh --check' },
      // Builds journal-server, runs the tests, and fails below 90% of the lines covered. The files
      // left out are in the comment of coverage.sh
      { os: 'darwin', run: './coverage.sh' },
      // The tests of swift-shared, among them the common tests (ADR 089). `swift test` here does
      // not run the tests of a package it depends on
      { os: 'darwin', cwd: 'client/apps/native-app/swift-shared', run: 'swift test' },
    ],
  },
  // The outliner on iOS, in SwiftUI. Everything here needs Xcode and the simulator of an iPhone, so
  // every step is for macOS. The tests of the packages, of the app and of the screen all run in the
  // simulator, in the coverage step
  'outliner-ios': {
    dir: 'client/apps/native-app/outliner-ios',
    touchedBy: ['client/apps/native-app/outliner-ios/', 'client/apps/native-app/swift-shared/', ...CLIENT_RUST],
    notTouchedBy: [
      'client/apps/web-app/',
      'client/apps/webview-app/',
      'client/apps/emacs/',
      'client/apps/native-app/outliner-android/',
      'client/apps/native-app/stickies-mac/',
      'client/apps/native-app/stickies-win/',
    ],
    steps: [
      // generated/ and ../swift-shared/generated/: the core as a library for the phone, the
      // simulator and the Mac, and the types of the border, from the Rust code
      { os: 'darwin', run: './generate.sh' },
      // swift-format checks the Swift of both Apple shells and of swift-shared at once
      { os: 'darwin', cwd: 'client/apps/native-app', run: './swift-format.sh --check' },
      // Runs every test in the simulator, and fails below 90% of the lines covered. The files left
      // out are in the comment of coverage.sh
      { os: 'darwin', run: './coverage.sh' },
      // The tests of swift-shared, among them the common tests (ADR 089). The scheme of the app
      // does not run them; they need no simulator
      { os: 'darwin', cwd: 'client/apps/native-app/swift-shared', run: 'swift test' },
    ],
  },
  // The outliner on Android, in Kotlin with Compose. The steps are shell scripts, so they are for
  // Linux, the OS of its runner in the CI: elsewhere the hook skips them and says so. They need
  // ANDROID_HOME, a JDK 21 and BoltFFI (README.md of the module)
  'outliner-android': {
    dir: 'client/apps/native-app/outliner-android',
    touchedBy: ['client/apps/native-app/outliner-android/', ...CLIENT_RUST],
    notTouchedBy: [
      'client/apps/web-app/',
      'client/apps/webview-app/',
      'client/apps/emacs/',
      'client/apps/native-app/outliner-ios/',
      'client/apps/native-app/stickies-mac/',
      'client/apps/native-app/stickies-win/',
      'client/apps/native-app/swift-shared/',
    ],
    steps: [
      // generated/: the Kotlin types of the border and the core as a library for each kind of
      // device, from the Rust code
      { os: 'linux', needs: ['ANDROID_NDK_HOME'], run: './generate.sh' },
      { os: 'linux', run: './gradlew ktfmtCheck' },
      // The tests of the JVM, among them the common tests (ADR 089). They need no device
      { os: 'linux', run: './gradlew :app:testDebugUnitTest' },
      // The app and the tests of the screen, built as they are without the coverage. The hook
      // does not run the step below, so only this step shows it what breaks in them
      { os: 'linux', run: './gradlew :app:assembleDebug :app:assembleDebugAndroidTest' },
      // Runs every test, those of the screen in an emulator without a window, and fails below 90%
      // of the lines covered (the coverage task of app/build.gradle.kts). The emulator is heavy,
      // so the hook leaves this step to the CI (#1205)
      { os: 'linux', screen: true, run: './coverage.sh' },
    ],
  },
};

// The jobs that the changed files touch. Every kind of file counts, Markdown too: a test can read
// a Markdown file as its data, and the name of a file does not tell which ones it reads. Only the
// files under docs/ touch no job, as the developer decided: no check reads them.
const NO_JOB = 'docs/';
export function jobsFor(files) {
  const checked = files.filter((f) => !f.startsWith(NO_JOB));
  return Object.keys(JOBS).filter((name) => {
    const { touchedBy, notTouchedBy = [] } = JOBS[name];
    return checked.some(
      (f) => ALL.includes(f) || (touchedBy.some((p) => f.startsWith(p)) && !notTouchedBy.some((p) => f.startsWith(p))),
    );
  });
}

// The MSBuild of Visual Studio is not on the PATH of a developer's machine. vswhere finds it
function findMsbuild() {
  if (process.env.MSBUILD) return process.env.MSBUILD;
  const onPath = spawnSync('msbuild -version', { stdio: 'ignore', shell: true });
  if (onPath.status === 0) return 'msbuild';
  const vswhere = 'C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe';
  if (process.platform !== 'win32' || !existsSync(vswhere)) return undefined;
  const found = spawnSync(vswhere, ['-latest', '-requires', 'Microsoft.Component.MSBuild', '-find', 'MSBuild\\**\\Bin\\MSBuild.exe'], {
    encoding: 'utf8',
  });
  return found.stdout.split(/\r?\n/).find((l) => l.trim())?.trim();
}

// The command of a step on an OS, or undefined when the step is for another OS
export function commandOf(step, platform = process.platform) {
  return step.os && step.os !== platform ? undefined : step.run;
}

// Why a step is for another OS, or undefined when it is for this one
function otherOs(step) {
  return commandOf(step) === undefined ? `${step.os}でしか動かない` : undefined;
}

// The other machine that runs the steps for its OS, for a checkout at `root`: the Mac of the
// developer, on another OS than macOS. Undefined when there is none
export function remoteFor(root = ROOT) {
  return process.platform === 'darwin' ? undefined : macOf(root);
}

// The log that the hook before a pull request writes for the commit at HEAD of `root`. The
// marks of the jobs that passed are in the same folder. The AI prints this path before the hook
// runs, so that the developer can watch the log
export function logFileFor(root = ROOT) {
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
  return join(resolve(root, git('rev-parse', '--git-common-dir')), 'peta-verify', `${git('rev-parse', 'HEAD')}.log`);
}

// Runs a command in the shell of this OS. `log` gets the output as it comes. Resolves to
// { status, out }, with all of the output
export function runCommand(command, { cwd, env, log }) {
  // The shell is called by hand: `shell: true` warns that the command is not escaped (DEP0190)
  const [shell, args] =
    process.platform === 'win32'
      ? [env.ComSpec ?? 'cmd.exe', ['/d', '/s', '/c', `"${command}"`]]
      : ['/bin/sh', ['-c', command]];
  return streamed(shell, args, { cwd, env, windowsVerbatimArguments: process.platform === 'win32' }, log);
}

// Runs a program, and hands its output to `log` as it comes. Resolves to { status, out }
export function streamed(program, args, options, log) {
  return new Promise((done) => {
    let out = '';
    const child = spawn(program, args, { ...options, stdio: ['ignore', 'pipe', 'pipe'] });
    const take = (chunk) => {
      out += chunk;
      log(chunk);
    };
    // The decoder of a stream keeps a letter whole when its bytes come in two chunks
    child.stdout.setEncoding('utf8').on('data', take);
    child.stderr.setEncoding('utf8').on('data', take);
    child.on('error', (e) => take(String(e)));
    child.on('close', (status) => done({ status: status ?? 1, out }));
  });
}

// Runs the steps of a job. `log` gets the output as it comes. Resolves to
// { failed: {step, tail} | undefined, skipped: [{step, why}], remote: [step] }
// The steps that this OS cannot run and `remote` can run are run there, with the whole job: the
// other machine runs the job by its own list of checks, from its first step. `remote` lists them.
// With `screen: false`, a step that touches the screen is skipped too
export async function runJob(
  name,
  { root = ROOT, strict = false, remote, screen = true, log = (s) => process.stdout.write(s) } = {},
) {
  const here = await runHere(name, { root, strict, screen, log });
  const forRemote = (s) => remote && commandOf(s.of, remote.platform) !== undefined;
  const result = { failed: here.failed, skipped: here.skipped.map(({ step, why }) => ({ step, why })), remote: [] };
  if (here.failed || !here.skipped.some(forRemote)) return result;
  log(`\n### ${name}（${remote.platform}）\n`);
  const there = await remote.runJob(name, { log });
  if (there.unreachable) {
    result.skipped = here.skipped.map((s) => ({ step: s.step, why: forRemote(s) ? `${s.why}。${there.unreachable}` : s.why }));
    return result;
  }
  if (there.failed) return { ...result, failed: { step: `${there.failed.step}（${remote.platform}）`, tail: there.failed.tail } };
  const left = new Set(there.skipped.map((s) => s.step));
  return {
    failed: undefined,
    skipped: result.skipped.filter((s) => left.has(s.step)),
    remote: here.skipped.filter((s) => !left.has(s.step)).map((s) => s.step),
  };
}

// Runs the steps of a job that this OS can run. The skipped ones keep their step in `of`
async function runHere(name, { root, strict, screen, log }) {
  const job = JOBS[name];
  if (!job) throw new Error(`unknown job: ${name}`);
  const env = { ...process.env };
  if (!env.MSBUILD) {
    const msbuild = findMsbuild();
    if (msbuild) env.MSBUILD = msbuild;
  }
  const skipped = [];
  for (const step of job.steps) {
    if (step.screen && !screen) {
      skipped.push({ step: step.run, why: '画面に触れる', of: step });
      continue;
    }
    const why = otherOs(step);
    if (why) {
      if (strict) return { failed: { step: step.run, tail: why }, skipped };
      skipped.push({ step: step.run, why, of: step });
      continue;
    }
    const cwd = join(root, step.cwd ?? job.dir);
    const unset = (step.needs ?? []).find((v) => !env[v]);
    if (unset) return { failed: { step: step.run, tail: `環境変数${unset}がない` }, skipped };
    const absent = (step.programs ?? []).find((p) => !existsSync(join(root, p)));
    if (absent) return { failed: { step: step.run, tail: `${absent}がない` }, skipped };
    log(`\n$ (${step.cwd ?? job.dir}) ${step.run}\n`);
    // The variables are put in here, not by the shell: cmd.exe on Windows does not read `$NAME`
    const command = step.run.replace(/\$([A-Z_]+)/g, (_, v) => env[v] ?? '');
    const { status, out } = await runCommand(command, { cwd, env: { ...env, ...step.env }, log });
    if (status !== 0) {
      return { failed: { step: step.run, tail: out.trimEnd().split('\n').slice(-40).join('\n') }, skipped };
    }
  }
  return { failed: undefined, skipped };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [command, ...args] = process.argv.slice(2);
  const result = args.includes('--result');
  const names = args.filter((a) => a !== '--result');
  if (command === 'run' && names.length === 1) {
    const strict = process.env.CI === 'true';
    const outcome = await runJob(names[0], { strict, remote: strict ? undefined : remoteFor() });
    const { failed, skipped } = outcome;
    for (const s of outcome.remote) console.log(`on the other machine: ${s}`);
    for (const s of skipped) console.log(`skipped: ${s.step} (${s.why})`);
    // The hook on another machine reads this line, when it ran the job here
    if (result) console.log(`${RESULT}${JSON.stringify({ failed, skipped })}`);
    if (failed) {
      console.error(`failed: ${failed.step}\n${failed.tail}`);
      process.exit(1);
    }
  } else if (command === 'jobs') {
    console.log(jobsFor(args).join('\n'));
  } else if (command === 'log' && args.length === 0) {
    console.log(logFileFor());
  } else {
    console.error('usage: node scripts/checks.mjs run <job> | jobs <file>... | log');
    process.exit(2);
  }
}
