import { resolve, sep } from 'node:path';
import { dataDir, serverPort } from '../playwright.config';

/**
 * Refuses to run the suite against a server that writes outside the throw-away data directory.
 *
 * The tests create and rewrite pages freely. If the server under test exported them into the
 * user's real markdown folder, the user's own server would import the test pages over real
 * ones. This happened once when the test server was started without its own config dir
 * (see ADR 002), so the markdown dir is checked here, before the first test writes anything.
 */
export default async function globalSetup(): Promise<void> {
  const resp = await fetch(`http://localhost:${serverPort}/api/config`);
  if (!resp.ok) {
    throw new Error(
      `GET /api/config on the test server failed: ${resp.status}`,
    );
  }
  const { markdownDir } = (await resp.json()) as { markdownDir?: string };
  const root = resolve(dataDir) + sep;
  if (!markdownDir || !(resolve(markdownDir) + sep).startsWith(root)) {
    throw new Error(
      `The test server's markdown dir is "${markdownDir}", which is not inside the test data dir "${dataDir}". ` +
        'Refusing to run: the tests would write into a real markdown folder.',
    );
  }
}
