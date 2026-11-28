// A PreToolUse hook of Claude Code. Before a pull request is created, it runs the checks of the CI
// (scripts/checks.mjs) for the jobs that the branch changes. A failed check stops the creation.
//
// Why a hook: the CI runs only after the pull request is made. The hook finds a failure before
// it is made, without waiting for the CI. The required `ci` check of the CI is what proves that
// every job passed on every OS (ADR 086).
//
// - The changes are those of the commits since the base branch (`--base`, or main).
// - A job that passed on the same commit is not run again. The marks are in .git/peta-verify/.
// - Only the steps that this machine can run are run. A step for another OS is skipped, and so is
//   a step that touches the screen (`screen: true`): the developer could not use the PC meanwhile.
//   A step too heavy for the hook (`ciOnly: true`, such as the tests in an emulator) is skipped
//   too: `screen: false` skips both, as `runJob` takes `ciOnly` from `screen`. No step runs on the Mac of the developer. A skipped step does not stop the pull request, as
//   the CI runs it. A program that is not set up stops it.
// - A hook that throws lets the command run: Claude Code does not take a crash as a "no". So an
//   error in the checks answers "deny", and the pull request waits until it is fixed.
// - The newer of main and the branch is used, for every file of the checks (VERIFICATION).
//   Claude Code runs this script from the checkout where it started, which must be on main. So a
//   branch made before the hook still gets it. When main changed a file of the checks after the
//   branch left it, the hook stops until the branch takes main in. Then the branch has the newest
//   files, and its hook and its list of checks run. So a job that the branch adds runs too.
// - A beta job is not run, whatever files changed (ADR 092). The answer names it in one line, so
//   that its silence is not taken for a pass.

import { execFileSync, spawnSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

// `gh pr create`, or a pull request made through the REST API: a POST to `.../pulls`. gh api
// sends a POST when it is given fields, unless the method says otherwise. A GET of the list of
// pull requests is not one, with a query (`pulls?head=...`) or without
function createsPr(command) {
  if (/\bgh\s+pr\s+create\b/.test(command)) return true;
  return command.split(/[\n|;&]/).some((part) => {
    if (!/\bgh\s+api\b/.test(part) || !/\/pulls(?=["'\s]|$)/.test(part)) return false;
    const method = part.match(/(?:-X|--method)[\s=]*["']?(\w+)/)?.[1]?.toUpperCase();
    if (method) return method === 'POST';
    return /\s(?:-f|-F|--field|--raw-field|--input)(?=[\s=])/.test(part);
  });
}

const lf = (s) => s.replace(/\r\n/g, '\n').trim();
// The code that runs. It is read first: a pull of the checkout while the hook runs must not hide
// that this code is older than main
const running = lf(readFileSync(fileURLToPath(import.meta.url), 'utf8'));

const input = JSON.parse(readFileSync(0, 'utf8'));
const command = input.tool_input?.command ?? '';
if (!createsPr(command)) process.exit(0);

// From here on, an error stops the pull request. Before here, it would stop every command
process.on('uncaughtException', (e) => {
  answer('deny', `PRの前の検証が、エラーで止まりました。直してから、PRを作り直してください。\n${e?.stack ?? e}`);
});

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
const { JOBS, jobsFor, betaLeftOut, runJob, logFileFor } = await import(pathToFileURL(join(root, 'scripts', 'checks.mjs')).href);

const files = git(root, 'diff', '--name-only', `origin/${base}...HEAD`).split('\n').filter(Boolean);
const jobs = jobsFor(files);
if (jobs.length === 0) process.exit(0);
// The beta jobs that the files touch: neither this hook nor the CI runs them (ADR 092)
const beta = betaLeftOut(files);
const betaNotice =
  beta.length === 0
    ? []
    : [
        `betaのジョブは、フックもCIも回しません（ADR 092）: ${beta.join('、')}。通ったことにはなりません。そのモジュールを変えたPRでは、${beta.map((name) => `node scripts/checks.mjs run ${name}`).join('、')} を手で回し、結果をPRの本文に書いてください。`,
      ];

const head = git(root, 'rev-parse', 'HEAD');
// Each step writes its output here as it comes (`node scripts/checks.mjs log` prints the path)
const logFile = logFileFor(root);
const marks = dirname(logFile);
mkdirSync(marks, { recursive: true });

const skipped = [];
for (const name of jobs) {
  const mark = join(marks, `${name}-${head}`);
  if (existsSync(mark)) {
    const kept = JSON.parse(readFileSync(mark, 'utf8'));
    // A mark of an older hook is the list of the skipped steps
    skipped.push(...(Array.isArray(kept) ? kept : kept.skipped).map((x) => ({ job: name, ...x })));
    continue;
  }
  appendFileSync(logFile, `\n### ${name}\n`);
  // No other machine, and no step that touches the screen: the CI runs them (ADR 086)
  const result = await runJob(name, { root, screen: false, log: (s) => appendFileSync(logFile, s) });
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
  // A result in another shape is not a pass, and leaves no mark: the job runs again next time
  if (!Array.isArray(result?.skipped)) {
    throw new Error(`${name}の結果の形が違います: ${JSON.stringify(result)}`);
  }
  writeFileSync(mark, JSON.stringify({ skipped: result.skipped }));
  skipped.push(...result.skipped.map((s) => ({ job: name, ...s })));
}

const done = [`CIと同じ検証が通りました（${jobs.join('、')}）。`, ...betaNotice].join('\n');
if (skipped.length === 0) answer(undefined, undefined, done);
answer(
  undefined,
  undefined,
  [
    done,
    '次の段は、この機械では回していません。PRのCIが回し、必須のciのチェックが結果を確かめます。PRを作ったら、CIの結果を確かめてください。',
    ...skipped.map((s) => `- ${s.job}: ${s.step}（${s.why}）`),
  ].join('\n'),
);
