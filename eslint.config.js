import js from '@eslint/js';
import { defineConfig, globalIgnores } from 'eslint/config';
import reactHooks from 'eslint-plugin-react-hooks';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default defineConfig(
  globalIgnores([
    '**/node_modules/**',
    '**/target/**',
    '**/build/**',
    '**/dist/**',
    '.cache/**',
    '.wrangler/**',
    'artifacts/**',
  ]),
  {
    files: ['**/*.{js,mjs,ts,tsx}'],
    extends: [js.configs.recommended],
  },
  {
    files: ['**/*.{ts,tsx}'],
    extends: [tseslint.configs.recommended],
  },
  {
    files: ['web/src/**/*.{ts,tsx}'],
    languageOptions: { globals: globals.browser },
  },
  {
    files: [
      'eslint.config.js',
      'scripts/**/*.mjs',
      'tests/**/*.ts',
      'web/vite.config.ts',
    ],
    languageOptions: { globals: globals.node },
  },
  {
    files: ['web/src/**/*.{ts,tsx}'],
    extends: [reactHooks.configs.flat.recommended],
  },
);
