// Tests of the hook before a pull request (.claude/hooks/verify-before-pr.mjs). Each test builds
// a small repository with a remote, a checkout on main where the hook runs, and a branch. The
// list of checks there is a stub that writes the jobs it runs to a file.
//
// The hook must use the newer of main and the branch, for every file of the checks.

import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { ROOT } from './checks.mjs';

const HOOK = '.claude/hooks/verify-before-pr.mjs';

// A list of checks with one job per entry of `jobs`. Each run appends the name of the job to RAN
const stub = (jobs) => `import { appendFileSync } from 'node:fs';
export const JOBS = ${JSON.stringify(Object.fromEntries(jobs.map((j) => [j, { dir: '.' }])))};
export const jobsFor = () => Object.keys(JOBS);
export function runJob(name) {
  appendFileSync(process.env.RAN, name + '\\n');
  return { skipped: [] };
}
`;

function git(cwd, ...args) {
  return execFileSync('git', ['-c', 'user.name=test', '-c', 'user.email=test@example.com', ...args], {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  }).trim();
}

function write(dir, path, text) {
  mkdirSync(dirname(join(dir, path)), { recursive: true });
  writeFileSync(join(dir, path), text);
}

// A remote with main, a checkout of main (where Claude Code started) and a branch of it
function repository() {
  const tmp = mkdtempSync(join(tmpdir(), 'peta-hook-'));
  const remote = join(tmp, 'remote.git');
  const main = join(tmp, 'main');
  const branch = join(tmp, 'branch');
  git(tmp, 'init', '-q', '--bare', '-b', 'main', remote);
  git(tmp, 'clone', '-q', remote, main);
  git(main, 'checkout', '-q', '-b', 'main');
  write(main, HOOK, readFileSync(join(ROOT, HOOK), 'utf8'));
  write(main, 'scripts/checks.mjs', stub(['old']));
  write(main, 'app.txt', 'one\n');
  commit(main, 'start');
  git(main, 'push', '-q', 'origin', 'main');
  git(tmp, 'clone', '-q', remote, branch);
  git(branch, 'checkout', '-q', '-b', 'feature');
  write(branch, 'app.txt', 'two\n');
  commit(branch, 'change the app');
  return { tmp, main, branch, ran: join(tmp, 'ran.txt') };
}

function commit(dir, message) {
  git(dir, 'add', '-A');
  git(dir, 'commit', '-q', '-m', message);
}

// Runs the hook of main for `gh pr create` in the branch. Returns the answer of the hook
function runHook({ main, branch, ran }) {
  const r = spawnSync(process.execPath, [join(main, HOOK)], {
    input: JSON.stringify({ tool_input: { command: 'gh pr create' }, cwd: branch }),
    encoding: 'utf8',
    env: { ...process.env, RAN: ran, PETA_VERIFY_HANDED: '' },
  });
  assert.equal(r.status, 0, r.stderr);
  const out = r.stdout.trim() ? JSON.parse(r.stdout).hookSpecificOutput : {};
  return { ...out, ran: existsSync(ran) ? readFileSync(ran, 'utf8').split('\n').filter(Boolean) : [] };
}

// mainが検証のファイルをあとで変えたら、ブランチに取り込むまでPRを止める。
test('the hook stops when main has newer checks', () => {
  const repo = repository();
  write(repo.main, 'scripts/checks.mjs', stub(['old', 'new']));
  commit(repo.main, 'add a job');
  git(repo.main, 'push', '-q', 'origin', 'main');

  const answer = runHook(repo);
  assert.equal(answer.permissionDecision, 'deny');
  assert.match(answer.permissionDecisionReason, /mainを取り込んで/);
  assert.deepEqual(answer.ran, []);

  git(repo.branch, 'pull', '-q', '--no-rebase', '--no-edit', 'origin', 'main');
  assert.deepEqual(runHook(repo).ran, ['old', 'new']);
});

// ブランチが足したジョブは、そのPRの前に回る。
test('the hook runs the jobs that the branch adds', () => {
  const repo = repository();
  write(repo.branch, 'scripts/checks.mjs', stub(['old', 'added']));
  commit(repo.branch, 'add a job');

  const answer = runHook(repo);
  assert.equal(answer.permissionDecision, undefined);
  assert.deepEqual(answer.ran, ['old', 'added']);
});

// ブランチがフックを変えたら、ブランチのフックが回る。
test('the hook of the branch runs when the branch changed it', () => {
  const repo = repository();
  const hook = readFileSync(join(repo.branch, HOOK), 'utf8');
  const marked = hook.replace(
    "const input = JSON.parse(readFileSync(0, 'utf8'));",
    "const input = JSON.parse(readFileSync(0, 'utf8'));\nappendFileSync(process.env.RAN, 'branch hook\\n');",
  );
  assert.notEqual(marked, hook);
  write(repo.branch, HOOK, marked);
  commit(repo.branch, 'change the hook');

  assert.deepEqual(runHook(repo).ran, ['branch hook', 'old']);
});

// mainをpullしていないフックは、PRを止める。
test('the hook stops when its checkout is behind main', () => {
  const repo = repository();
  const other = join(repo.tmp, 'other');
  git(repo.tmp, 'clone', '-q', join(repo.tmp, 'remote.git'), other);
  appendFileSync(join(other, HOOK), '// a newer hook\n');
  commit(other, 'change the hook');
  git(other, 'push', '-q', 'origin', 'main');

  const answer = runHook(repo);
  assert.equal(answer.permissionDecision, 'deny');
  assert.match(answer.permissionDecisionReason, /pull/);
});
