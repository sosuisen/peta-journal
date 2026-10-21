import { defineConfig } from '@playwright/test';
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
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
export const dataDir = mkdtempSync(join(tmpdir(), 'peta-journal-e2e-'));
/** The markdown folder the test server exports to. Lives inside `dataDir`, never the user's. */
export const markdownDir = join(dataDir, 'markdown');
const shellDir = resolve(__dirname, '../frontend-rust/crates/shell-leptos');

/**
 * Writes the test server's own config.json and returns its directory.
 *
 * Without `--config-dir` the server reads `~/.peta-journal/config.json` and exports the test
 * pages into the user's real markdown folder, which the user's own server then imports.
 * That overwrote real journal pages once (see ADR 002), so the config dir is never left to the default.
 */
function writeConfigDir(): string {
  const configDir = join(dataDir, 'config');
  mkdirSync(configDir, { recursive: true });
  mkdirSync(markdownDir, { recursive: true });
  writeFileSync(join(configDir, 'config.json'), JSON.stringify({ markdownDir, weekNumbering: 'iso' }));
  return configDir;
}

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
    '--config-dir', JSON.stringify(writeConfigDir()),
  ].join(' ');
}

export default defineConfig({
  testDir: 'tests',
  timeout: 30_000,
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  // Fails the whole run before any test writes a page unless the server is confined to `dataDir`.
  globalSetup: './tests/global-setup.ts',
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
