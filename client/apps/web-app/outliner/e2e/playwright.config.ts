import { defineConfig } from '@playwright/test';
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

/**
 * End-to-end tests drive the real app the way it runs for the user (ADR 010): one journal
 * server, on its own port with a fresh, throw-away data directory, that answers the API and
 * serves the frontend trunk built.
 *
 * The run builds the server and the frontend first. Setup, once: `cargo install trunk`.
 */
export const serverPort = 8187;
export const dataDir = mkdtempSync(join(tmpdir(), 'peta-journal-e2e-'));
/** The markdown folder the test server exports to. Lives inside `dataDir`, never the user's. */
export const markdownDir = join(dataDir, 'markdown');
const shellDir = resolve(__dirname, '..');
/** Not `dist`: that folder belongs to the developer's own server, which may be reading it now. */
const frontendDir = resolve(shellDir, 'dist-e2e');

/**
 * Writes the test server's own config.json. It names no markdown folder: a throw-away server
 * has `markdown/` under its folder. Its `docs/` is there from the start, as in a folder in use:
 * the server watches the folder from its start then, and a file a test writes is taken in at
 * once, not after the next look for the folders.
 */
function writeConfig(): void {
  const configDir = join(dataDir, 'config');
  mkdirSync(configDir, { recursive: true });
  mkdirSync(join(markdownDir, 'docs'), { recursive: true });
  writeFileSync(join(configDir, 'config.json'), JSON.stringify({ weekNumbering: 'iso' }));
  writeAutoTemplate(join(configDir, 'functions', 'auto-template'));
}

/** The pages and the tag the templates of the test server answer. */
export const templatePagePrefix = 'template-new-';
export const templateTag = 'template-tag';

/**
 * Puts the real auto-template extension into the test server, with templates of its own.
 * Not the sample template.md: its weekday templates would fill the journal pages of other tests.
 */
function writeAutoTemplate(dir: string): void {
  const source = resolve(__dirname, '../../../../../ext-functions/auto-template');
  mkdirSync(dir, { recursive: true });
  for (const file of ['config.json', 'main.js']) copyFileSync(join(source, file), join(dir, file));
  writeFileSync(join(dir, 'e2e.md'), [
    '---', `pattern: /^${templatePagePrefix}/`, '---', 'from page template', '',
    '---', `tag: ${templateTag}`, '---', 'from tag template', '',
  ].join('\n'));
}

/**
 * Builds the Leptos frontend into `frontendDir`. Runs in `shellDir`, where Trunk.toml is. The
 * timing test (`PETA_TIMING=1`) needs the release build: a debug build of wasm is many times
 * slower, and its times say nothing.
 */
function frontendBuildCommand(): string {
  const release = process.env.PETA_TIMING ? ' --release' : '';
  return `trunk build${release} --dist ${JSON.stringify(frontendDir)}`;
}

const serverDir = resolve(__dirname, '../../../../../journal');

/**
 * Builds the server. Without this the tests ran against whatever binary was there: on
 * 2026-11-11 a binary a few commits old made a test of a new request fail, and it looked like
 * a bug of the app. A build with nothing to do takes under a second.
 */
function serverBuildCommand(): string {
  return `cargo build --release --manifest-path ${JSON.stringify(join(serverDir, 'Cargo.toml'))}`;
}

function serverCommand(): string {
  writeConfig();
  const binary = join(serverDir, 'target/release/journal-server'
    + (process.platform === 'win32' ? '.exe' : ''));
  return [
    JSON.stringify(binary), '--port', String(serverPort),
    // Everything the server writes is under this folder. Started with `--data-dir` alone, a
    // test server read `~/.peta-journal/config.json` and exported the test pages into the
    // user's real markdown folder, which the user's own server then imported. That overwrote
    // real journal pages once (see ADR 002)
    '--throwaway-dir', JSON.stringify(dataDir),
    '--frontend-dir', JSON.stringify(frontendDir),
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
    baseURL: `http://localhost:${serverPort}`,
    trace: 'retain-on-failure',
  },
  webServer: {
    // The server starts only after both builds, so no test ever sees an older build
    command: `${serverBuildCommand()} && ${frontendBuildCommand()} && ${serverCommand()}`,
    cwd: shellDir,
    url: `http://localhost:${serverPort}/`,
    // Long enough for a first build of the server and the frontend
    timeout: 900_000,
    reuseExistingServer: false,
  },
});
