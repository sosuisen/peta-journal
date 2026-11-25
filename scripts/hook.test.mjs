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
export const logFileFor = () => process.env.RAN + '.log';
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
function runHook({ main, branch, ran }, command = 'gh pr create') {
  const r = spawnSync(process.execPath, [join(main, HOOK)], {
    input: JSON.stringify({ tool_input: { command }, cwd: branch }),
    encoding: 'utf8',
    env: { ...process.env, RAN: ran, PETA_VERIFY_HANDED: '' },
  });
  assert.equal(r.status, 0, r.stderr);
  const out = r.stdout.trim() ? JSON.parse(r.stdout).hookSpecificOutput : {};
  return { ...out, ran: existsSync(ran) ? readFileSync(ran, 'utf8').split('\n').filter(Boolean) : [] };
}

// フックは、PRを作るコマンドの前にだけ動く。PRの一覧や1つのPRを読むGETでは動かない。
test('the hook runs before a command that makes a pull request, and not before one that reads them', () => {
  const repo = repository();
  const makes = [
    'gh pr create --base main',
    'gh api -X POST repos/o/r/pulls -f title=x',
    'gh api repos/o/r/pulls -f title=x -F "body=@pr.md"',
    'gh api --method=post "repos/o/r/pulls" --input body.json',
    'git push && gh api repos/o/r/pulls -f title=x',
  ];
  const reads = [
    'gh api "repos/o/r/pulls?head=o:b&state=all" --jq .',
    'gh api repos/o/r/pulls',
    'gh api -X GET repos/o/r/pulls -f state=all',
    'gh api repos/o/r/pulls/12 -f state=closed -X PATCH',
    'gh api -X POST repos/o/r/issues/12/assignees -f "assignees[]=me"',
    'gh pr list --head b',
    'gh api "repos/o/r/pulls" --jq . && gh api -X POST repos/o/r/issues/1/comments -f body=x',
  ];
  // The hook answers only when it ran: a command it lets pass gets no answer at all
  const answered = (command) => Object.keys(runHook(repo, command)).some((k) => k !== 'ran');
  for (const command of reads) assert.equal(answered(command), false, command);
  for (const command of makes) assert.equal(answered(command), true, command);
});

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

// フックは、ほかの機械を使わず、画面に触れる段を飛ばす。飛ばした段はCIに任せ、PRは止めない（ADR 086）。
test('the hook uses no other machine, skips the steps on the screen, and leaves them to the ci', () => {
  const repo = repository();
  write(
    repo.branch,
    'scripts/checks.mjs',
    `export const JOBS = { win: { dir: '.' } };
export const jobsFor = () => ['win'];
export const remoteFor = () => ({ platform: 'darwin' });
export const logFileFor = () => process.env.RAN + '.log';
export const runJob = (name, { remote, screen }) => ({
  skipped: [{ step: 'press keys', why: JSON.stringify({ remote: remote ?? null, screen }) }],
});
`,
  );
  commit(repo.branch, 'a job with a step on the screen');

  const answer = runHook(repo);
  assert.equal(answer.permissionDecision, undefined);
  assert.match(answer.additionalContext, /win: press keys（\{"remote":null,"screen":false\}）/);
  assert.match(answer.additionalContext, /PRのCIが回し/);
  assert.doesNotMatch(answer.additionalContext, /issue/);
});

// 検証の途中で例外が出たら、PRを止める。落ちたフックを、Claude Codeは止める答えとして扱わないため。
test('the hook stops the pull request when a check throws', () => {
  const repo = repository();
  write(
    repo.branch,
    'scripts/checks.mjs',
    `export const JOBS = { broken: { dir: '.' } };
export const jobsFor = () => ['broken'];
export const logFileFor = () => process.env.RAN + '.log';
export async function runJob() {
  throw new Error('the check broke');
}
`,
  );
  commit(repo.branch, 'break the checks');

  const answer = runHook(repo);
  assert.equal(answer.permissionDecision, 'deny');
  assert.match(answer.permissionDecisionReason, /the check broke/);
});

// 形の違う結果は、通ったことにしない。印も残さないので、次に同じコミットで回し直せる。
test('the hook stops and leaves no mark when a check answers in another shape', () => {
  const repo = repository();
  write(
    repo.branch,
    'scripts/checks.mjs',
    `export const JOBS = { odd: { dir: '.' } };
export const jobsFor = () => ['odd'];
export const logFileFor = () => process.env.RAN + '.log';
export const runJob = () => ({ remote: [] });
`,
  );
  commit(repo.branch, 'answer in another shape');

  const answer = runHook(repo);
  assert.equal(answer.permissionDecision, 'deny');
  const head = git(repo.branch, 'rev-parse', 'HEAD');
  assert.equal(existsSync(join(repo.tmp, `odd-${head}`)), false);
});

// 動いているフックがmainと違えば、起動したあとにディスクのファイルがmainと同じになっても、PRを止める。
test('the hook stops when it started from a file older than main, even if the file is pulled while it runs', () => {
  const repo = repository();
  const onMain = readFileSync(join(repo.main, HOOK), 'utf8');
  // The running hook is older than main, and a pull puts the hook of main on disk right after it starts
  const older = onMain.replace(
    "const input = JSON.parse(readFileSync(0, 'utf8'));",
    `const input = JSON.parse(readFileSync(0, 'utf8'));\nwriteFileSync(fileURLToPath(import.meta.url), ${JSON.stringify(onMain)});`,
  );
  assert.notEqual(older, onMain);
  write(repo.main, HOOK, older);

  const answer = runHook(repo);
  assert.equal(answer.permissionDecision, 'deny');
  assert.match(answer.permissionDecisionReason, /pull/);
  assert.deepEqual(answer.ran, []);
});
