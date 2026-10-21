import { defineConfig } from '@playwright/test';
import { existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

/**
 * End-to-end tests drive the real app: the journal server on its own port with a fresh,
 * throw-away data directory, and the frontend dev server (trunk) in front of it.
 *
 * Build first: `cd journal-server-rust && cargo build --release`, and `cargo install trunk`.
 */
export const serverPort = 8187;
const webPort = 4300;
const dataDir = mkdtempSync(join(tmpdir(), 'peta-journal-e2e-'));
const shellDir = resolve(__dirname, '../frontend-rust/crates/shell-leptos');

/** Serves the Leptos frontend on `webPort`, proxying `/api` to `serverPort`. */
function frontendCommand(): string {
  return `trunk serve --config ${JSON.stringify(resolve(shellDir, 'Trunk.e2e.toml'))}`;
}

function serverCommand(): string {
  const binary = resolve(__dirname, '../journal-server-rust/target/release/journal-server'
    + (process.platform === 'win32' ? '.exe' : ''));
  if (!existsSync(binary)) {
    throw new Error(`Server binary not found: ${binary}\nRun "cargo build --release" in journal-server-rust first.`);
  }
  return [
    JSON.stringify(binary), '--port', String(serverPort),
    '--data-dir', JSON.stringify(dataDir), '--index-dir', JSON.stringify(join(dataDir, 'index')),
  ].join(' ');
}

export default defineConfig({
  testDir: 'tests',
  timeout: 30_000,
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: `http://localhost:${webPort}`,
    trace: 'retain-on-failure',
  },
  webServer: [
    {
      command: serverCommand(),
      url: `http://localhost:${serverPort}/api/pages/recent`,
      timeout: 60_000,
      reuseExistingServer: false,
    },
    {
      command: frontendCommand(),
      cwd: shellDir,
      url: `http://localhost:${webPort}/`,
      timeout: 180_000,
      reuseExistingServer: false,
    },
  ],
});
