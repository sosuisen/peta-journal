// Guards the settings of the checks. A change that lowers the coverage floor, measures less, or
// takes a check out of the CI or of the hook fails here. Such a change needs the consent of the
// developer, and then a change of the values below too, so that it shows in the diff of this file.
//
// Run with `node --test "scripts/*.test.mjs"`. The `guard` job of scripts/checks.mjs runs it.

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { JOBS, ROOT, commandOf, jobsFor, runJob } from './checks.mjs';

const read = (path) => readFileSync(join(ROOT, path), 'utf8');

const FLOOR = 90;

// The files left out of the measurement, with the consent of the developer. The reasons are in the
// comments of each setting.
const RUST = {
  journal:
    'src[/\\\\]main\\.rs$|application[/\\\\]folder_chooser\\.rs$|infrastructure[/\\\\]launcher\\.rs$|infrastructure[/\\\\]pending_delete_prompt\\.rs$|[/\\\\]atlas[/\\\\]',
  client: '[/\\\\](journal|atlas)[/\\\\]|app-icon[/\\\\]src[/\\\\]main\\.rs$',
  atlas: '[/\\\\]journal[/\\\\]',
};

const VITEST = {
  'client/apps/web-app/todo': {
    provider: 'v8',
    include: ['src/**/*.{ts,tsx}'],
    exclude: ['src/**/*.test.{ts,tsx}'],
    thresholds: { lines: FLOOR },
  },
  'client/apps/web-app/outliner-react': {
    provider: 'v8',
    include: ['src/**/*.{ts,tsx}'],
    exclude: ['src/**/*.test.{ts,tsx}', 'src/test/**', 'src/border/**', 'src/wasm/**', 'src/layout.ts', 'src/main.tsx'],
    thresholds: { lines: FLOOR },
  },
};

// The step of each job that measures the coverage
const COVERAGE_STEP = {
  journal: 'cargo cov --summary-only',
  client: 'cargo cov --summary-only',
  atlas: 'cargo cov --summary-only',
  todo: 'npm run coverage',
  'outliner-react': 'npm run coverage',
  'memoview-win': 'dotnet build -t:Coverage',
  'stickies-win': 'dotnet build -t:Coverage',
  'stickies-mac': './coverage.sh',
};

// The coverage steps that run on one OS only, as the module builds nowhere else. The CI runs them
// on that OS, where a step for another OS fails the job
const COVERAGE_OS = {
  'stickies-mac': 'darwin',
};

// The floor and the folder measured by coverage.sh of stickies-mac. App/, generated/ and
// ../swift-shared are left out, with the consent of the developer
const SWIFT = {
  'client/apps/native-app/stickies-mac/coverage.sh': { floor: String(FLOOR), measured: 'Sources/' },
};

// The arguments of coverlet.console in the Coverage target of each C# test project
const DOTNET = {
  'client/apps/webview-app/memoview-win/memoview-tests/MemoView.Tests.csproj': [
    '--include-test-assembly',
    '--exclude-by-file "**/memoview-tests/**"',
  ],
  'client/apps/native-app/stickies-win/PetaBorder.Tests/PetaBorder.Tests.csproj': ['--include "[PetaBorder]*"'],
};
const DOTNET_FLOOR = [`--threshold ${FLOOR}`, '--threshold-type line', '--threshold-stat total'];

// The options of the coverlet command of a csproj, each with its value
function coverletOptions(project) {
  const targets = [...read(project).matchAll(/<Target Name="Coverage"[\s\S]*?<\/Target>/g)];
  assert.equal(targets.length, 1, `${project}: one Coverage target`);
  const command = targets[0][0].match(/Command="([^"]*)"/)[1].replaceAll('&quot;', '"');
  assert.ok(command.startsWith('dotnet coverlet '), project);
  // The first two options name the assembly and how to run its tests
  return command.split(/ (?=--)/).slice(1).filter((o) => !/^--(target|targetargs|format|output) /.test(o));
}

