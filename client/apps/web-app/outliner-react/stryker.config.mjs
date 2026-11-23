// @ts-check
/**
 * `npm run mutants` runs the mutation testing of Stryker on the code of the shell. A mutant
 * that the tests do not catch shows a line whose result no test checks.
 *
 * Before a PR, only the files the PR changed are mutated:
 * `npm run mutants -- --mutate src/screen.ts,src/core.ts`.
 *
 * The wasm and the types of the border are made from the Rust code, and are left out with the
 * code of the tests, as in the coverage. src/layout.ts and src/main.tsx are left out for the
 * reason that vitest.config.ts gives.
 *
 * @type {import('@stryker-mutator/api/core').PartialStrykerOptions}
 */
export default {
  testRunner: 'vitest',
  // Vitest strips the types and runs a mutant that does not compile; tsc drops it first
  checkers: ['typescript'],
  tsconfigFile: 'tsconfig.app.json',
  coverageAnalysis: 'perTest',
  mutate: [
    'src/**/*.{ts,tsx}',
    '!src/**/*.test.{ts,tsx}',
    '!src/test/**',
    '!src/border/**',
    '!src/wasm/**',
    '!src/layout.ts',
    '!src/main.tsx',
  ],
  reporters: ['clear-text', 'progress', 'html'],
  htmlReporter: { fileName: 'reports/mutation/index.html' },
};
