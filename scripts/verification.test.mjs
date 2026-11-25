// Guards the settings of the checks. A change that lowers the coverage floor, measures less, or
// takes a check out of the CI or of the hook fails here. Such a change needs the consent of the
// developer, and then a change of the values below too, so that it shows in the diff of this file.
//
// Run with `node --test "scripts/*.test.mjs"`. The `guard` job of scripts/checks.mjs runs it.

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { test } from 'node:test';
import { JOBS, ROOT, commandOf, jobsFor, logFileFor, runJob } from './checks.mjs';

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
  'outliner-ios': './coverage.sh',
};

// The coverage steps that run on one OS only: stickies-mac and outliner-ios build nowhere else, and
// the developer runs stickies-win on Windows only (#1085). The CI runs them on that OS, where a step
// for another OS fails the job
const COVERAGE_OS = {
  'stickies-win': 'win32',
  'stickies-mac': 'darwin',
  'outliner-ios': 'darwin',
};

// The floor and the folder measured by coverage.sh of stickies-mac. App/, generated/ and
// ../swift-shared are left out, with the consent of the developer
const SWIFT = {
  'client/apps/native-app/stickies-mac/coverage.sh': { floor: String(FLOOR), measured: 'Sources/' },
};

// The floor, the folders and the files left out of coverage.sh of outliner-ios, which reads the
// result of xcodebuild. The two files only pass each call on to the SDK of Dropbox, and are left
// out with the consent of the developer (2026-11-25, #1088)
const XCCOV = {
  'client/apps/native-app/outliner-ios/coverage.sh': {
    floor: String(FLOOR),
    measured: 'App/ Sources/',
    excluded: 'Sources/PetaDropbox/SdkDropbox.swift Sources/PetaDropbox/DropboxSdk.swift',
  },
};

// The arguments of coverlet.console in the Coverage target of each C# test project
// The options of each coverlet command in the Coverage target of each C# test project, in order.
// The last command adds up what the commands before it measured, and fails under the floor
const STICKIES_MEASURED = [
  '--include "[PetaBorder]*"',
  '--include "[PetaScreens]*"',
  // The files left out with the consent of the developer (2026-11-25, #1085)
  '--exclude-by-file "**/obj/**"',
  '--exclude-by-file "**/PetaScreens/Launch.cs"',
  '--exclude-by-file "**/PetaScreens/RowView.Drop.cs"',
];
const DOTNET_FLOOR = [`--threshold ${FLOOR}`, '--threshold-type line', '--threshold-stat total'];
const DOTNET = {
  'client/apps/webview-app/memoview-win/memoview-tests/MemoView.Tests.csproj': [
    ['--include-test-assembly', '--exclude-by-file "**/memoview-tests/**"', ...DOTNET_FLOOR],
  ],
  'client/apps/native-app/stickies-win/PetaBorder.Tests/PetaBorder.Tests.csproj': [
    // The tests of the windows
    STICKIES_MEASURED,
    // The tests of PetaBorder, added to those of the windows
    [...STICKIES_MEASURED, '--merge-with coverage/screens.json', ...DOTNET_FLOOR],
  ],
};