// The strings of the `cov` alias in .cargo/config.toml, unescaped
function covAlias(workspace) {
  const text = read(`${workspace}/.cargo/config.toml`);
  const block = text.match(/^cov = \[([\s\S]*?)^\]/m);
  assert.ok(block, `${workspace}: no cov alias`);
  return [...block[1].matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => m[1].replace(/\\(.)/g, '$1'));
}

const valueAfter = (args, flag) => {
  const i = args.indexOf(flag);
  return i < 0 ? undefined : args[i + 1];
};

// The package names of the members of a Rust workspace
function members(workspace) {
  const text = read(`${workspace}/Cargo.toml`);
  const list = text.match(/^members = \[([\s\S]*?)^\]/m);
  if (!list) return undefined;
  return [...list[1].matchAll(/^\s*"([^"]+)"/gm)].map(
    (m) => read(`${workspace}/${m[1]}/Cargo.toml`).match(/^name = "([^"]+)"/m)[1],
  );
}

// The `coverage` object of a vitest.config.ts. It holds only literals, so it is read as a value
function vitestCoverage(app) {
  const text = read(`${app}/vitest.config.ts`).replace(/^\s*\/\/.*$/gm, '');
  const start = text.indexOf('coverage: {');
  assert.ok(start >= 0, `${app}: no coverage settings`);
  assert.equal(text.indexOf('coverage:', start + 1), -1, `${app}: coverage is set twice`);
  let depth = 0;
  let end = text.indexOf('{', start);
  do {
    if (text[end] === '{') depth++;
    if (text[end] === '}') depth--;
    end++;
  } while (depth > 0);
  const { reporter: _, ...rest } = new Function(`return (${text.slice(text.indexOf('{', start), end)})`)();
  return rest;
}

// Rustの各ワークスペースは、9割未満で失敗し、了解した除外のほかは測る。
test('each rust workspace fails under the floor and leaves out only the agreed files', () => {
  for (const [workspace, ignored] of Object.entries(RUST)) {
    const args = covAlias(workspace);
    assert.equal(valueAfter(args, '--fail-under-lines'), String(FLOOR), workspace);
    assert.equal(valueAfter(args, '--ignore-filename-regex'), ignored, workspace);
    assert.equal(args.filter((a) => a === '--ignore-filename-regex').length, 1, workspace);
    // Without it, some crates were not instrumented on Windows, and the report left them out
    assert.ok(args.includes('--no-rustc-wrapper'), workspace);
  }
});

// Rustのワークスペースのクレートは、すべて測る。
test('every crate of a rust workspace is measured', () => {
  for (const workspace of Object.keys(RUST)) {
    const crates = members(workspace);
    const args = covAlias(workspace);
    const measured = args.flatMap((a, i) => (args[i - 1] === '-p' ? [a] : []));
    if (!crates) {
      // A workspace of one package: the alias measures it all
      assert.deepEqual(measured, [], workspace);
      continue;
    }
    assert.ok(!args.includes('--exclude'), workspace);
    assert.deepEqual([...measured].sort(), [...crates].sort(), workspace);
  }
});

// TypeScriptのアプリは、9割未満で失敗し、了解した除外のほかは測る。
test('each typescript app fails under the floor and leaves out only the agreed files', () => {
  for (const [app, expected] of Object.entries(VITEST)) {
    assert.deepEqual(vitestCoverage(app), expected, app);
    // The floor can be changed from the command line too
    assert.equal(JSON.parse(read(`${app}/package.json`)).scripts.coverage, 'vitest run --coverage', app);
  }
});

// カバレッジを測るジョブは、どれもカバレッジの段を持ち、その段は飛ばされない。
test('each job measures the coverage in a step that is never skipped', () => {
  for (const [job, run] of Object.entries(COVERAGE_STEP)) {
    const step = JOBS[job].steps.find((s) => s.run === run);
    assert.ok(step, `${job}: no step "${run}"`);
    assert.equal(step.needs, undefined, job);
    assert.equal(step.os, COVERAGE_OS[job], job);
    assert.equal(step.tools, undefined, job);
  }
});

