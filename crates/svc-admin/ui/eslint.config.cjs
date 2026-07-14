// crates/svc-admin/ui/eslint.config.cjs
//
// RO:WHAT — ESLint 9 flat configuration for the svc-admin TypeScript UI.
// RO:WHY — Keep React hook usage and TypeScript source lintable without
//          relying on the removed legacy .eslintrc configuration path.
// RO:INVARIANTS — Build artifacts and dependencies are ignored. Linting does
//                 not fabricate runtime truth or alter node behavior.

const typescriptParser = require('@typescript-eslint/parser')
const typescriptPlugin = require('@typescript-eslint/eslint-plugin')
const reactHooksPlugin = require('eslint-plugin-react-hooks')

module.exports = [
  {
    ignores: [
      'dist/**',
      'node_modules/**',
      'coverage/**',
    ],
  },
  {
    files: ['src/**/*.{ts,tsx}'],

    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
      },
    },

    plugins: {
      '@typescript-eslint': typescriptPlugin,
      'react-hooks': reactHooksPlugin,
    },

    rules: {
      // TypeScript owns variable analysis for TS/TSX files.
      'no-unused-vars': 'off',

      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
          ignoreRestSiblings: true,
        },
      ],

      // Invalid hook ordering is a correctness failure.
      'react-hooks/rules-of-hooks': 'error',

      // Dependency findings remain visible during the first flat-config
      // rollout without converting historical cleanup into a build blocker.
      'react-hooks/exhaustive-deps': 'warn',
    },
  },
]
