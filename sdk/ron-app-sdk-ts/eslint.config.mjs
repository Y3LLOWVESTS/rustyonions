import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default [
  // Ignore generated / external files
  {
    ignores: ['dist', 'coverage', 'node_modules'],
  },

  // TS + JS configs
  ...tseslint.config(js.configs.recommended, ...tseslint.configs.recommended, {
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
    },
    rules: {
      'no-console': 'warn',
      'no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-explicit-any': 'error',
    },
  }),
];
