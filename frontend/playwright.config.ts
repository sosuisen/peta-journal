import { defineConfig } from '@playwright/test';
import { existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

/**
 * End-to-end tests drive the real app: the journal server on its own port with a fresh,
 * throw-away data directory, and the Angular dev server in front of it.
 *
 * Build the server first: `cd journal-server && mvn package -DskipTests`, or set
 * `PETA_JOURNAL_SERVER=rust` to drive the Rust server (`cd journal-server-rust && cargo build --release`).
 */
export const serverPort = 8187;
const webPort = 4300;
const dataDir = mkdtempSync(join(tmpdir(), 'peta-journal-e2e-'));

/**
 * Which frontend to drive: the Angular dev server (default) or the Rust (Leptos) frontend
 * served by trunk (`PETA_JOURNAL_FRONTEND=rust`, needs `cargo install trunk`).
 */
function frontendCommand(): string {
  if (process.env['PETA_JOURNAL_FRONTEND'] === 'rust') {
    const shell = resolve(__dirname, '../frontend-rust/crates/shell-leptos');
    return `trunk serve --config ${JSON.stringify(resolve(shell, 'Trunk.e2e.toml'))}`;
  }
  return `npx ng serve --port ${webPort} --proxy-config e2e/proxy.conf.json`;
}

/**
 * Which server to drive: the Java server (default) or the Rust server
 * (`PETA_JOURNAL_SERVER=rust`, built with `cargo build --release` in journal-server-rust).
 */
function serverCommand(): string {
  if (process.env['PETA_JOURNAL_SERVER'] === 'rust') {
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
  const serverJar = resolve(__dirname, '../journal-server/target/quarkus-app/quarkus-run.jar');
  if (!existsSync(serverJar)) {
    throw new Error(`Server jar not found: ${serverJar}\nRun "mvn package -DskipTests" in journal-server first.`);
  }
  return [
    'java', '--enable-preview', '--enable-native-access=ALL-UNNAMED',
    `-Dquarkus.http.port=${serverPort}`, `-Dpetajournal.data.dir=${dataDir}`,
    '-jar', serverJar,
  ].join(' ');
}

export default defineConfig({
  testDir: 'e2e',
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
      cwd: process.env['PETA_JOURNAL_FRONTEND'] === 'rust' ? resolve(__dirname, '../frontend-rust/crates/shell-leptos') : __dirname,
      url: `http://localhost:${webPort}/`,
      timeout: 180_000,
      reuseExistingServer: false,
    },
  ],
});
