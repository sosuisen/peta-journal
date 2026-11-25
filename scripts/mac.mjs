// Runs a job of scripts/checks.mjs on a Mac of the developer, over SSH. A job run by hand on
// Windows uses it, for the steps that only macOS can run. The hook before a pull request does not
// (ADR 086): the CI runs those steps.
//
// Nothing about the Mac is written here. It comes from the Git settings of the developer:
// - A remote named `mac`, whose URL is the clone on the Mac that the tests use (`host:path`, or
//   `ssh://host/path`). The commit is pushed there, so no push to GitHub is needed.
// - `core.sshCommand`, the SSH that Git uses, or `ssh`. It must not ask anything: the hook
//   cannot answer, so a key with a passphrase must be in the agent.
// The commands run in the login shell of the Mac, so they find the tools of its PATH.

import { spawnSync } from 'node:child_process';
import { streamed } from './checks.mjs';

export const REMOTE = 'mac';

// The branch of the Mac clone that holds the commit to check. Each commit has its own, so no push
// is forced: the developer would have to allow each forced push. It is deleted after the job
const branchOf = (head) => `peta-verify-${head}`;

// The line of the output that holds the result of the job on the Mac
export const RESULT = 'PETA_RESULT ';

// The host and the folder of a remote URL, or undefined when it is not one over SSH.
// A path without `/` at its start is in the home folder
export function parseRemote(url) {
  const ssh = url.match(/^ssh:\/\/([^/]+)(\/.+)$/);
  if (ssh) return { host: ssh[1], path: ssh[2].replace(/^\/~\//, '') };
  // A letter and a colon is a folder of Windows, not a host. `://` begins a URL of another kind
  const scp = url.match(/^([^/:]{2,}):(?!\/\/)(.+)$/);
  if (scp) return { host: scp[1], path: scp[2] };
  return undefined;
}

// A text in single quotes for the shell of the Mac
const quote = (s) => `'${s.replaceAll("'", `'\\''`)}'`;

const run = (program, args, options) => spawnSync(program, args, { encoding: 'utf8', maxBuffer: 1 << 30, ...options });

function gitOut(root, ...args) {
  const r = run('git', args, { cwd: root });
  return r.status === 0 ? r.stdout.trim() : undefined;
}

// The Mac of the developer for the checkout at `root`, or undefined when there is none.
// Its `runJob` answers as runJob of checks.mjs does: { failed, skipped }
export function macOf(root) {
  const url = gitOut(root, 'remote', 'get-url', REMOTE);
  const remote = url && parseRemote(url);
  if (!remote) return undefined;
  // A path with a space in the SSH command is not split right. The paths of OpenSSH have none
  const [ssh, ...sshArgs] = (gitOut(root, 'config', 'core.sshCommand') || 'ssh').split(/\s+/);
  const sshTo = (command) => [...sshArgs, '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', '-o', 'ServerAliveInterval=30', remote.host, command];
  const onMac = (command) => run(ssh, sshTo(command));
  // The output of the job comes to `log` as the Mac sends it
  const streamOnMac = (command, log) => streamed(ssh, sshTo(command), {}, log);
  return {
    platform: 'darwin',
    runJob: (name, { log = () => {} } = {}) => runOnMac({ root, remote, onMac, streamOnMac, name, log }),
  };
}

async function runOnMac({ root, remote, onMac, streamOnMac, name, log }) {
  const reach = onMac('echo ok');
  if (reach.status !== 0) {
    // Away from the Mac, the steps are skipped as without one. The hook says so
    return { unreachable: `Macにつながらない（${(reach.stderr || String(reach.error ?? '')).trim()}）` };
  }
  if (gitOut(root, 'status', '--porcelain')) {
    return { failed: { step: 'Macへ渡す', tail: '作業ツリーに変更がある。Macで回すのは、コミットだけである。' } };
  }
  const head = gitOut(root, 'rev-parse', 'HEAD');
  const branch = branchOf(head);
  const push = run('git', ['push', '-q', REMOTE, `HEAD:refs/heads/${branch}`], { cwd: root });
  if (push.status !== 0) return { failed: { step: 'Macへ渡す', tail: `${push.stdout}${push.stderr}`.trim() } };
  // The clone goes back to main, where the developer left it, even when the job fails. The
  // branch goes too
  const command = [
    `cd ${quote(remote.path)}`,
    `git switch -q --detach ${head}`,
    `{ "$SHELL" -lic ${quote(`node scripts/checks.mjs run ${name} --result`)}; s=$?; git switch -q main; git branch -q -D ${branch}; exit $s; }`,
  ].join(' && ');
  const { out } = await streamOnMac(command, log);
  const line = out.split('\n').findLast((l) => l.startsWith(RESULT));
  if (!line) return { failed: { step: 'Macで回す', tail: out.trimEnd().split('\n').slice(-40).join('\n') } };
  return JSON.parse(line.slice(RESULT.length));
}
