// crates/svc-admin/ui/eslint.config.mjs
//
// Flat ESLint 9 config for the svc-admin SPA.
// Keeps TS + React Hooks rules minimal but sane.

import tsParser from '@typescript-eslint/parser'
import tsEslintPlugin from '@typescript-eslint/eslint-plugin'
import reactHooks from 'eslint-plugin-react-hooks'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

export default [
  // Ignore build output
  {
    ignores: ['dist/**'],
  },

  // TypeScript + React source files
  {
    files: ['src/**/*.{ts,tsx}'],

    languageOptions: {
      parser: tsParser,
      parserOptions: {
        // Absolute paths so @typescript-eslint is happy
        project: path.join(__dirname, 'tsconfig.json'),
        tsconfigRootDir: __dirname,
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
    },

    plugins: {
      '@typescript-eslint': tsEslintPlugin,
      'react-hooks': reactHooks,
    },

    rules: {
      // Prefer the TS-aware unused-vars rule; warnings only.
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],

      // Allow `any` during early dev.
      '@typescript-eslint/no-explicit-any': 'off',

      // Let TypeScript handle these.
      'no-undef': 'off',
      'no-unused-vars': 'off',

      // React Hooks rules
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
    },
  },
]
