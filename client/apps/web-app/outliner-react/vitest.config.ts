import { defineConfig, mergeConfig } from 'vitest/config';
import vite from './vite.config';

/**
 * `npm run coverage` measures the test coverage with the V8 coverage of Node. It prints a table
 * per file and writes a browsable report to coverage/index.html.
 *
 * The numbers are not a target: the report is read for the lines no test reaches, which are
 * either removed or given a test of the behaviour that needs them.
 *
 * The wasm and the types of the border are made from the Rust code (`npm run border`), and
 * are left out. Stryker skips them too; see stryker.config.mjs.
 */
export default mergeConfig(
  vite,
  defineConfig({
    test: {
      coverage: {
        provider: 'v8',
        include: ['src/**/*.{ts,tsx}'],
        exclude: ['src/**/*.test.ts', 'src/border/**', 'src/wasm/**'],
        reporter: ['text', 'html'],
      },
    },
  }),
);