// Swiftのカバレッジのスクリプトは、9割未満で失敗し、了解した範囲を測る。
test('each swift coverage script fails under the floor and measures the agreed folder', () => {
  for (const [script, expected] of Object.entries(SWIFT)) {
    const text = read(script);
    const floors = [...text.matchAll(/^floor=(.*)$/gm)].map((m) => m[1]);
    assert.deepEqual(floors, [expected.floor], script);
    const reports = [...text.matchAll(/llvm-cov report [\s\S]*?-instr-profile="[^"]+" (\S+)\)/g)].map((m) => m[1]);
    assert.deepEqual(reports, [expected.measured], script);
    // A filter of llvm-cov would leave files out of that folder
    assert.ok(!/-ignore-filename-regex/.test(text), script);
  }
});

// C#のテストのプロジェクトは、9割未満で失敗し、了解した範囲を測る。
test('each c# test project fails under the floor and measures the agreed files', () => {
  for (const [project, measured] of Object.entries(DOTNET)) {
    assert.deepEqual(coverletOptions(project).sort(), [...DOTNET_FLOOR, ...measured].sort(), project);
  }
});

// CIは、検証の一覧のジョブをすべて回し、一覧にないジョブは回さない。
test('the CI runs every job of the list', () => {
  const ci = read('.github/workflows/ci.yml');
  const named = [...ci.matchAll(/node scripts\/checks\.mjs run ([\w-]+)/g)].map((m) => m[1]);
  const matrix = ci.match(/workspace: \[([^\]]+)\]/)[1].split(',').map((s) => s.trim());
  assert.ok(ci.includes('node scripts/checks.mjs run ${{ matrix.workspace }}'));
  assert.deepEqual([...named, ...matrix].sort(), Object.keys(JOBS).sort());
});

// PRの前のフックが、Claude Codeの設定にある。
test('the hook before a pull request is set', () => {
  const { hooks } = JSON.parse(read('.claude/settings.json'));
  const entry = hooks.PreToolUse.find((h) => h.matcher === 'Bash|PowerShell');
  assert.ok(entry);
  assert.ok(entry.hooks.some((h) => h.type === 'command' && h.command.includes('.claude/hooks/verify-before-pr.mjs')));
});

// Markdownのファイルも、ほかのファイルと同じにジョブを回す。テストのデータかもしれない。
// 例外は、開発者が決めたdocs/の下だけである。
test('a markdown file touches the jobs of its folder, except under docs', () => {
  assert.deepEqual(jobsFor(['journal/journal-core/tests/data/page.md']).sort(), jobsFor(['journal/journal-core/src/lib.rs']).sort());
  assert.ok(jobsFor(['README.md']).includes('guard'));
  assert.deepEqual(jobsFor(['docs/plans.md', 'docs/verification.html', 'docs/adr/adr-001.md']), []);
  assert.ok(jobsFor(['docs/plans.md', 'scripts/checks.mjs']).includes('guard'));
});

// 手元で飛ばしてよいのは、ほかのOSの段だけである。環境変数のない段は、手元でも失敗する。
test('only a step for another os is skipped on a developer machine', () => {
  const other = process.platform === 'win32' ? 'darwin' : 'win32';
  const quiet = { log: () => {} };
  JOBS['test-other-os'] = { dir: '.', touchedBy: [], steps: [{ os: other, run: 'never' }] };
  JOBS['test-no-variable'] = { dir: '.', touchedBy: [], steps: [{ needs: ['PETA_NO_SUCH_VARIABLE'], run: 'never' }] };
  try {
    const skipped = runJob('test-other-os', quiet);
    assert.equal(skipped.failed, undefined);
    assert.equal(skipped.skipped.length, 1);
    assert.equal(runJob('test-other-os', { ...quiet, strict: true }).failed.step, 'never');
    assert.equal(runJob('test-no-variable', quiet).failed.step, 'never');
  } finally {
    delete JOBS['test-other-os'];
    delete JOBS['test-no-variable'];
  }
});

