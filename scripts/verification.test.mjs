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
import { JOBS, ROOT } from './checks.mjs';

const read = (path) => readFileSync(join(ROOT, path), 'utf8');

const FLOOR = 90;

// The files left out of the measurement, with the consent of the developer. The reasons are in the
// comments of each setting.
const RUST = {
  journal:
    'src[/\\\\]main\\.rs$|application[/\\\\]folder_chooser\\.rs$|infrastructure[/\\\\]launcher\\.rs$|infrastructure[/\\\\]pending_delete_prompt\\.rs$|[/\\\\]atlas[/\\\\]',
  client: '[/\\\\](journal|atlas)[/\\\\]',
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
};

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
    assert.equal(step.os, undefined, job);
    assert.equal(step.tools, undefined, job);
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

// コードに、カバレッジの測定から行を外す印がない。
test('no code leaves lines out of the coverage with a comment or an attribute', () => {
  const marks = /\b(v8|c8|istanbul) ignore\b|LCOV_EXCL|cfg(_attr)?\((not\()?coverage|coverage\(off\)|coverage_nightly/;
  const files = execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' })
    .split('\n')
    .filter((f) => /\.(rs|ts|tsx|js|mjs|cs|swift|kt|el)$/.test(f) && f !== 'scripts/verification.test.mjs');
  const found = files.filter((f) => marks.test(read(f)));
  assert.deepEqual(found, []);
});