// The options of each coverlet command of a csproj, each option with its value
function coverletOptions(project) {
  const targets = [...read(project).matchAll(/<Target Name="Coverage"[\s\S]*?<\/Target>/g)];
  assert.equal(targets.length, 1, `${project}: one Coverage target`);
  return [...targets[0][0].matchAll(/Command="([^"]*)"/g)]
    .map((m) => m[1].replaceAll('&quot;', '"'))
    .filter((command) => command.startsWith('dotnet coverlet '))
    .map((command) =>
      // The first option names the assembly. The others name how to run its tests and where
      // the report goes
      command
        .split(/ (?=--)/)
        .slice(1)
        .filter((o) => !/^--(target|targetargs|format|output) /.test(o)),
    );
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

// カバレッジを測るジョブは、どれもカバレッジの段を持ち、その段はCIでは飛ばされない。
test('each job measures the coverage in a step that the ci never skips', () => {
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

// xcodebuildの結果を読むカバレッジのスクリプトは、9割未満で失敗し、了解したフォルダーを、了解したファイルだけ除いて測る。
test('each xcodebuild coverage script fails under the floor and measures the agreed folders', () => {
  for (const [script, expected] of Object.entries(XCCOV)) {
    const text = read(script);
    const value = (name) => [...text.matchAll(new RegExp(`^${name}=(.*)$`, 'gm'))].map((m) => m[1].replace(/^"|"$/g, ''));
    assert.deepEqual(value('floor'), [expected.floor], script);
    assert.deepEqual(value('measured'), [expected.measured], script);
    assert.deepEqual(value('excluded'), [expected.excluded], script);
    // Each file of those folders is read, and the tests run with the coverage on
    assert.match(text, /for folder in \$measured; do/, script);
    assert.match(text, /-enableCodeCoverage YES/, script);
  }
});

// C#のテストのプロジェクトは、9割未満で失敗し、了解した範囲を測る。
test('each c# test project fails under the floor and measures the agreed files', () => {
  for (const [project, measured] of Object.entries(DOTNET)) {
    const sorted = (commands) => commands.map((options) => [...options].sort());
    assert.deepEqual(sorted(coverletOptions(project)), sorted(measured), project);
  }
});

// CIは、検証の一覧のジョブをすべて回し、一覧にないジョブは回さない。
test('the CI runs every job of the list', () => {
  const ci = read('.github/workflows/ci.yml');
  const named = [...ci.matchAll(/node scripts\/checks\.mjs run ([\w-]+)/g)].map((m) => m[1]);
  const matrix = rustWorkspaces(ci);
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
test('only a step for another os is skipped on a developer machine', async () => {
  const other = process.platform === 'win32' ? 'darwin' : 'win32';
  const quiet = { log: () => {} };
  JOBS['test-other-os'] = { dir: '.', touchedBy: [], steps: [{ os: other, run: 'never' }] };
  JOBS['test-no-variable'] = { dir: '.', touchedBy: [], steps: [{ needs: ['PETA_NO_SUCH_VARIABLE'], run: 'never' }] };
  try {
    const skipped = await runJob('test-other-os', quiet);
    assert.equal(skipped.failed, undefined);
    assert.equal(skipped.skipped.length, 1);
    assert.equal((await runJob('test-other-os', { ...quiet, strict: true })).failed.step, 'never');
    assert.equal((await runJob('test-no-variable', quiet)).failed.step, 'never');
  } finally {
    delete JOBS['test-other-os'];
    delete JOBS['test-no-variable'];
  }
});

// 画面に触れる段は、screen: falseのとき（フック）だけ飛ばす。手で回したときとCIでは回す（ADR 086）。
test('a step on the screen is skipped only when the screen is not to be touched', async () => {
  const quiet = { log: () => {} };
  JOBS['test-screen'] = { dir: '.', touchedBy: [], steps: [{ screen: true, run: 'node -e "process.exit(3)"' }] };
  try {
    const hook = await runJob('test-screen', { ...quiet, screen: false });
    assert.equal(hook.failed, undefined);
    assert.deepEqual(
      hook.skipped.map((s) => s.step),
      ['node -e "process.exit(3)"'],
    );
    assert.notEqual((await runJob('test-screen', quiet)).failed, undefined);
    assert.notEqual((await runJob('test-screen', { ...quiet, strict: true })).failed, undefined);
  } finally {
    delete JOBS['test-screen'];
  }
});

// 画面に触れる段は、stickies-winのカバレッジの段だけである（開発者が決めた、ADR 086）。
test('only the coverage step of stickies-win touches the screen', () => {
  const screen = Object.entries(JOBS).flatMap(([job, { steps }]) => steps.filter((s) => s.screen).map((s) => `${job}: ${s.run}`));
  assert.deepEqual(screen, ['stickies-win: dotnet build -t:Coverage']);
});

// 段の出力は、段が終わるのを待たずに、出たそばからログに渡る。開発者が進み具合を見られるようにするため。
test('the output of a step reaches the log while the step runs', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'peta-stream-'));
  const script = join(dir, 'slow.mjs');
  writeFileSync(script, "console.log('first'); setTimeout(() => console.log('second'), 1500);\n");
  JOBS['test-slow'] = { dir: '.', touchedBy: [], steps: [{ run: `node "${script}"` }] };
  try {
    let firstAt;
    const result = await runJob('test-slow', {
      log: (s) => {
        if (firstAt === undefined && s.includes('first')) firstAt = Date.now();
      },
    });
    const endedAt = Date.now();
    assert.equal(result.failed, undefined);
    assert.ok(endedAt - firstAt >= 1000, `the first line came ${endedAt - firstAt}ms before the end`);
  } finally {
    delete JOBS['test-slow'];
  }
});

// フックのログは、HEADのコミットの名前で、Gitの共通のフォルダーの下に置く。ワークツリーでも同じ場所になる。
test('the log of the hook is named by the commit at HEAD, in the common folder of git', () => {
  const git = (...args) => execFileSync('git', args, { cwd: ROOT, encoding: 'utf8' }).trim();
  const common = resolve(ROOT, git('rev-parse', '--git-common-dir'));
  assert.equal(logFileFor(ROOT), join(common, 'peta-verify', `${git('rev-parse', 'HEAD')}.log`));
});

// 落ちた段は、その出力の終わりを添えて知らせる。
test('a failed step tells the end of its output', async () => {
  JOBS['test-fails'] = { dir: '.', touchedBy: [], steps: [{ run: 'node -e "console.log(41); process.exit(3)"' }] };
  try {
    const { failed } = await runJob('test-fails', { log: () => {} });
    assert.match(failed.tail, /41/);
  } finally {
    delete JOBS['test-fails'];
  }
});

// A machine of another OS that runs a job by its own list, and answers with `answer`
function otherMachine(platform, answer) {
  const ran = [];
  return { ran, remote: { platform, runJob: (name) => (ran.push(name), answer) } };
}

// ほかのOSの段は、そのOSの機械があれば、ジョブごとそこで回す。そこでも飛んだ段だけが、飛ばした段に残る。
test('a step for another os runs with its job on a machine of that os', async () => {
  const quiet = { log: () => {} };
  JOBS['test-two-oses'] = {
    dir: '.',
    touchedBy: [],
    steps: [
      { os: 'far', run: 'far only' },
      { os: 'nowhere', run: 'nowhere only' },
    ],
  };
  try {
    const { ran, remote } = otherMachine('far', { failed: undefined, skipped: [{ step: 'nowhere only', why: 'no' }] });
    const result = await runJob('test-two-oses', { ...quiet, remote });
    assert.deepEqual(ran, ['test-two-oses']);
    assert.equal(result.failed, undefined);
    assert.deepEqual(result.remote, ['far only']);
    assert.deepEqual(
      result.skipped.map((s) => s.step),
      ['nowhere only'],
    );
  } finally {
    delete JOBS['test-two-oses'];
  }
});

// ほかのOSの機械で段が落ちたら、ジョブも落ちる。
test('a step that fails on the machine of another os fails the job', async () => {
  const quiet = { log: () => {} };
  JOBS['test-far'] = { dir: '.', touchedBy: [], steps: [{ os: 'far', run: 'far only' }] };
  try {
    const { remote } = otherMachine('far', { failed: { step: 'far only', tail: 'broken' }, skipped: [] });
    const { failed } = await runJob('test-far', { ...quiet, remote });
    assert.match(failed.step, /far only/);
    assert.equal(failed.tail, 'broken');
  } finally {
    delete JOBS['test-far'];
  }
});

// ほかのOSの機械につながらなければ、機械がないときと同じく飛ばし、そのわけを添える。
test('a step for another os is skipped with the reason when its machine cannot be reached', async () => {
  const quiet = { log: () => {} };
  JOBS['test-far'] = { dir: '.', touchedBy: [], steps: [{ os: 'far', run: 'far only' }] };
  try {
    const { remote } = otherMachine('far', { unreachable: 'away' });
    const result = await runJob('test-far', { ...quiet, remote });
    assert.equal(result.failed, undefined);
    assert.equal(result.skipped.length, 1);
    assert.match(result.skipped[0].why, /away/);
  } finally {
    delete JOBS['test-far'];
  }
});

// ほかのOSの機械は、その機械で回せる段がないジョブでは使わない。
test('a job with no step for the other machine does not use it', async () => {
  const quiet = { log: () => {} };
  JOBS['test-nowhere'] = { dir: '.', touchedBy: [], steps: [{ os: 'nowhere', run: 'nowhere only' }] };
  try {
    const { ran, remote } = otherMachine('far', { failed: undefined, skipped: [] });
    assert.equal((await runJob('test-nowhere', { ...quiet, remote })).skipped.length, 1);
    assert.deepEqual(ran, []);
  } finally {
    delete JOBS['test-nowhere'];
  }
});

// The workspaces that the rust job can run, from the plan job of ci.yml
function rustWorkspaces(ci) {
  return JSON.parse(ci.match(/\n {10}RUST: '(\[[^\]]+\])'\n/)[1]);
}

// The OS of each job in the CI, from the runs-on of its block in ci.yml
function ciOses() {
  const RUNNER = { ubuntu: 'linux', windows: 'win32', macos: 'darwin' };
  const oses = {};
  for (const block of read('.github/workflows/ci.yml').split(/\n(?=  [\w-]+:\n)/)) {
    const os = RUNNER[block.match(/runs-on: (\w+)-/)?.[1]];
    const matrix = block.includes('${{ matrix.workspace }}') ? rustWorkspaces(read('.github/workflows/ci.yml')) : [];
    const named = [...block.matchAll(/node scripts\/checks\.mjs run ([\w-]+)/g)].map((m) => m[1]);
    for (const job of [...named, ...matrix]) oses[job] = os;
  }
  return oses;
}

// CIは、PR、PRへのpush、mainへのpushで回る。docs/の下だけの変更でも起動する。
// 起動しないと、mainの保護が求めるチェックが来ず、PRをマージできないためである（開発者が決めた）。
test('the CI runs on pull requests and on pushes to main, also for docs only', () => {
  const ci = read('.github/workflows/ci.yml');
  const on = ci.split('\non:\n')[1].split('\npermissions:')[0];
  assert.match(on, /\n {2}pull_request:\n {2}push:\n {4}branches: \[main\]\n$/);
});

// planのジョブは、フックと同じ決まり（checks.mjsのjobs）で、回すジョブを選ぶ。
// ciのジョブは、ほかのすべてのジョブを待ち、1つでも失敗か中止があれば失敗する。mainの保護が求めるのは、このチェックだけである。
test('the plan job picks the jobs as the hook does, and the ci job waits for every other job', () => {
  const ci = read('.github/workflows/ci.yml');
  const ids = [...ci.split('\njobs:\n')[1].matchAll(/^ {2}([\w-]+):$/gm)].map((m) => m[1]);
  assert.match(ci, /\| xargs -0 node scripts\/checks\.mjs jobs \|/);
  const needs = ci
    .match(/\n {2}ci:\n {4}if: always\(\)\n {4}needs: \[([^\]]+)\]/)[1]
    .split(',')
    .map((s) => s.trim());
  assert.deepEqual(needs.sort(), ids.filter((id) => id !== 'ci').sort());
  assert.match(
    ci,
    /\n {6}- if: contains\(needs\.\*\.result, 'failure'\) \|\| contains\(needs\.\*\.result, 'cancelled'\)\n {8}run: exit 1\n/,
  );
});

// 手で起こすときは、OSごとに回すかを選べる。既定はすべてのOSを回す（開発者が決めた。publicのリポジトリでは、macOSも無料である）。
// ほかのジョブは、planが選んだときだけ回る。guardは毎回回る。ios_onlyのときは、outliner-iosだけが回る。
test('the CI runs the jobs that the plan picks, each os when asked, only outliner-ios with ios_only, and the guard every time', () => {
  const ci = read('.github/workflows/ci.yml');
  const INPUT = { linux: 'linux', win32: 'windows', darwin: 'macos' };
  const defaults = Object.fromEntries(
    [...ci.matchAll(/\n {6}(\w+):\n {8}description: [^\n]*\n {8}type: boolean\n {8}default: (true|false)(?=\n)/g)].map((m) => [m[1], m[2]]),
  );
  assert.deepEqual(defaults, { linux: 'true', windows: 'true', macos: 'true', ios_only: 'false' });
  const UNPICKED = { plan: undefined, guard: undefined, ci: 'always()' };
  for (const block of ci.split('\njobs:\n')[1].split(/\n(?=  [\w-]+:\n)/)) {
    const name = block.match(/^ {2}([\w-]+):$/m)?.[1];
    if (!name) continue; // a comment before the first job
    const cond = block.match(/\n {4}if: (.*)/)?.[1];
    if (name in UNPICKED) {
      assert.equal(cond, UNPICKED[name], name);
      continue;
    }
    const input = INPUT[{ ubuntu: 'linux', windows: 'win32', macos: 'darwin' }[block.match(/runs-on: (\w+)-/)[1]]];
    const picked = name === 'rust' ? `needs.plan.outputs.rust != '[]'` : `contains(fromJSON(needs.plan.outputs.jobs), '${name}')`;
    const expected =
      name === 'outliner-ios'
        ? `${picked} && (github.event_name != 'workflow_dispatch' || inputs.${input} || inputs.ios_only)`
        : `${picked} && (github.event_name != 'workflow_dispatch' || inputs.${input}) && !inputs.ios_only`;
    assert.equal(cond, expected, name);
    assert.match(block, /\n {4}needs: plan\n/, name);
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

// stickies-winのどの段も、Windowsでしか回らない。Macでは回さない（開発者が決めた、#1085）。
test('every step of stickies-win runs on windows only', () => {
  for (const step of JOBS['stickies-win'].steps) {
    assert.notEqual(commandOf(step, 'win32'), undefined, step.run);
    assert.equal(commandOf(step, 'darwin'), undefined, step.run);
  }
});

// stickies-winの2つのテストのプロジェクトは、どちらもx64のプロセスで回る。コアのDLLはx64で、ARM64のWindowsでも、arm64のプロセスでは読めない（#1175）。
test('both test projects of stickies-win run their tests in an x64 process', () => {
  const dir = 'client/apps/native-app/stickies-win';
  for (const project of ['PetaScreens.Tests', 'PetaBorder.Tests']) {
    assert.match(read(`${dir}/${project}/${project}.csproj`), /<RuntimeIdentifier>win-x64<\/RuntimeIdentifier>/, project);
  }
  // coverlet starts the .exe of each project: `dotnet` would start the process of the machine
  const targets = [...read(`${dir}/PetaBorder.Tests/PetaBorder.Tests.csproj`).matchAll(/--target &quot;([^&]*)&quot;/g)];
  assert.deepEqual(
    targets.map((m) => m[1]),
    ['$(ScreensTests).exe', '$(BorderTests).exe'],
  );
});

// コードに、カバレッジの測定から行を外す印がない。
test('no code leaves lines out of the coverage with a comment or an attribute', () => {
  const marks =
    /\b(v8|c8|istanbul) ignore\b|LCOV_EXCL|cfg(_attr)?\((not\()?coverage|coverage\(off\)|coverage_nightly|ExcludeFromCodeCoverage/;
  const files = execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' })
    .split('\n')
    .filter((f) => /\.(rs|ts|tsx|js|mjs|cs|swift|kt|el)$/.test(f) && f !== 'scripts/verification.test.mjs');
  const found = files.filter((f) => marks.test(read(f)));
  assert.deepEqual(found, []);
});
