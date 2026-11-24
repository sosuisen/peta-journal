// The checks of each CI job, in one place. The CI runs them with `node scripts/checks.mjs run <job>`,
// after the job has installed its tools. The hook before a pull request (.claude/hooks) runs the
// same checks on a developer's machine, for the jobs that the changed files touch.
//
// Usage:
//   node scripts/checks.mjs run <job>          runs the checks of the job
//   node scripts/checks.mjs jobs <file>...     prints the jobs that the files touch
//
// A step can need something that a machine cannot have: another OS, or a program given by an
// environment variable. On a developer's machine, such a step is skipped and reported. In the CI
// (CI=true), a missing need fails the job, so that no check is skipped there without a sign.
// A tool that can be installed on any OS, such as dotnet, is not such a need. Without it the
// step fails, on a developer's machine too.

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(fileURLToPath(import.meta.url), '..', '..');

const EXE = process.platform === 'win32' ? '.exe' : '';

// Rust workspaces, by the workspaces they use through path dependencies
const JOURNAL = ['journal/'];
const ATLAS = ['atlas/', ...JOURNAL];
const CLIENT_RUST = ['client/', ...ATLAS];
// Files that change every job
const ALL = ['rust-toolchain.toml', '.github/workflows/ci.yml', 'scripts/checks.mjs'];

const rust = (args) => [
  { run: 'cargo fmt --check' },
  { run: `cargo clippy --all-targets ${args}` },
  { run: `cargo test ${args}` },
  // Fails below 90% of the lines. The `cov` alias in .cargo/config.toml holds the crates and the floor
  { run: 'cargo cov --summary-only' },
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
      // What only a native shell needs is behind features: the generation of its types, and the
      // entrance for another language
      {
        run: 'cargo clippy -p client-native-bridge -p client-shell-http --features client-native-bridge/facet_typegen,client-shell-http/ffi --all-targets',
      },
      {
        run: 'cargo test -p client-native-bridge -p client-shell-http --features client-native-bridge/facet_typegen,client-shell-http/ffi --test codegen --test ffi',
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
  // The tests of the windows (PetaScreens.Tests) press real keys, and run on a developer's
  // machine only. The app builds on Windows only, but PetaBorder and its tests run on a Mac too.
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
      { os: 'darwin', run: './generate.sh' },
      // Some tests of PetaBorder talk to a real journal server
      { cwd: 'journal', run: 'cargo build -p journal-server' },
      // CSharpier is a local tool of stickies-win (dotnet-tools.json)
      { run: 'dotnet tool restore' },
      { run: 'dotnet csharpier check .' },
      // Runs the tests, and fails below 90% of the lines covered (the Coverage target of
      // PetaBorder.Tests.csproj). The analyzers run in the build
      {
        cwd: 'client/apps/native-app/stickies-win/PetaBorder.Tests',
        programs: [`journal/target/debug/journal-server${EXE}`],
        env: { PETA_E2E_SERVER: join(ROOT, 'journal', 'target', 'debug', `journal-server${EXE}`) },
        run: 'dotnet build -t:Coverage',
      },
    ],
  },
};

// The jobs that the changed files touch. Markdown files touch no job: they change no check.
export function jobsFor(files) {
  const code = files.filter((f) => !f.endsWith('.md'));
  return Object.keys(JOBS).filter((name) => {
    const { touchedBy, notTouchedBy = [] } = JOBS[name];
    return code.some(
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

// Why a step cannot run on this machine, or undefined when it can
function missing(step, env) {
  if (step.os && step.os !== process.platform) return `${step.os}でしか動かない`;
  for (const name of step.needs ?? []) {
    if (!env[name]) return `環境変数${name}がない`;
  }
  return undefined;
}

// Runs the steps of a job. `log` gets every line of output. Returns
// { failed: {step, tail} | undefined, skipped: [{step, why}] }
export function runJob(name, { root = ROOT, strict = false, log = (s) => process.stdout.write(s) } = {}) {
  const job = JOBS[name];
  if (!job) throw new Error(`unknown job: ${name}`);
  const env = { ...process.env };
  if (!env.MSBUILD) {
    const msbuild = findMsbuild();
    if (msbuild) env.MSBUILD = msbuild;
  }
  const skipped = [];
  for (const step of job.steps) {
    const why = missing(step, env);
    if (why) {
      if (strict) return { failed: { step: step.run, tail: why }, skipped };
      skipped.push({ step: step.run, why });
      continue;
    }
    const cwd = join(root, step.cwd ?? job.dir);
    const absent = (step.programs ?? []).find((p) => !existsSync(join(root, p)));
    if (absent) return { failed: { step: step.run, tail: `${absent}がない` }, skipped };
    log(`\n$ (${step.cwd ?? job.dir}) ${step.run}\n`);
    // The variables are put in here, not by the shell: cmd.exe on Windows does not read `$NAME`
    const command = step.run.replace(/\$([A-Z_]+)/g, (_, v) => env[v] ?? '');
    // The shell is called by hand: `shell: true` warns that the command is not escaped (DEP0190)
    const [shell, args] =
      process.platform === 'win32'
        ? [env.ComSpec ?? 'cmd.exe', ['/d', '/s', '/c', `"${command}"`]]
        : ['/bin/sh', ['-c', command]];
    const r = spawnSync(shell, args, {
      cwd,
      env: { ...env, ...step.env },
      windowsVerbatimArguments: process.platform === 'win32',
      encoding: 'utf8',
      maxBuffer: 1 << 30,
    });
    const out = `${r.stdout ?? ''}${r.stderr ?? ''}${r.error ? String(r.error) : ''}`;
    log(out);
    if (r.status !== 0) {
      return { failed: { step: step.run, tail: out.trimEnd().split('\n').slice(-40).join('\n') }, skipped };
    }
  }
  return { failed: undefined, skipped };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [command, ...args] = process.argv.slice(2);
  if (command === 'run' && args.length === 1) {
    const { failed, skipped } = runJob(args[0], { strict: process.env.CI === 'true' });
    for (const s of skipped) console.log(`skipped: ${s.step} (${s.why})`);
    if (failed) {
      console.error(`failed: ${failed.step}\n${failed.tail}`);
      process.exit(1);
    }
  } else if (command === 'jobs') {
    console.log(jobsFor(args).join('\n'));
  } else {
    console.error('usage: node scripts/checks.mjs run <job> | jobs <file>...');
    process.exit(2);
  }
}
