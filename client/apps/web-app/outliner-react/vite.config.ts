import { defineConfig, searchForWorkspaceRoot } from 'vite';
import react from '@vitejs/plugin-react';

/**
 * The journal server serves the built app (ADR 010). `npm run dev` serves it from Vite with
 * hot reload, and sends the API to the server at PETA_API, the developer's own by default.
 */
const api = process.env.PETA_API ?? 'http://localhost:8087';

export default defineConfig({
  plugins: [react()],
  build: {
    // At the root, as Trunk puts the files of the Leptos shell: `/assets/` of the server is
    // the images pasted into the journal
    assetsDir: '',
  },
  server: {
    port: 5181,
    proxy: { '/api': { target: api, changeOrigin: true } },
    fs: {
      // The app, the styles of the Leptos shell, and the scripts and fonts of the atlas
      allow: [
        searchForWorkspaceRoot(process.cwd()),
        '../outliner',
        '../../../../atlas/web',
        '../../../assets',
      ],
    },
  },
});
