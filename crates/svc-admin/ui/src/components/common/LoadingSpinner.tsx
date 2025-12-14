// crates/svc-admin/ui/src/components/common/LoadingSpinner.tsx

// Re-export the shared spinner so both paths work:
// - ../components/common/LoadingSpinner
// - ../components/shared/LoadingSpinner

// NOTE: This file is a shim because vite keeps throwing an error 

export { LoadingSpinner as default } from "../shared/LoadingSpinner";
export { LoadingSpinner } from "../shared/LoadingSpinner";