// 1つの段は、OSごとのコマンドを持てる。コマンドのあるOSでは、CIでも飛ばさずに回す。
test('a step with a command for each os runs on those oses', () => {
  const quiet = { log: () => {} };
  JOBS['test-each-os'] = { dir: '.', touchedBy: [], steps: [{ run: { [process.platform]: 'node --version', nowhere: 'never' } }] };
  JOBS['test-no-command'] = { dir: '.', touchedBy: [], steps: [{ run: { nowhere: 'never' } }] };
  try {
    assert.deepEqual(runJob('test-each-os', { ...quiet, strict: true }), { failed: undefined, skipped: [] });
    assert.equal(runJob('test-no-command', quiet).skipped.length, 1);
    assert.equal(runJob('test-no-command', { ...quiet, strict: true }).failed.step, 'never');
  } finally {
    delete JOBS['test-each-os'];
    delete JOBS['test-no-command'];
  }
});

// The OS of each job in the CI, from the runs-on of its block in ci.yml
function ciOses() {
  const RUNNER = { ubuntu: 'linux', windows: 'win32', macos: 'darwin' };
  const oses = {};
  for (const block of read('.github/workflows/ci.yml').split(/\n(?=  [\w-]+:\n)/)) {
    const os = RUNNER[block.match(/runs-on: (\w+)-/)?.[1]];
    const matrix = block.match(/workspace: \[([^\]]+)\]/)?.[1].split(',').map((s) => s.trim()) ?? [];
    const named = [...block.matchAll(/node scripts\/checks\.mjs run ([\w-]+)/g)].map((m) => m[1]);
    for (const job of [...named, ...matrix]) oses[job] = os;
  }
  return oses;
}

// CIは、OSごとに回すかを選べる。既定はLinuxとWindowsを回し、macOSは回さない（開発者が決めた）。
// guardは毎回回る。ほかのジョブは、ランナーのOSの入力だけで決まる。
test('the CI runs each os when asked, and the guard every time', () => {
  const ci = read('.github/workflows/ci.yml');
  const INPUT = { linux: 'linux', win32: 'windows', darwin: 'macos' };
  const defaults = Object.fromEntries(
    [...ci.matchAll(/\n {6}(\w+):\n {8}description: [^\n]*\n {8}type: boolean\n {8}default: (true|false)(?=\n)/g)].map((m) => [m[1], m[2]]),
  );
  assert.deepEqual(defaults, { linux: 'true', windows: 'true', macos: 'false' });
  for (const block of ci.split('\njobs:\n')[1].split(/\n(?=  [\w-]+:\n)/)) {
    const name = block.match(/^ {2}([\w-]+):$/m)?.[1];
    if (!name) continue; // a comment before the first job
    const os = { ubuntu: 'linux', windows: 'win32', macos: 'darwin' }[block.match(/runs-on: (\w+)-/)?.[1]];
    const cond = block.match(/\n {4}if: (.*)/)?.[1];
    assert.equal(cond, name === 'guard' ? undefined : `inputs.${INPUT[os]}`, name);
  }
});

// CIのランナーでは、どのジョブのどの段にも、そのOSのコマンドがある。CIで飛ばす段はない。
test('every step of a job has a command on the os of its runner in the CI', () => {
  const oses = ciOses();
  assert.deepEqual(Object.keys(oses).sort(), Object.keys(JOBS).sort());
  for (const [job, os] of Object.entries(oses)) {
    assert.ok(os, `${job}: no runner`);
    for (const step of JOBS[job].steps) assert.ok(commandOf(step, os) !== undefined, `${job} on ${os}: ${JSON.stringify(step.run)}`);
  }
});

// コードに、カバレッジの測定から行を外す印がない。
test('no code leaves lines out of the coverage with a comment or an attribute', () => {
  const marks = /\b(v8|c8|istanbul) ignore\b|LCOV_EXCL|cfg(_attr)?\((not\()?coverage|coverage\(off\)|coverage_nightly/;
  const files = execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' })
    .split('\n')
    .filter((f) => /\.(rs|ts|tsx|js|mjs|cs|swift|kt|el)$/.test(f) && f !== 'scripts/verification.test.mjs');
  const found = files.filter((f) => marks.test(read(f)));
  assert.deepEqual(found, []);
});
