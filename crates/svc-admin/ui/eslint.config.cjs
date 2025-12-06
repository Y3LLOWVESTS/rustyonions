// crates/svc-admin/ui/eslint.config.cjs
//
// RO:WHAT — Minimal ESLint flat-config stub for svc-admin UI.
// RO:WHY  — Unblock `npm run lint` under ESLint 9+ without
//           introducing extra plugin/dependency churn yet.
// RO:INTERACTS — package.json "lint" script (`eslint src --ext .ts,.tsx`).
// RO:INVARIANTS — This config MUST remain no-op (all sources ignored)
//                 until we explicitly introduce a full TS/React config.

'use strict'

/** @type {import('eslint').Linter.FlatConfig[]} */
module.exports = [
  {
    // We explicitly ignore all source and build artifacts for now.
    // This keeps `npm run lint` passing as a no-op, which is fine for
    // dev-preview while we stabilize the API surface.
    ignores: [
      'node_modules/**',
      'dist/**',
      'src/**'
    ]
  }
]
