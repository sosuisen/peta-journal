// Tests of scripts/mac.mjs, the Mac that runs the steps for macOS. A run over SSH needs a real
// Mac, so it is not tested here.

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
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
