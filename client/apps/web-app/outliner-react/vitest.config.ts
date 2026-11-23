import { defineConfig, mergeConfig } from 'vitest/config';
import vite from './vite.config.ts';

/**
 * `npm run coverage` measures the test coverage with the V8 coverage of Node. It prints a table
 * per file and writes a browsable report to coverage/index.html.
 *
 * The numbers are not a target: the report is read for the lines no test reaches, which are
 * either removed or given a test of the behaviour that needs them.
 *
 * The wasm and the types of the border are made from the Rust code (`npm run border`), and
 * are left out, as is the code of the tests in src/test. Stryker skips them too; see
 * stryker.config.mjs.
 *
 * The components are drawn in jsdom, on a wasm of the tests (src/test/shell.ts).
 */
export default mergeConfig(
  vite,
  defineConfig({
    test: {
      environment: 'jsdom',
      setupFiles: ['src/test/setup.ts'],
      coverage: {
        provider: 'v8',
        include: ['src/**/*.{ts,tsx}'],
        exclude: [
          'src/**/*.test.{ts,tsx}',
          'src/test/**',
          'src/border/**',
          'src/wasm/**',
        ],
        reporter: ['text', 'html'],
      },
    },
  }),
);
