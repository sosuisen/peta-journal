import { defineConfig, mergeConfig } from 'vitest/config';
import vite from './vite.config.ts';

/**
 * `npm run coverage` runs the tests and measures their coverage with the V8 coverage of Node.
 * It prints a table per file and writes a browsable report to coverage/index.html.
 *
 * Below 90% of the lines, it fails, and so does the CI. The lines are counted over the whole
 * app, not file by file. The report is read for the lines no test reaches. They are either
 * removed or given a test of the behaviour that needs them. A test only to reach a line is not
 * written.
 *
 * No file of the app is left out, only the tests. The components are drawn in jsdom, with the
 * module of the server (src/api.ts) replaced in their tests.
 */
export default mergeConfig(
  vite,
  defineConfig({
    test: {
      coverage: {
        provider: 'v8',
        include: ['src/**/*.{ts,tsx}'],
        exclude: ['src/**/*.test.{ts,tsx}'],
        reporter: ['text', 'html'],
        thresholds: { lines: 90 },
      },
    },
  }),
);
