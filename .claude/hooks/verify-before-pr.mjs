// A PreToolUse hook of Claude Code. Before a pull request is created, it runs the checks of the CI
// (scripts/checks.mjs) for the jobs that the branch changes. A failed check stops the creation.
//
// Why a hook: the CI runs only by hand, and a failed CI does not stop a merge. The rules of the
// checks are kept by this hook, not only by the skills of the AI.
//
// - The changes are those of the commits since the base branch (`--base`, or main).
// - A job that passed on the same commit is not run again. The marks are in .git/peta-verify/.
// - A step for another OS does not stop the pull request. The hook tells the AI, which writes it
//   in the pull request and in the issue. A program that is not set up stops it.
// - The newer of main and the branch is used, for every file of the checks (VERIFICATION).
//   Claude Code runs this script from the checkout where it started, which must be on main. So a
//   branch made before the hook still gets it. When main changed a file of the checks after the
//   branch left it, the hook stops until the branch takes main in. Then the branch has the newest
//   files, and its hook and its list of checks run. So a job that the branch adds runs too.

import { execFileSync, spawnSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

// `gh pr create`, or a pull request made through the REST API
const CREATES_PR = /\bgh\s+pr\s+create\b|\bgh\s+api\b[^\n|;&]*\/pulls(?![\w/])/;

const input = JSON.parse(readFileSync(0, 'utf8'));
const command = input.tool_input?.command ?? '';
if (!CREATES_PR.test(command)) process.exit(0);

const git = (cwd, ...args) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

function answer(decision, reason, context) {
  // On a pass, no decision is given: the usual permission rules still ask about the command
  const out = { hookSpecificOutput: { hookEventName: 'PreToolUse' } };
  if (decision) out.hookSpecificOutput.permissionDecision = decision;
  if (reason) out.hookSpecificOutput.permissionDecisionReason = reason;
  if (context) out.hookSpecificOutput.additionalContext = context;
  console.log(JSON.stringify(out));
  process.exit(0);
}

let root;
try {
  root = git(input.cwd ?? process.cwd(), 'rev-parse', '--show-toplevel');
} catch {
  process.exit(0); // not in a repository: nothing to check
}

if (git(root, 'status', '--porcelain')) {
  answer('deny', '検証の前に、作業ツリーの変更をコミットしてください。検証は、PRに入るコミットに対して行います。');
}

const base = command.match(/(?:--base|-B)[\s=]+["']?([\w./-]+)/)?.[1] ?? 'main';
try {
  execFileSync('git', ['fetch', '-q', 'origin', base], { cwd: root, stdio: 'ignore' });
} catch {
  // Offline: the last fetched base is used
}
// The files of the checks: the hook, its setting, the list of checks, and the tests that guard it
const VERIFICATION = ['.claude/hooks/', '.claude/settings.json', 'scripts/'];
const HOOK = '.claude/hooks/verify-before-pr.mjs';
// Set when the hook of main hands the work to the newer hook of the branch
const HANDED = 'PETA_VERIFY_HANDED';

if (!process.env[HANDED]) {
  // This script is read from the checkout where Claude Code started, on main. A checkout not
  // pulled would run an old hook, so the hook stops until it is pulled
  const HOOK_ROOT = resolve(fileURLToPath(import.meta.url), '..', '..', '..');
  try {
    execFileSync('git', ['fetch', '-q', 'origin', 'main'], { cwd: HOOK_ROOT, stdio: 'ignore' });
  } catch {
    // Offline: the last fetched main is used
  }
  const lf = (s) => s.replace(/\r\n/g, '\n').trim();
  const running = lf(readFileSync(join(HOOK_ROOT, HOOK), 'utf8'));
  let onMain;
  try {
    onMain = lf(git(HOOK_ROOT, 'show', `origin/main:${HOOK}`));
  } catch {
    onMain = undefined; // not on main yet
  }
  if (onMain !== undefined && onMain !== running) {
    answer('deny', `検証のスクリプトが、origin/mainと違います（${HOOK}）。${HOOK_ROOT}でmainをpullしてから、PRを作り直してください。`);
  }

  // The files of the checks that main changed after the branch left it
  const fork = git(root, 'merge-base', 'origin/main', 'HEAD');
  const newerOnMain = git(root, 'diff', '--name-only', fork, 'origin/main', '--', ...VERIFICATION)
    .split('\n')
    .filter(Boolean);
  if (newerOnMain.length > 0) {
    answer(
      'deny',
      `mainの検証のファイルが、このブランチより新しくなっています（${newerOnMain.join('、')}）。mainを取り込んでから、PRを作り直してください。`,
    );
  }

  // The branch changed the hook: its hook is the newer one, and does the work
  const branchHook = join(root, HOOK);
  if (lf(readFileSync(branchHook, 'utf8')) !== running) {
    const r = spawnSync(process.execPath, [branchHook], {
      input: JSON.stringify(input),
      encoding: 'utf8',
      env: { ...process.env, [HANDED]: '1' },
    });
    if (r.status !== 0) {
      answer('deny', `このブランチのフック（${HOOK}）が失敗しました。\n${(r.stderr ?? '').slice(-2000)}`);
    }
    process.stdout.write(r.stdout);
    process.exit(0);
  }
}

// The list of checks of the branch. It is the newest: main has none newer (above)
const { JOBS, jobsFor, runJob } = await import(pathToFileURL(join(root, 'scripts', 'checks.mjs')).href);

const files = git(root, 'diff', '--name-only', `origin/${base}...HEAD`).split('\n').filter(Boolean);
const jobs = jobsFor(files);
if (jobs.length === 0) process.exit(0);

const head = git(root, 'rev-parse', 'HEAD');
const marks = join(resolve(root, git(root, 'rev-parse', '--git-common-dir')), 'peta-verify');
mkdirSync(marks, { recursive: true });
const logFile = join(marks, `${head}.log`);

const skipped = [];
for (const name of jobs) {
  const mark = join(marks, `${name}-${head}`);
  if (existsSync(mark)) {
    skipped.push(...JSON.parse(readFileSync(mark, 'utf8')).map((s) => ({ job: name, ...s })));
    continue;
  }
  appendFileSync(logFile, `\n### ${name}\n`);
  const result = runJob(name, { root, log: (s) => appendFileSync(logFile, s) });
  if (result.failed) {
    answer(
      'deny',
      [
        `CIと同じ検証が、${name}（${JOBS[name].dir}）で失敗しました。直してから、PRを作り直してください。`,
        `失敗した段: ${result.failed.step}`,
        `全体のログ: ${logFile}`,
        '',
        result.failed.tail,
      ].join('\n'),
    );
  }
  writeFileSync(mark, JSON.stringify(result.skipped));
  skipped.push(...result.skipped.map((s) => ({ job: name, ...s })));
}

const done = `CIと同じ検証が通りました（${jobs.join('、')}）。`;
if (skipped.length === 0) answer(undefined, undefined, done);
answer(
  undefined,
  undefined,
  [
    done,
    'ただし、ほかのOSの段は回していません。PRの本文の「確かめていないこと」に書き、Refsのissueにチェックボックスを足してください。',
    ...skipped.map((s) => `- ${s.job}: ${s.step}（${s.why}）`),
  ].join('\n'),
);
