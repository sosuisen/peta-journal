import { defineConfig } from '@playwright/test';
import { existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

/**
 * End-to-end tests drive the real app: the journal server on its own port with a fresh,
 * throw-away data directory, and the Angular dev server in front of it.
 *
 * Build the server first: `cd journal-server && mvn package -DskipTests`.
 */
export const serverPort = 8187;
const webPort = 4300;
const serverJar = resolve(__dirname, '../journal-server/target/quarkus-app/quarkus-run.jar');
if (!existsSync(serverJar)) {
  throw new Error(`Server jar not found: ${serverJar}\nRun "mvn package -DskipTests" in journal-server first.`);
}
const dataDir = mkdtempSync(join(tmpdir(), 'peta-journal-e2e-'));

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
      command: [
        'java', '--enable-preview', '--enable-native-access=ALL-UNNAMED',
        `-Dquarkus.http.port=${serverPort}`, `-Dpetajournal.data.dir=${dataDir}`,
        '-jar', serverJar,
      ].join(' '),
      url: `http://localhost:${serverPort}/api/pages/recent`,
      timeout: 60_000,
      reuseExistingServer: false,
    },
    {
      command: `npx ng serve --port ${webPort} --proxy-config e2e/proxy.conf.json`,
      url: `http://localhost:${webPort}/`,
      timeout: 180_000,
      reuseExistingServer: false,
    },
  ],
});
