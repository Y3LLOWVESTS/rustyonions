// crates/svc-admin/ui/src/components/common/LoadingSpinner.tsx
//
// Compatibility shim so both paths work:
// - ../components/common/LoadingSpinner
// - ../components/shared/LoadingSpinner
//
// IMPORTANT: Avoid `export { X as default } from ...` shims here.
// They can break depending on bundler export resolution.

import React from 'react'
import { LoadingSpinner as SharedLoadingSpinner } from '../shared/LoadingSpinner'

export function LoadingSpinner() {
  return <SharedLoadingSpinner />
}

export default LoadingSpinner
