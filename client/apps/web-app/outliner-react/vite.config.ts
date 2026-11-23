import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig, searchForWorkspaceRoot } from 'vite';
import react from '@vitejs/plugin-react';

/**
 * The journal server serves the built app (ADR 010). `npm run dev` serves it from Vite with
 * hot reload, and sends the API to the server at PETA_API, the developer's own by default.
 */
const api = process.env.PETA_API ?? 'http://localhost:8087';

/**
 * The root of the repository, found by the canvas of the atlas in it. It is searched for and
 * not written as a relative path: Stryker runs the tests in a copy of this app two folders
 * deeper.
 */
function repositoryRoot(): string {
  let dir = dirname(fileURLToPath(import.meta.url));
  while (!existsSync(join(dir, 'atlas', 'web', 'atlas.js'))) {
    const up = dirname(dir);
    if (up === dir) {
      throw new Error('no atlas/web/atlas.js above this app');
    }
    dir = up;
  }
  return dir;
}

const root = repositoryRoot();
/** The scripts of the canvas of the atlas, and the font of its group titles. */
const atlasWeb = join(root, 'atlas', 'web');
/** The fonts of the clients. */
const clientAssets = join(root, 'client', 'assets');

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { '@atlas-web': atlasWeb, '@client-assets': clientAssets },
  },
  build: {
    // At the root: `/assets/` of the server is the images pasted into the journal
    assetsDir: '',
  },
  server: {
    port: 5181,
    proxy: { '/api': { target: api, changeOrigin: true } },
    fs: {
      // The app, and the scripts and fonts of the atlas
      allow: [searchForWorkspaceRoot(process.cwd()), atlasWeb, clientAssets],
    },
  },
});
