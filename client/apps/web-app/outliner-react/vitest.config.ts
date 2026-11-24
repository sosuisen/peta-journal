import { defineConfig, mergeConfig } from 'vitest/config';
import vite from './vite.config.ts';

/**
 * `npm run coverage` measures the test coverage with the V8 coverage of Node. It prints a table
 * per file and writes a browsable report to coverage/index.html.
 *
 * Below 90% of the lines, it fails, and so does the CI. The lines are counted over the whole
 * app, not file by file. The report is read for the lines no test reaches. They are either
 * removed or given a test of the behaviour that needs them. A test only to reach a line is not
 * written.
 *
 * The wasm and the types of the border are made from the Rust code (`npm run border`), and
 * are left out, as is the code of the tests in src/test. Stryker skips them too; see
 * stryker.config.mjs.
 *
 * These files are left out too, as agreed on 2026-11-23, because no test in jsdom can check
 * them. They are checked in a real browser.
 * - src/layout.ts: it measures where the browser lays out text, and jsdom lays nothing out
 * - src/main.tsx: the entry point, which loads the wasm and puts the app in #root
 *
 * The components are drawn in jsdom, on a wasm of the tests (src/test/shell.ts).
 */
export default mergeConfig(
  vite,
  defineConfig({
    test: {
      environment: 'jsdom',
      setupFiles: ['src/test/setup.ts'],
      // Not the Playwright tests in e2e/: Playwright runs them
      include: ['src/**/*.test.{ts,tsx}'],
      coverage: {
        provider: 'v8',
        include: ['src/**/*.{ts,tsx}'],
        exclude: [
          'src/**/*.test.{ts,tsx}',
          'src/test/**',
          'src/border/**',
          'src/wasm/**',
          'src/layout.ts',
          'src/main.tsx',
        ],
        reporter: ['text', 'html'],
        thresholds: { lines: 90 },
      },
    },
  }),
);
