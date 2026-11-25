// Tests of scripts/mac.mjs, the Mac that runs the steps for macOS. A run over SSH is tested with
// a fake SSH and a clone in a temporary folder that plays the Mac (fakeMac below).

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { test } from 'node:test';
import { macOf, parseRemote } from './mac.mjs';

// リモートのURLから、ホストとフォルダーを取る。フォルダーは、ホームからの道でもよい。
test('a remote url over ssh gives its host and folder', () => {
  assert.deepEqual(parseRemote('mac:src/clone'), { host: 'mac', path: 'src/clone' });
  assert.deepEqual(parseRemote('me@mac:/Volumes/disk/clone'), { host: 'me@mac', path: '/Volumes/disk/clone' });
  assert.deepEqual(parseRemote('ssh://mac/~/src/clone'), { host: 'mac', path: 'src/clone' });
});

// SSHでないリモートは、Macとして使わない。
test('a remote url that is not over ssh gives no mac', () => {
  assert.equal(parseRemote('https://github.com/owner/repo'), undefined);
  assert.equal(parseRemote('C:/repos/clone'), undefined);
  assert.equal(parseRemote('/home/me/clone'), undefined);
});

// macという名前のリモートがなければ、Macはない。
test('a checkout without a remote named mac has no mac', () => {
  const dir = mkdtempSync(join(tmpdir(), 'peta-mac-'));
  execFileSync('git', ['init', '-q', dir]);
  assert.equal(macOf(dir), undefined);
  execFileSync('git', ['remote', 'add', 'mac', 'mac:src/clone'], { cwd: dir });
  assert.equal(macOf(dir).platform, 'darwin');
});

// The sh that runs the commands of the fake Mac. On Windows it is the one that comes with Git
function shell() {
  if (process.platform !== 'win32') return 'sh';
  const gitCore = execFileSync('git', ['--exec-path'], { encoding: 'utf8' }).trim();
  return resolve(gitCore, '..', '..', '..', 'bin', 'sh.exe');
}

// The list of checks in the commit that goes to the Mac. It tells the commit it ran on, and the
// job `broken` fails without a result line
const STUB_CHECKS = `import { execFileSync } from 'node:child_process';
const job = process.argv[3];
console.log('working on the mac');
if (job === 'broken') {
  console.log('the job broke');
  process.exit(1);
}
const head = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
console.log('PETA_RESULT ' + JSON.stringify({ skipped: [], job, head }));
`;

// A checkout here with a remote named mac, and a fake Mac behind a fake SSH. The Mac is a clone
// in a home folder. Its host is fakemac: the SSH answers no other host, as a Mac that is away
function fakeMac({ host = 'fakemac' } = {}) {
  const tmp = mkdtempSync(join(tmpdir(), 'peta-fake-mac-'));
  const git = (cwd, ...args) =>
    execFileSync('git', ['-c', 'user.name=test', '-c', 'user.email=test@example.com', ...args], {
      cwd,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  const here = join(tmp, 'here');
  const home = join(tmp, 'home');
  mkdirSync(home);
  git(tmp, 'init', '-q', '-b', 'main', here);
  mkdirSync(join(here, 'scripts'));
  writeFileSync(join(here, 'scripts', 'checks.mjs'), STUB_CHECKS);
  git(here, 'add', '-A');
  git(here, 'commit', '-q', '-m', 'start');
  const clone = join(home, 'clone');
  git(home, 'clone', '-q', here, clone);

  // The login shell of the Mac (`"$SHELL" -lic <command>`) runs its command
  const login = join(tmp, 'login.sh');
  writeFileSync(login, '#!/bin/sh\nshift\nexec sh -c "$1"\n');
  const ssh = join(tmp, 'ssh.mjs');
  writeFileSync(
    ssh,
    `import { spawnSync } from 'node:child_process';
const args = process.argv.slice(2);
while (args[0] === '-o') args.splice(0, 2);
const [host, command] = args;
if (host !== 'fakemac') {
  process.stderr.write('ssh: connect to host ' + host + ' port 22: Connection refused\\n');
  process.exit(255);
}
const r = spawnSync(${JSON.stringify(shell())}, ['-c', command], {
  cwd: ${JSON.stringify(home)},
  stdio: 'inherit',
  env: { ...process.env, SHELL: ${JSON.stringify(login)} },
});
process.exit(r.status ?? 1);
`,
  );
  git(here, 'remote', 'add', 'mac', `${host}:clone`);
  // Git runs the command in sh, which reads a backslash as an escape
  git(here, 'config', 'core.sshCommand', `node ${ssh.replaceAll('\\', '/')}`);
  // Git asks an SSH it does not know which options it takes. This one takes none
  git(here, 'config', 'ssh.variant', 'simple');
  // A commit that the Mac does not have yet
  writeFileSync(join(here, 'app.txt'), 'two\n');
  git(here, 'add', '-A');
  git(here, 'commit', '-q', '-m', 'change the app');
  return { here, clone, git };
}

// Macにつながらなければ、そのわけを添えて、つながらないと答える。
test('a mac that cannot be reached answers so with the reason', async () => {
  const { here } = fakeMac({ host: 'away' });
  const answer = await macOf(here).runJob('stickies-mac');
  assert.match(answer.unreachable, /Connection refused/);
});

// 作業ツリーに変更があれば、Macへ渡さずに止まる。Macで回すのはコミットだけである。
test('a checkout with changes is not sent to the mac', async () => {
  const { here, clone, git } = fakeMac();
  writeFileSync(join(here, 'app.txt'), 'three\n');
  const answer = await macOf(here).runJob('stickies-mac');
  assert.equal(answer.failed.step, 'Macへ渡す');
  assert.throws(() => git(clone, 'rev-parse', '--verify', 'refs/heads/peta-verify'));
});

// コミットをMacのクローンへpushし、そのコミットでジョブを回して、結果の行を返す。
test('the commit is pushed to the mac, and the job runs on it there', async () => {
  const { here, clone, git } = fakeMac();
  const head = git(here, 'rev-parse', 'HEAD');
  let logged = '';
  const answer = await macOf(here).runJob('stickies-mac', { log: (s) => (logged += s) });
  assert.deepEqual(answer, { skipped: [], job: 'stickies-mac', head });
  assert.equal(git(clone, 'rev-parse', 'refs/heads/peta-verify'), head);
  assert.match(logged, /working on the mac/);
});

// 結果の行がなければ、出力の終わりを添えて落ちる。落ちても、クローンはmainに戻る。
test('a job on the mac without a result line fails with its output, and the clone goes back to main', async () => {
  const { here, clone, git } = fakeMac();
  const answer = await macOf(here).runJob('broken');
  assert.equal(answer.failed.step, 'Macで回す');
  assert.match(answer.failed.tail, /the job broke/);
  assert.equal(git(clone, 'symbolic-ref', '--short', 'HEAD'), 'main');
});

// ジョブが通ったあとも、クローンはmainに戻る。開発者がMacで使う状態のままにするため。
test('the clone goes back to main after a job on the mac', async () => {
  const { here, clone, git } = fakeMac();
  await macOf(here).runJob('stickies-mac');
  assert.equal(git(clone, 'symbolic-ref', '--short', 'HEAD'), 'main');
});
