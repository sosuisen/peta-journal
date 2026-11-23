import js from '@eslint/js';
import { defineConfig, globalIgnores } from 'eslint/config';
import prettier from 'eslint-config-prettier';
import reactHooks from 'eslint-plugin-react-hooks';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default defineConfig([
  // The wasm and the types of the border are made from the Rust code (`npm run border`)
  globalIgnores([
    'dist',
    'dist-e2e',
    'src/wasm',
    'src/border',
    // The E2E tests have their own package and config of Playwright
    'e2e',
    // Reports of `npm run coverage` and `npm run mutants`
    'coverage',
    'reports',
    '.stryker-tmp',
  ]),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommendedTypeChecked,
      reactHooks.configs.flat.recommended,
      // Last, so that no rule above argues with Prettier about the layout
      prettier,
    ],
    languageOptions: {
      globals: globals.browser,
      // typescript-eslint reads the code through the API of TypeScript 6, because TypeScript 7.0
      // has no API. So `typescript` in package.json is an alias of `@typescript/typescript6`,
      // and `tsc` is TypeScript 7, installed as `@typescript/native`.
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      '@typescript-eslint/explicit-member-accessibility': [
        'error',
        { accessibility: 'no-public' },
      ],
      '@typescript-eslint/prefer-readonly': 'error',
    },
  },
]);
