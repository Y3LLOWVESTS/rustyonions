### BEGIN NOTE - JULY 22 2026 - 22:20 CST

# Passport Native Buildplan — Carry-Over Notes

## Session Status

We are currently working on the **Passport Native Buildplan** inside the existing `svc-passport` crate.

The important design correction made at the start of this work is that Passport native support must be folded into the existing `crates/svc-passport` crate, not built as a new crate. `svc-passport` remains the canonical Passport service owner. The native Passport work is being introduced behind the `native-passport` feature, with default builds staying clean and native surfaces staying isolated until they are ready.

Current completed status:

* Phase 0: Complete
* Phase 1: Complete
* Phase 2: Complete
* Phase 3: Complete
* Phase 4: Complete
* Phase 5: Complete
* Phase 6: Complete
* Phase 7A: Green
* Next session start: `NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO`

The most recent terminal output confirmed:

* `NATIVE_PASSPORT_PHASE7A_RESTORE_CONTRACT_DTO=GREEN`
* `RESTORE_CONTRACT_DTOS=GREEN`
* `OVERBROAD_SEED_PHRASE_FIELD_GUARD=REPAIRED`
* `RESTORE_EXECUTION_ADDED=NO`
* `MNEMONIC_OR_SEED_IMPORT_ADDED=NO`
* `KEY_DERIVATION_RUNTIME_ADDED=NO`
* `PIN_VALIDATION_RUNTIME_ADDED=NO`
* `PIN_DERIVATION_RUNTIME_ADDED=NO`
* `VAULT_UNLOCK_ADDED=NO`
* `VAULT_RUNTIME_ADDED=NO`
* `PLATFORM_SEALER_IMPLEMENTATION_ADDED=NO`
* `SECRET_STORAGE_ADDED=NO`
* `MATERIAL_EXPORT_ADDED=NO`
* `ENCRYPTION_RUNTIME_ADDED=NO`
* `DECRYPTION_RUNTIME_ADDED=NO`
* `DELEGATED_ENROLLMENT_RUNTIME_ADDED=NO`
* `RUNTIME_IO_ADDED=NO`
* `ROUTES_ADDED=NO`
* `CAPABILITY_ISSUANCE_ADDED=NO`
* `RUNTIME_AUTHORITY_CHANGED=NO`
* `NATIVE_SECRET_IMPLEMENTATION_ADDED=NO`
* `NEXT_SESSION_START=NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO`

Important caveat:

Phase 7A repair verification ran the focused `svc-passport` checks and predecessor tests, but it did **not** rerun `cargo check --workspace`. The last confirmed workspace green was Phase 6C. At the next session start, run `cargo check --workspace` first or include it early in the Phase 7B verification lane.

---

# Current Buildplan Intent

The active design is a wallet-like, native Passport model:

* No cloud-login Passport authority.
* No backend-custodied Passport secrets.
* No server-side password account pretending to be Passport source of truth.
* Native/client custody is the direction.
* Recovery/root and device key surfaces are local/native surfaces.
* `@username` is a handle, not a legal/real-name authority.
* Real names, if ever supported, are optional profile text only and must never become a signed identity pointer or restore/recovery authority.
* Native Passport work must stay feature-gated until real runtime surfaces are intentionally added.
* Contract DTOs must not pretend to execute behavior before runtime implementation exists.

The current implementation has intentionally moved through contract/DTO/review foundations before adding runtime behavior.

---

# Active Workflow Rules

Continue using the established QuickChain-style patch cadence:

1. Patch a small real behavior surface.
2. Add focused tests beside the behavior.
3. Run the focused test.
4. Run directly related predecessor tests.
5. Run `cargo check -p svc-passport`.
6. Run `cargo check -p svc-passport --features native-passport`.
7. Run `cargo check --workspace` at phase closures or after meaningful surface additions.
8. Fix the first failure before expanding scope.

Patch style:

* Use paste-ready Bash mutation blocks.
* Use separate verification blocks.
* Do not use compressed, encoded, base64, gzip, lzma, or opaque payload wrappers.
* Do not include `git add`, `git commit`, or `git push` unless explicitly requested.
* Do not end scripts with `exit`, because it can close the terminal.
* Keep scripts human-readable and inspectable.
* Print APPLIED/GREEN/RED banners.
* Make source guards targeted enough to avoid false positives on valid DTO/string-boundary assertions.
* Prefer exact runtime-shaped forbidden patterns over broad token scans.

---

# Files Added or Changed So Far

## Phase 0 files

Added or used:

* `crates/svc-passport/src/native_plan.rs`
* `crates/svc-passport/tests/native_passport_phase0_authority.rs`
* `crates/svc-passport/tests/vectors/native_passport_phase0b_inventory.json`
* `crates/svc-passport/tests/native_passport_phase0b_vector_inventory.rs`
* Root identity vector/test files
* Device authorization vector/test files
* Vault header vector/test files
* Phase 0 acceptance test

Phase 0 established static contracts, public vectors, identifiers, challenge/proof transcript anchors, and vault header fixtures without runtime behavior.

## Phase 1 files

Added or changed:

* `crates/svc-passport/src/native/mod.rs`
* `crates/svc-passport/src/native/dto.rs`
* `crates/svc-passport/src/native/status.rs`
* `crates/svc-passport/src/http/router.rs`
* `crates/svc-passport/src/kms/mod.rs`
* Phase 1 tests for feature gates, DTOs, posture/status, KMS injection, feature matrix acceptance

Phase 1 introduced feature-gated native surfaces, isolated KMS injection, DTO foundations, redacted status posture, and feature matrix acceptance.

## Phase 2 files

Added or changed:

* `crates/ron-proto/src/id/passport.rs`
* `crates/ron-proto/src/id/mod.rs`
* `crates/ron-naming/src/passport_username.rs`
* `crates/ron-naming/src/lib.rs`
* `crates/svc-passport/src/native/dto.rs`
* `crates/svc-passport/src/native/username.rs`
* `crates/svc-passport/src/profile.rs` or profile normalization surface
* `crates/svc-passport/Cargo.toml`
* Phase 2 tests for proto ID reuse, username handle reuse, profile username reconciliation, and Phase 2 acceptance

Phase 2 moved canonical Passport IDs into `ron-proto`, reused `ron-naming` for native handle rules, and reconciled legacy `svc-passport` profile username rules where safe.

## Phase 3 files

Added or changed:

* `crates/svc-passport/src/native/authorization.rs`
* `crates/svc-passport/src/native/mod.rs`
* `crates/svc-passport/tests/native_passport_phase3a_root_device_authorization_dto_foundation.rs`
* `crates/svc-passport/tests/native_passport_phase3b_pure_authorization_review.rs`
* `crates/svc-passport/tests/native_passport_phase3c_authorization_acceptance.rs`

Phase 3 introduced root/device authorization DTOs and pure deterministic authorization review without signature verification runtime.

## Phase 4 files

Added or changed:

* `crates/svc-passport/src/native/recovery.rs`
* `crates/svc-passport/src/native/device_key.rs`
* `crates/svc-passport/src/native/mod.rs`
* `crates/svc-passport/tests/native_passport_phase4a_native_recovery_root_dto.rs`
* `crates/svc-passport/tests/native_passport_phase4b_device_key_foundation_dto.rs`
* `crates/svc-passport/tests/native_passport_phase4c_native_root_device_acceptance.rs`

Phase 4 introduced recovery-root DTOs, device-key DTOs, validation/review functions, and acceptance proving public recovery-root → device-key flow while still adding no key generation, storage, signing, verification, vault runtime, or sealer runtime.

## Phase 5 files

Added or changed:

* `crates/svc-passport/src/native/sealer.rs`
* `crates/svc-passport/src/native/secure_surface.rs`
* `crates/svc-passport/src/native/mod.rs`
* `crates/svc-passport/tests/native_passport_phase5a_platform_sealer_contract_dto.rs`
* `crates/svc-passport/tests/native_passport_phase5b_secure_surface_compartment_contract.rs`
* `crates/svc-passport/tests/native_passport_phase5c_sealer_surface_acceptance.rs`

Phase 5 introduced:

* Platform sealer contract DTOs.
* Platform family labels:

  * macOS Keychain
  * iOS Keychain
  * Android Keystore
  * Windows DPAPI
  * Linux Secret Service
  * UnknownLocal
* Secure compartment labels:

  * RecoveryRoot
  * DeviceKey
* Secure-surface compartment contract DTOs.
* Stable compartment labels:

  * `passport_root_compartment`
  * `device_compartment`
* Acceptance from platform sealer contract → secure-surface contract.
* No platform sealer implementation.
* No secret storage.
* No material export.
* No encryption/decryption runtime.
* No vault runtime.
* No runtime I/O.
* No routes.
* No signing or verification runtime.
* No capability issuance.
* No wallet/ledger mutation.

A Phase 5B unused import warning appeared and was cleaned in the Phase 5C repair. The original cleanup guard was too broad because it matched a valid source-boundary assertion string. The repair changed the guard to inspect only the import block.

Phase 5C ended with:

* `NATIVE_PASSPORT_PHASE5=COMPLETE`
* `WORKSPACE_CHECK=GREEN`

## Phase 6 files

Added or changed:

* `crates/svc-passport/src/native/vault.rs`
* `crates/svc-passport/src/native/pin.rs`
* `crates/svc-passport/src/native/mod.rs`
* `crates/svc-passport/tests/native_passport_phase6a_two_compartment_vault_header_dto.rs`
* `crates/svc-passport/tests/native_passport_phase6b_pin_unlock_contract_dto.rs`
* `crates/svc-passport/tests/native_passport_phase6c_vault_contract_acceptance.rs`

Phase 6 introduced:

* Two-compartment vault header DTOs.
* Vault header domain:

  * `native-passport/two-compartment-vault-header/v1`
* Vault header version:

  * `1`
* KDF algorithm label:

  * `argon2id`
* AEAD algorithm label:

  * `xchacha20poly1305`
* Authenticated header label:

  * `vault-header-v1-authenticated`
* KDF salt length:

  * `16`
* AEAD nonce length:

  * `24`
* AEAD tag length:

  * `16`
* Required vault compartment headers:

  * RecoveryRoot bound to `passport_root_compartment`
  * DeviceKey bound to `device_compartment`
* PIN unlock contract DTOs.
* PIN policy:

  * minimum PIN length: `6`
  * maximum PIN length: `64`
  * max unlock attempts: `10`
  * cooldown seconds: `300`
  * KDF label: `argon2id`

Phase 6 acceptance proved:

* Phase 6A vault header DTO review works.
* Phase 6B PIN unlock contract DTO review works.
* Secure-surface → vault header → PIN contract flow works.
* Header metadata and PIN policy metadata are locked.
* Rejection boundaries are covered.
* No PIN validation runtime.
* No PIN derivation runtime.
* No PIN secret storage.
* No vault unlock.
* No vault runtime.
* No platform sealer implementation.
* No secret storage.
* No material export.
* No encryption/decryption runtime.
* No runtime I/O.
* No routes.
* No signing or verification runtime.
* No capability issuance.
* No wallet/ledger mutation.

Phase 6C ended with:

* `NATIVE_PASSPORT_PHASE6=COMPLETE`
* `WORKSPACE_CHECK=GREEN`

## Phase 7A files

Added or changed:

* `crates/svc-passport/src/native/restore.rs`
* `crates/svc-passport/src/native/mod.rs`
* `crates/svc-passport/tests/native_passport_phase7a_restore_contract_dto.rs`

Phase 7A introduced restore contract DTOs only.

Added restore concepts:

* Restore contract domain:

  * `native-passport/restore-contract/v1`
* Restore contract version:

  * `1`
* Restore intents:

  * `RestorePassportRoot`
  * `RestoreDeviceKey`
  * `PrepareDelegatedEnrollment`
* Restore sources:

  * `LocalVaultHeader`
  * `PlatformSealerContract`
  * `DelegatedDeviceInvitation`
* Restore descriptor:

  * `NativeRestoreContractDescriptorV1`
* Restore draft:

  * `NativeRestoreContractDraftV1`
* Restore review errors:

  * domain mismatch
  * version mismatch
  * platform mismatch
  * PIN contract domain mismatch
  * PIN contract version mismatch
  * vault header domain mismatch
  * vault header version mismatch
  * missing restore intents
  * duplicate restore intents
  * missing restore sources
  * duplicate restore sources
  * missing required restore intent
  * unsupported restore source
  * unsafe restore authority flag
* Restore posture:

  * `NativePassportRestorePosture`
* Functions:

  * `native_passport_restore_posture`
  * `validate_native_restore_contract_descriptor`
  * `review_native_restore_contract_draft`

Phase 7A added native surface:

* `NativePassportSurface::RestoreContractDto`
* `PHASE7A_ENABLED_SURFACES`

Phase 7A explicitly did **not** add:

* restore execution
* mnemonic import
* seed phrase import
* root key derivation runtime
* device key derivation runtime
* PIN validation runtime
* PIN derivation runtime
* vault unlock
* vault runtime
* platform sealer implementation
* secret storage
* material export
* encryption runtime
* decryption runtime
* delegated enrollment runtime
* runtime I/O
* routes
* capability issuance
* runtime authority changes
* native secret implementation
* wallet/ledger mutation

Phase 7A initially failed one source-boundary test because the DTO flag `imports_mnemonic_or_seed_phrase` created the field-shaped substring `seed_phrase:` in source. This was a false-positive guard issue, not a behavior failure.

Repair:

* Renamed `imports_mnemonic_or_seed_phrase` to `imports_recovery_phrase_material`.
* Confirmed old field name removed.
* Confirmed no `seed_phrase:` field-shaped pattern remains.
* Confirmed no `mnemonic:` field-shaped pattern remains.
* Confirmed restore source/test still keep the recovery phrase boundary flag.
* Reran Phase 7A default test.
* Reran Phase 7A native feature test.
* Reran Phase 6C, 6B, and 6A predecessor tests.
* Reran `cargo check -p svc-passport`.
* Reran `cargo check -p svc-passport --features native-passport`.

Phase 7A final result:

* `NATIVE_PASSPORT_PHASE7A_RESTORE_CONTRACT_DTO=GREEN`
* `RESTORE_CONTRACT_DTOS=GREEN`
* `OVERBROAD_SEED_PHRASE_FIELD_GUARD=REPAIRED`
* Next session:

  * `NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO`

Important caveat:

The Phase 7A repair verification did not run `cargo check --workspace`. The next session should start with a workspace check or include it in the first verification lane before moving too far.

---

# Current Green Test/Check State

Latest confirmed green checks:

* `cargo test -p svc-passport --test native_passport_phase7a_restore_contract_dto`
* `cargo test -p svc-passport --features native-passport --test native_passport_phase7a_restore_contract_dto`
* `cargo test -p svc-passport --features native-passport --test native_passport_phase6c_vault_contract_acceptance`
* `cargo test -p svc-passport --features native-passport --test native_passport_phase6b_pin_unlock_contract_dto`
* `cargo test -p svc-passport --features native-passport --test native_passport_phase6a_two_compartment_vault_header_dto`
* `cargo check -p svc-passport`
* `cargo check -p svc-passport --features native-passport`

Latest confirmed workspace check:

* Green at Phase 6C:

  * `cargo check --workspace`

Not yet rerun after Phase 7A repair:

* `cargo check --workspace`

Recommended first command next session:

```bash
cargo check --workspace
```

Then proceed to Phase 7B if green.

---

# Phase-by-Phase Summary

## Phase 0 — Source, Authority, and Cryptographic Spec Reconciliation

Purpose:

Establish native Passport source/authority/spec posture before adding implementation behavior.

Completed:

* Created a native plan/spec surface.
* Locked public vectors and DTO posture.
* Added public BIP39 all-abandon fixture.
* Locked Passport ID format.
* Locked Device ID format.
* Locked device authorization vector.
* Locked challenge and proof transcript anchors.
* Locked request proof vector.
* Locked vault header vector.
* Closed Phase 0 with acceptance.

Important locked values:

Passport ID:

```text
passport:v1:main:ed25519:b3:acc2761e583fafc93cbb880bef1bd7285f43b3bbf326b9e185b226c5533cb7df
```

Device ID:

```text
device:v1:ed25519:b3:4c18b950feb56bcad2579821d89aee76b259c28f77d459d5fec33bedd3f41f2d
```

Root public key:

```text
3d7f7a7cf1ca3e1af8e812d2ac349b13770d152c3f26b72560ee6870b9dec909
```

Device public key:

```text
2dfbfd60452275c726f8beb1a3d6ff9e91abbe670977716225807e4645044b17
```

Challenge B3:

```text
58577b7fde04e28ccce564ea08ed46575bb00f60924a9c1cf8d7546497b1148e
```

Proof transcript B3:

```text
66eb9fbd0da6e2425978405a9e32094bbf8bbd078a2c62021fc301d622afdd3e
```

Phase 0 outcome:

* Complete.
* No runtime authority added.
* No wallet/ledger mutation.
* No vault runtime.
* No route integration.

## Phase 1 — svc-passport Feature Isolation and KMS Injection

Purpose:

Isolate native Passport behind feature gates and keep default service behavior clean.

Completed:

* Added `native-passport` feature gate.
* Added native module.
* Added native DTO types.
* Added feature posture.
* Added redacted native status posture.
* Reworked KMS/router injection so default router uses dev KMS only through explicit seam.
* Added feature matrix acceptance.

Important lessons:

* Keep native module invisible in default build.
* Feature-gated tests must verify native surfaces compile only with `--features native-passport`.
* Do not back-mutate earlier surface constants when adding later phase surfaces.

Phase 1 outcome:

* Complete.
* Default build remains gated.
* Native feature build exposes DTO/posture/status surfaces.
* No native routes added.

## Phase 2 — ron-proto Canonical IDs and ron-naming Reuse

Purpose:

Prevent duplicate identity/handle logic inside `svc-passport`.

Completed:

* Moved/reused canonical Passport ID parsing through `ron-proto`.
* Added native Passport username handle parsing in `ron-naming`.
* Reused `ron-naming` native username handles in `svc-passport`.
* Reconciled profile username rules where safe.
* Reserved `site` and `ledger` appropriately.
* Added Phase 2 acceptance.

Important design decision:

* `@username` is a handle.
* It is not a real/legal name.
* It must not become a recovery key, wallet authority, or signed mutable pointer.
* Legacy profile hyphen behavior was documented rather than force-migrated in this phase.

Phase 2 outcome:

* Complete.
* Canonical ID reuse is in place.
* Native username handle reuse is in place.
* Profile username reconciliation is partially done without route migration.

## Phase 3 — Root/Device Authorization DTOs and Pure Authorization Review

Purpose:

Define root/device authorization DTOs and pure local review without signature verification runtime.

Completed:

* Added root Passport descriptor.
* Added device authorization draft/grant DTOs.
* Added pure authorization review.
* Rejected Passport mismatch.
* Rejected root public key mismatch.
* Rejected duplicate/collapsed keys.
* Rejected unsafe authority flags.
* Added Phase 3 acceptance.

Phase 3 outcome:

* Complete.
* Pure review works.
* No signature verification runtime added.
* No routes added.
* No capability issuance.

## Phase 4 — Native Recovery Root and Device-Key Foundation

Purpose:

Define recovery-root and device-key DTO foundations before actual key generation/storage.

Completed:

* Added recovery-root DTOs.
* Added recovery-root draft review.
* Added device-key DTOs.
* Added device-key draft review.
* Added rejection boundaries:

  * domain mismatch
  * missing scopes/purposes
  * duplicate scopes/purposes
  * key collision
  * unsafe authority flags
* Added Phase 4 acceptance.

Phase 4 outcome:

* Complete.
* Recovery-root and device-key contract surfaces exist.
* No recovery root generation.
* No device key generation.
* No secret storage.
* No platform sealer.
* No vault runtime.
* No signing or verification runtime.

## Phase 5 — Platform Sealer and Native Secure Surface Contracts

Purpose:

Define platform sealer and secure-surface compartment contracts before platform-specific secure storage is implemented.

Completed:

* Phase 5A: Platform sealer contract DTO.
* Phase 5B: Secure-surface compartment contract DTO.
* Phase 5C: Sealer/surface acceptance.

Key surfaces:

* `NativePlatformFamily`
* `NativeSecureCompartment`
* `NativePlatformSealerContractDescriptorV1`
* `NativePlatformSealerContractDraftV1`
* `NativeSecureSurfaceCompartmentBindingV1`
* `NativeSecureSurfaceContractDescriptorV1`
* `NativeSecureSurfaceContractDraftV1`

Important labels:

* `native-passport/platform-sealer-contract/v1`
* `native-passport/secure-surface-compartment/v1`
* `passport_root_compartment`
* `device_compartment`

Phase 5 outcome:

* Complete.
* Workspace check green.
* Contract DTOs only.
* No platform sealer implementation.
* No storage/export/runtime authority.

## Phase 6 — Two-Compartment Vault and PIN Unlock Contracts

Purpose:

Define vault header and PIN policy contracts before any PIN unlock or encryption/decryption runtime exists.

Completed:

* Phase 6A: Two-compartment vault header DTO.
* Phase 6B: PIN unlock contract DTO.
* Phase 6C: Vault contract acceptance.

Key surfaces:

* `NativeVaultCompartmentHeaderV1`
* `NativeTwoCompartmentVaultHeaderDescriptorV1`
* `NativeTwoCompartmentVaultHeaderDraftV1`
* `NativePinPolicyV1`
* `NativePinUnlockContractDescriptorV1`
* `NativePinUnlockContractDraftV1`

Locked vault header metadata:

* Domain: `native-passport/two-compartment-vault-header/v1`
* Version: `1`
* KDF: `argon2id`
* AEAD: `xchacha20poly1305`
* Authenticated header label: `vault-header-v1-authenticated`
* KDF salt length: `16`
* AEAD nonce length: `24`
* AEAD tag length: `16`

Locked PIN policy metadata:

* Domain: `native-passport/pin-unlock-contract/v1`
* Version: `1`
* Minimum PIN length: `6`
* Maximum PIN length: `64`
* Maximum attempts: `10`
* Cooldown seconds: `300`
* KDF label: `argon2id`

Phase 6 outcome:

* Complete.
* Workspace check green.
* Header/policy contracts only.
* No PIN validation.
* No PIN derivation.
* No vault unlock.
* No vault runtime.
* No encryption/decryption.
* No storage/export.
* No runtime authority.

## Phase 7A — Restore Contract DTO

Purpose:

Define restore/new-device/delegated-enrollment contract metadata before actual restore or enrollment runtime exists.

Completed:

* Added restore contract source.
* Added restore contract tests.
* Added native restore surface exports.
* Added restore posture.
* Added review and validation functions.
* Repaired false-positive `seed_phrase:` source guard.

Key surfaces:

* `NativeRestoreIntent`
* `NativeRestoreSource`
* `NativeRestoreContractDescriptorV1`
* `NativeRestoreContractDraftV1`
* `NativeRestoreContractReviewError`
* `NativePassportRestorePosture`
* `native_passport_restore_posture`
* `review_native_restore_contract_draft`
* `validate_native_restore_contract_descriptor`

Locked restore contract metadata:

* Domain: `native-passport/restore-contract/v1`
* Version: `1`

Required restore intents:

* `RestorePassportRoot`
* `RestoreDeviceKey`
* `PrepareDelegatedEnrollment`

Allowed restore sources:

* `LocalVaultHeader`
* `PlatformSealerContract`
* `DelegatedDeviceInvitation`

Phase 7A final outcome:

* Green.
* No restore execution.
* No mnemonic/seed import.
* No key derivation runtime.
* No PIN validation/derivation runtime.
* No vault unlock/runtime.
* No platform sealer implementation.
* No storage/export.
* No encryption/decryption.
* No delegated enrollment runtime.
* No routes.
* No capabilities.
* No runtime authority.

---

# Important Guard/Test Lessons From This Session

## Avoid broad token scans

Several failures were caused by broad source scans matching valid DTO source, comments, or test assertions.

Examples:

* `NativeSecureSurfaceContractDescriptorV1` appeared in a valid source-boundary assertion, not as an unused import.
* `seed_phrase:` appeared because a DTO field was named `imports_mnemonic_or_seed_phrase`.

Preferred approach:

* Scan for runtime-shaped patterns:

  * `encrypt(`
  * `decrypt(`
  * `unlock_vault(`
  * `derive_pin(`
  * `platform_seal(`
  * `Router::`
  * `.route(`
  * `wallet.spend(`
  * `ledger.write(`
* For unused imports, inspect only the import block rather than grepping the whole file.
* For secret-like names, avoid field names that produce `secret_name:` patterns unless the field really is a secret field.

## Keep boundary flags clear but not secret-shaped

Good replacement from Phase 7A:

* Old:

  * `imports_mnemonic_or_seed_phrase`
* New:

  * `imports_recovery_phrase_material`

This keeps the boundary meaning but avoids field-shaped patterns like `seed_phrase:` and `mnemonic:`.

## Preserve contract-only posture

Every phase so far has intentionally stayed contract/DTO/review-only until runtime is deliberately introduced.

Do not accidentally add:

* secret material
* private keys
* PIN values
* PIN hashes
* derived keys
* ciphertext/plaintext fields
* sealed bytes
* route handlers
* runtime I/O
* encryption/decryption calls
* platform sealer calls
* vault unlock calls
* capability issuance
* wallet/ledger mutation

---

# Next Session Start

## First action

Because Phase 7A repair did not rerun workspace, start with:

```bash
cargo check --workspace
```

If green, proceed to:

```text
NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO
```

If workspace check fails, fix the first workspace failure before starting Phase 7B.

---

# Next Patch: Phase 7B — New Device Enrollment Contract DTO

## Goal

Add a contract-only DTO surface for new-device enrollment.

This should model the metadata and review boundaries for preparing a new device enrollment flow, but **not** execute enrollment, generate device keys, sign challenges, issue capabilities, contact live RPC, mutate storage, or add routes.

Phase 7B should build on Phase 7A restore contract DTOs.

## Suggested new file

Likely add:

```text
crates/svc-passport/src/native/enrollment.rs
```

Alternative names:

* `device_enrollment.rs`
* `new_device.rs`

Recommended: `enrollment.rs`, because later Phase 7C may add delegated enrollment contract acceptance and can reuse the module cleanly.

## Suggested native module changes

Update:

```text
crates/svc-passport/src/native/mod.rs
```

Add:

```rust
pub mod enrollment;
```

Add re-exports for the new DTOs, constants, posture, review function, and validation function.

Add new surface:

```rust
NativePassportSurface::NewDeviceEnrollmentContractDto
```

Add:

```rust
PHASE7B_ENABLED_SURFACES
```

This should extend `PHASE7A_ENABLED_SURFACES` without back-mutating it.

## Suggested test file

Add:

```text
crates/svc-passport/tests/native_passport_phase7b_new_device_enrollment_contract_dto.rs
```

## Suggested constants

```rust
pub const NATIVE_PASSPORT_PHASE7B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO";

pub const PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN: &str =
    "native-passport/new-device-enrollment-contract/v1";

pub const PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION: u16 = 1;
```

## Suggested enums

```rust
pub enum NativeEnrollmentIntent {
    RequestDeviceEnrollment,
    BindDeviceAuthorization,
    PrepareDeviceCapability,
}
```

Potential enrollment sources:

```rust
pub enum NativeEnrollmentSource {
    RestoreContract,
    DelegatedDeviceInvitation,
    LocalRootApproval,
}
```

Potential enrollment target class:

```rust
pub enum NativeEnrollmentDeviceClass {
    TvReadOnly,
    DesktopReadOnly,
    MobileReadOnly,
}
```

But note: `DeviceClass` may already exist from earlier native DTO work. Prefer reusing existing `DeviceClass` if available rather than inventing a duplicate.

## Suggested DTOs

```rust
pub struct NativeNewDeviceEnrollmentContractDescriptorV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub platform_family: NativePlatformFamily,
    pub restore_contract_domain: &'static str,
    pub restore_contract_version: u16,
    pub enrollment_intents: Vec<NativeEnrollmentIntent>,
    pub enrollment_sources: Vec<NativeEnrollmentSource>,
    pub contract_only: bool,
}
```

```rust
pub struct NativeNewDeviceEnrollmentContractDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub platform_family: NativePlatformFamily,
    pub restore_contract_domain: &'static str,
    pub restore_contract_version: u16,
    pub requested_enrollment_intents: Vec<NativeEnrollmentIntent>,
    pub requested_enrollment_sources: Vec<NativeEnrollmentSource>,

    pub requests_enrollment_execution: bool,
    pub requests_device_key_generation: bool,
    pub requests_device_authorization_signature: bool,
    pub requests_capability_issuance: bool,
    pub requests_restore_execution: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_pin_validation_runtime: bool,
    pub requests_pin_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub includes_platform_sealer_implementation: bool,
    pub stores_secret_material: bool,
    pub exports_secret_or_material: bool,
    pub requests_encryption_or_decryption: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}
```

Avoid field names that create secret-shaped patterns like:

* `seed_phrase:`
* `mnemonic:`
* `private_key:`
* `device_private_key:`
* `root_private_key:`
* `pin:`
* `pin_hash:`
* `ciphertext:`
* `plaintext:`
* `sealed_bytes:`

## Suggested errors

```rust
pub enum NativeNewDeviceEnrollmentContractReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    PlatformFamilyMismatch,
    RestoreContractDomainMismatch,
    RestoreContractVersionMismatch,
    MissingEnrollmentIntents,
    DuplicateEnrollmentIntent,
    MissingRequiredEnrollmentIntent,
    MissingEnrollmentSources,
    DuplicateEnrollmentSource,
    UnsupportedEnrollmentSource,
    UnsafeEnrollmentAuthorityFlag,
}
```

## Suggested posture

```rust
pub struct NativePassportNewDeviceEnrollmentPosture {
    pub phase_label: &'static str,
    pub new_device_enrollment_contract_dtos_added: bool,
    pub enrollment_execution_added: bool,
    pub device_key_generation_added: bool,
    pub device_authorization_signature_added: bool,
    pub capability_issuance_added: bool,
    pub restore_execution_added: bool,
    pub key_derivation_runtime_added: bool,
    pub pin_validation_runtime_added: bool,
    pub pin_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_implementation_added: bool,
    pub secret_storage_added: bool,
    pub material_export_added: bool,
    pub encryption_runtime_added: bool,
    pub decryption_runtime_added: bool,
    pub runtime_io_added: bool,
    pub routes_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}
```

## Suggested review function

```rust
pub fn review_native_new_device_enrollment_contract_draft(
    restore_contract: &NativeRestoreContractDescriptorV1,
    draft: NativeNewDeviceEnrollmentContractDraftV1,
) -> Result<NativeNewDeviceEnrollmentContractDescriptorV1, NativeNewDeviceEnrollmentContractReviewError>
```

The function should:

* verify contract domain
* verify contract version
* verify platform family matches restore contract
* verify restore contract domain/version match
* validate enrollment intents
* validate enrollment sources
* reject unsafe flags
* return descriptor with `contract_only: true`

## Suggested validation function

```rust
pub fn validate_native_new_device_enrollment_contract_descriptor(
    restore_contract: &NativeRestoreContractDescriptorV1,
    descriptor: &NativeNewDeviceEnrollmentContractDescriptorV1,
) -> Result<(), NativeNewDeviceEnrollmentContractReviewError>
```

## Suggested tests

Phase 7B test should include:

1. Default feature gate test.
2. Label and posture locked.
3. `PHASE7B_ENABLED_SURFACES` extends `PHASE7A_ENABLED_SURFACES` without back-mutating it.
4. Valid restore contract → new-device enrollment contract flow.
5. Required enrollment intents and sources are locked.
6. Rejection tests for:

   * domain mismatch
   * version mismatch
   * platform mismatch
   * restore contract domain mismatch
   * restore contract version mismatch
   * missing enrollment intents
   * duplicate enrollment intents
   * missing required enrollment intent
   * missing enrollment sources
   * duplicate enrollment sources
7. Unsafe authority flag rejection:

   * enrollment execution
   * device key generation
   * device authorization signature
   * capability issuance
   * restore execution
   * key derivation runtime
   * PIN validation runtime
   * PIN derivation runtime
   * vault unlock
   * vault runtime
   * platform sealer implementation
   * secret storage
   * material export
   * encryption/decryption
   * runtime I/O
   * routes
   * wallet/ledger mutation
8. Source guard test:

   * no route patterns
   * no runtime patterns
   * no crypto calls
   * no restore execution
   * no enrollment execution
   * no device key generation
   * no signing
   * no verification
   * no storage/export
   * no secret-shaped fields

## Suggested verification lane for Phase 7B

Use focused commands:

```bash
cargo check --workspace
cargo fmt -p svc-passport
cargo test -p svc-passport --test native_passport_phase7b_new_device_enrollment_contract_dto
cargo test -p svc-passport --features native-passport --test native_passport_phase7b_new_device_enrollment_contract_dto
cargo test -p svc-passport --features native-passport --test native_passport_phase7a_restore_contract_dto
cargo test -p svc-passport --features native-passport --test native_passport_phase6c_vault_contract_acceptance
cargo check -p svc-passport
cargo check -p svc-passport --features native-passport
```

If this is only a focused Phase 7B slice, workspace check can be first because the last repair did not rerun it. If Phase 7B becomes a phase closure later, rerun workspace at the end too.

---

# After Phase 7B

## Phase 7C — Delegated Enrollment Contract DTO or Phase 7 Acceptance

The original buildplan says Phase 7 covers restore, new device, and delegated enrollment.

After Phase 7B, likely next should be one of:

```text
NATIVE_PASSPORT_PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DTO
```

or, if Phase 7B includes enough delegated enrollment contract coverage:

```text
NATIVE_PASSPORT_PHASE7C_RESTORE_NEW_DEVICE_DELEGATED_ENROLLMENT_ACCEPTANCE
```

Recommended path:

* Phase 7B: New-device enrollment contract DTO.
* Phase 7C: Delegated enrollment contract DTO.
* Phase 7D: Restore/new-device/delegated enrollment acceptance and workspace check.

## Phase 7C suggested intent

Define delegated enrollment invitation/approval contract DTOs, but no runtime.

Should cover:

* invitation contract domain/version
* invitation source labels
* approving authority label
* enrolled device target label
* challenge/proof placeholders as public metadata only
* expiry/capability policy metadata only if safe
* no signing
* no verification
* no capability issuance
* no route
* no runtime I/O
* no storage
* no key generation

## Phase 7D suggested acceptance

Acceptance should prove:

* Phase 7A restore contract review works.
* Phase 7B new-device enrollment contract review works.
* Phase 7C delegated enrollment contract review works.
* Restore → new-device enrollment → delegated enrollment contract flow is deterministic.
* All unsafe runtime flags are rejected.
* All sources remain contract-only.
* `cargo check --workspace` passes.

Expected phase closure:

```text
NATIVE_PASSPORT_PHASE7=COMPLETE
WORKSPACE_CHECK=GREEN
NEXT_PATCH=NATIVE_PASSPORT_PHASE8A_PURPOSE_BOUND_CHALLENGE_CONTRACT_DTO
```

---

# Later Buildplan Direction

## Phase 8 — Purpose-Bound Challenge and Proof Foundation

Likely work:

* Define purpose-bound challenge DTOs.
* Define challenge domains and labels.
* Define proof request contracts.
* Reject mismatched purposes/domains/versions.
* Still no real signing or verification runtime unless explicitly scoped.

Potential surfaces:

* challenge purpose:

  * restore
  * enroll device
  * authorize device
  * revoke device
  * request proof
* challenge metadata:

  * challenge ID
  * expiration policy
  * nonce label
  * proof purpose
  * bound Passport ID
  * bound Device ID when applicable

## Phase 9 — Native Root/Device Proof and Request-Proof Signing

This is where signing surfaces may begin, but only after contract DTOs and challenge foundation are complete.

Must be careful:

* Signing must be local/native.
* No server custody.
* No fake success.
* No backend-issued Passport source of truth.
* Must be deterministic and testable.
* Must reject mismatches and replay.

## Phase 10 — Durable Passport/Device Registry and Challenge Replay

Likely work:

* durable registry model
* challenge state
* replay rejection
* idempotency boundaries
* device registry persistence abstraction
* no live production mutation unless explicitly scoped

## Phase 11 — Device-Bound Capability and Request Proof

Likely work:

* device-bound capability DTOs
* local proof presentation
* purpose-bound proof verification
* no broad authority grants
* no wallet/ledger mutation

## Phase 12 — Private-Beta `@username` Registry

Likely work:

* registry integration for handles
* reserved name enforcement
* profile/handle distinction
* no real-name authority
* no recovery authority through handle
* possible profile route migration once native handle rules are stable

## Phase 13 — Public Profile and Site Namespace

Likely work:

* public profile namespace
* site namespace
* profile display rules
* non-authoritative display metadata
* separation from Passport recovery/root identity

## Phase 14 — Gateway and Omnigate Integration

Likely work:

* gateway consumes Passport proof/capability status
* Omnigate integration
* no server custody
* no fake validation
* typed failure surfaces

## Phase 15 — CrabLink Desktop Root-Capable Integration

Likely work:

* desktop native Passport creation/import/unlock
* local custody
* secure local storage integration
* root-capable behavior only on desktop

## Phase 16 — Android TV Delegated Integration

Likely work:

* Android TV delegated read-only/passport device model
* TV should not be root-capable by default
* delegated enrollment flow
* read-only scopes
* no root secret entry on TV unless explicitly designed

## Phase 17 — Regular Android and iOS Integration

Likely work:

* mobile local custody
* secure enclave/keystore/keychain integration
* import/unlock UX
* device key flow
* delegated and root-capable modes as designed

## Phase 18 — Public Namespace Finality, Hardening, and Final Acceptance

Likely work:

* final acceptance
* integration/chaos tests
* replay hardening
* mismatch hardening
* UX/route hardening
* workspace green
* local green runs

---

# Current Non-Negotiable Invariants

Continue preserving these:

* `svc-passport` is the Passport implementation home.
* Do not create a new Passport crate.
* Default build must not expose native Passport surfaces.
* Native Passport surfaces must remain behind `native-passport`.
* No cloud-stored Passport secrets.
* No backend-issued Passport source of truth.
* No real/legal name as identity authority.
* `@username` remains a handle.
* Recovery/root is not a profile field.
* Device keys are separate from recovery/root.
* Contract DTO phases must not fake runtime behavior.
* No restore execution until scoped.
* No key generation until scoped.
* No PIN validation/derivation until scoped.
* No vault unlock until scoped.
* No platform sealer implementation until scoped.
* No encryption/decryption runtime until scoped.
* No storage/export of secret material until scoped.
* No routes until scoped.
* No signing/verification runtime until scoped.
* No capability issuance until scoped.
* No wallet/ledger mutation.
* No live wallet calls.
* No live ledger mutation.
* No production deployment.
* No production mint/burn.
* No fake finality or fake success output.

---

# Practical Next-Session Checklist

Start next session with:

1. Confirm current branch/repo is still on the latest Phase 7A repair state.
2. Run:

```bash
cargo check --workspace
```

3. If green, begin:

```text
NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO
```

4. Patch only the Phase 7B surface.
5. Add `enrollment.rs` or equivalent.
6. Update native module exports and surface chain.
7. Add Phase 7B default/feature tests.
8. Run focused verification.
9. Run Phase 7A predecessor.
10. Run Phase 6C predecessor.
11. Run `cargo check -p svc-passport`.
12. Run `cargo check -p svc-passport --features native-passport`.
13. Run workspace check at either the start or end, preferably both if Phase 7B is larger than expected.

Expected next green marker:

```text
NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO=GREEN
NEXT_PATCH=NATIVE_PASSPORT_PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DTO
```

---

# Suggested First Terminal Command Next Session

```bash
cargo check --workspace
```

Then continue with the Phase 7B mutation.


### END NOTE - JULY 22 2026 - 22:20 CST




### BEGIN NOTE - JULY 23 2026 - 17:40 CST
Yes — since you meant **15D**, the notes should be updated to cover the full session span as **Phase 7A through Phase 15D**, with **Phase 15E** as the next marker. I used your uploaded notes as the base and expanded them to include the missing Phase 7A–13 work, while preserving the Phase 14–15D details and the important “Phase 15 runtime is not complete” warning.  The Phase 15 buildplan still expects real desktop create/restore/unlock/device/proof/capability/username behavior and React-only-redacted DTOs before desktop runtime is complete. 

Here is the updated replacement version:

# Native Passport Carry-Over Notes — End of Session

## Current status

We are working on the **Passport Native Buildplan** inside the existing canonical owner crate:

```text
crates/svc-passport
```

This session advanced the Passport Native build from **Phase 7A through Phase 15D**.

Latest confirmed green baseline:

```text
NATIVE_PASSPORT_PHASE15D_DESKTOP_COMMAND_SURFACE_ACCEPTANCE=GREEN
DESKTOP_COMMAND_SURFACE_CONTRACT_CHAIN=COMPLETE_GREEN_PARKED
WORKSPACE_CHECK_AFTER_PHASE15D=GREEN
NEXT_PATCH=NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION
```

Important distinction:

```text
PHASE15_DESKTOP_INTEGRATION_RUNTIME_COMPLETE=NO
```

So the current green result means the **desktop command surface contract chain** is complete and parked, not that full desktop runtime Passport behavior is implemented.

We must not claim that desktop Passport create/restore/unlock/proof/capability/username runtime is complete yet.

The next patch should start from:

```text
NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION
```

---

# Session scope

This session covered:

```text
Phase 7A  — Restore Contract DTO
Phase 7B  — New Device Enrollment Contract DTO
Phase 7C  — Delegated Enrollment Contract DTO
Phase 7D  — Restore/New Device/Delegated Enrollment Acceptance
Phase 8A  — Purpose-Bound Challenge Contract DTO
Phase 8B  — Proof Contract DTO
Phase 8C  — Proof Request Contract DTO
Phase 8D  — Challenge/Proof Acceptance
Phase 9A  — Native Proof Signing Contract Boundary
Phase 9B  — Native Proof Signing Adapter
Phase 9C  — Native Proof Signing Acceptance
Phase 10A — Native Proof Verification Contract Boundary
Phase 10B — Native Proof Verification Adapter
Phase 10C — Native Proof Verification Acceptance
Phase 11A — Replay and Challenge Consumption Contract
Phase 11B — Replay and Challenge Consumption Adapter
Phase 11C — Device-Bound Capability Contract
Phase 11D — Device-Bound Capability Adapter
Phase 11E — Capability Acceptance
Phase 12A — Private-Beta Username Registry Contract
Phase 12B — Private-Beta Username Registry Adapter
Phase 12C — Username Index Projection
Phase 12D — Username Private-Beta Acceptance
Phase 13  — Local Status and Inspection
Phase 14A — Gateway/Omnigate Route Contract
Phase 14B — Omnigate Fixed Route Admission
Phase 14C — Gateway Fixed Route Admission
Phase 14D — Gateway/Omnigate Route Mount Acceptance
Phase 15A — Client Status Command Contract
Phase 15B — Desktop Command Inventory and Denylist
Phase 15C — Desktop Redacted Command DTO Adapter
Phase 15D — Desktop Command Surface Acceptance
```

Final safe state:

```text
crate: svc-passport
latest green phase: Phase 15D
workspace status: GREEN
next patch: NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION
```

---

# Global Passport Native invariants preserved

Throughout this session, the work intentionally stayed on the contract/review/DTO side unless explicitly scoped otherwise.

Preserved invariants:

```text
no recovery phrase to React
no PIN to React
no root private key to React
no device private key to React
no vault key to React
no raw capability to React
no cloud login
no cloud-stored Passport secret
no backend custody of Passport secrets
no backend-issued identity pretending to be source of truth
no arbitrary scope issuance
no policy disabling
no live wallet/ledger mutation
no fake finality
no live route mount unless explicitly scoped
no live Tauri command wiring unless explicitly scoped
```

The native Passport model remains wallet-like and local-custody oriented.

---

# Crate scope

All implementation work in this session landed in:

```text
crates/svc-passport
```

Related regressions/checks touched:

```text
ron-naming
ron-proto
svc-passport
workspace crates through cargo check --workspace
```

But all new Passport Native session source/test changes were in `svc-passport`.

---

# Phase 7 work completed

## Phase 7A — Restore Contract DTO

Phase 7A established the restore contract DTO surface.

### Purpose

Model a restore contract boundary without actually restoring a Passport.

### What it proved

```text
restore contract DTO exists
restore metadata is reviewed
restore remains contract-only
unsafe restore/runtime flags are rejected
surface chain extends prior native phases
default build remains gated
native-passport feature build passes
```

### Important lesson from Phase 7A

A broad source scan caught secret-shaped field names such as `seed_phrase:` / `mnemonic:` even when they were used as safe boundary flags.

We corrected the naming style to avoid secret-shaped field patterns.

Good replacement pattern:

```text
old: imports_mnemonic_or_seed_phrase
new: imports_recovery_phrase_material
```

The lesson carried forward:

```text
avoid field names that produce forbidden source substrings like seed_phrase:, mnemonic:, pin_hash:, root_private_key:
use neutral boundary names like recovery_words, signing_material, capability_material, vault_material
scan for runtime-shaped behavior, not harmless DTO wording
```

### What Phase 7A did not add

```text
RESTORE_EXECUTION=NO
MNEMONIC_OR_SEED_IMPORT=NO
KEY_DERIVATION_RUNTIME=NO
PIN_VALIDATION_RUNTIME=NO
VAULT_UNLOCK=NO
VAULT_RUNTIME=NO
PLATFORM_SEALER_IMPLEMENTATION=NO
STORAGE_EXPORT=NO
ENCRYPTION_DECRYPTION=NO
DELEGATED_ENROLLMENT_RUNTIME=NO
ROUTES=NO
CAPABILITIES=NO
RUNTIME_AUTHORITY=NO
```

---

## Phase 7B — New Device Enrollment Contract DTO

Phase 7B added the new-device enrollment contract DTO surface.

### Purpose

Model a new-device enrollment contract without executing enrollment.

### What it proved

```text
new-device enrollment metadata is represented
restore contract boundary is reused
device enrollment remains contract-only
device key generation is not performed
device authorization signing is not performed
proof signing is not performed
proof verification is not performed
capability issuance is not performed
storage/runtime/route/wallet/ledger mutation is absent
surface chain extends Phase 7A
```

### What it did not add

```text
ENROLLMENT_EXECUTION=NO
DEVICE_KEY_GENERATION=NO
DEVICE_AUTHORIZATION_SIGNING=NO
PROOF_SIGNING=NO
PROOF_VERIFICATION=NO
CAPABILITY_ISSUANCE=NO
RESTORE_EXECUTION=NO
KEY_DERIVATION_RUNTIME=NO
PIN_RUNTIME=NO
VAULT_UNLOCK=NO
VAULT_RUNTIME=NO
PLATFORM_SEALER_IMPLEMENTATION=NO
RUNTIME_IO=NO
ROUTES=NO
WALLET_OR_LEDGER_MUTATION=NO
NATIVE_SECRET_CUSTODY=NO
```

---

## Phase 7C — Delegated Enrollment Contract DTO

Phase 7C added delegated enrollment contract DTO coverage.

### Purpose

Represent delegated enrollment invitation/approval boundaries without implementing runtime delegation.

### What it proved

```text
delegated enrollment contract metadata exists
delegated enrollment remains contract-only
approval labels and delegated device targets are modeled
challenge/proof placeholders remain public metadata only
no signing, verification, capability issuance, route, runtime I/O, storage, or key generation is performed
surface chain extends Phase 7B
```

### What it did not add

```text
DELEGATED_ENROLLMENT_RUNTIME=NO
INVITATION_RUNTIME=NO
DEVICE_KEY_GENERATION=NO
SIGNING=NO
VERIFICATION=NO
CAPABILITY_ISSUANCE=NO
ROUTES=NO
RUNTIME_IO=NO
STORAGE_MUTATION=NO
WALLET_OR_LEDGER_MUTATION=NO
```

---

## Phase 7D — Restore/New Device/Delegated Enrollment Acceptance

Phase 7D closed Phase 7.

### Purpose

Prove the Phase 7 restore, new-device enrollment, and delegated enrollment contract surfaces compose safely.

### What it proved

```text
restore contract review works
new-device enrollment contract review works
delegated enrollment contract review works
restore -> new device enrollment -> delegated enrollment contract flow is deterministic
unsafe runtime flags are rejected
all sources remain contract-only
Phase 7 surface chain is complete
```

### Phase 7 status

```text
PHASE7_RESTORE_NEW_DEVICE_DELEGATED_ENROLLMENT_ACCEPTANCE=GREEN
PHASE7=COMPLETE_GREEN_PARKED
```

---

# Phase 8 work completed

## Phase 8A — Purpose-Bound Challenge Contract DTO

Phase 8A added purpose-bound challenge DTOs.

### Purpose

Model challenges that are bound to a specific native Passport purpose.

### What it proved

```text
challenge contract domain/version is locked
challenge IDs/nonces are modeled as public metadata
challenge purpose is explicit
restore/enroll/device/proof purposes are not interchangeable
expired or mismatched challenge posture is rejected
contract-only posture is preserved
```

### What it did not add

```text
CHALLENGE_ISSUANCE_RUNTIME=NO
CHALLENGE_STORAGE_MUTATION=NO
SIGNING=NO
VERIFICATION=NO
ROUTES=NO
CAPABILITY_ISSUANCE=NO
```

---

## Phase 8B — Proof Contract DTO

Phase 8B added the proof contract DTO surface.

### Key proof kinds modeled

```text
RootRegistration
RootDeviceAuthorization
RootDeviceRevocation
DeviceSession
DeviceCapability
NamespaceTransition
```

### Key authorities modeled

```text
RootAuthority
DeviceAuthority
```

### What it proved

```text
proof kind is explicit
proof authority is explicit
proof domain/version is locked
proof is bound to challenge purpose
root/device authority boundaries are not interchangeable
contract-only posture is preserved
```

### What it did not add

```text
SIGNING_RUNTIME=NO
VERIFY_RUNTIME=NO
PRIVATE_KEY_ACCESS=NO
KEY_LOADING=NO
WALLET_OR_LEDGER_MUTATION=NO
```

---

## Phase 8C — Proof Request Contract DTO

Phase 8C added request-proof contract DTOs.

### Purpose

Model request-proof metadata and TTL/skew boundaries without implementing proof runtime.

### What it proved

```text
proof request contract domain/version is locked
request proof kind is bound to purpose
TTL and skew policies are explicit
wrong purpose/domain/version is rejected
contract-only posture is preserved
```

### What it did not add

```text
REQUEST_PROOF_TRANSPORT=NO
SIGNING_RUNTIME=NO
VERIFY_RUNTIME=NO
STORAGE_MUTATION=NO
ROUTES=NO
```

---

## Phase 8D — Challenge/Proof Acceptance

Phase 8D closed the challenge/proof/request-proof contract foundation.

### What it proved

```text
purpose-bound challenge contracts work
proof contract DTOs work
proof request contract DTOs work
challenge/proof/request-proof relationships are deterministic
mismatched domains/purposes/versions are rejected
contract-only posture remains intact
```

### Phase 8 status

```text
PHASE8_CHALLENGE_PROOF_ACCEPTANCE=GREEN
PHASE8=COMPLETE_GREEN_PARKED
```

---

# Phase 9 work completed

## Phase 9A — Native Proof Signing Contract Boundary

Phase 9A added the native proof-signing contract boundary.

### Purpose

Review whether a proof is eligible to be signed, without performing signing.

### What it proved

```text
challenge/proof/request contracts are reused
operation kind is explicit
unlock state is reviewed
wrong signer/algorithm/purpose/domain is rejected
locked/unsafe posture is rejected
contract-only posture is preserved
```

### What it did not add

```text
SIGNATURE_PRODUCTION=NO
PRIVATE_KEY_ACCESS=NO
KEY_LOADING=NO
KEY_DERIVATION=NO
VAULT_UNLOCK=NO
PLATFORM_SEALER_UNSEAL=NO
RUNTIME_IO=NO
STORAGE_MUTATION=NO
```

---

## Phase 9B — Native Proof Signing Adapter

Phase 9B added an injected local signing adapter contract.

### Purpose

Model an adapter boundary for producing a signed payload, while keeping signing injected and contract-reviewed.

### What it proved

```text
adapter draft/request/envelope bindings are deterministic
operation, authority, challenge, passport/device, proof/request domain, transcript, signer, algorithm, and evidence are bound
signed payload presence can be reviewed without leaking key material
unsafe signing evidence is rejected
```

### What it did not add

```text
BUILT_IN_SECRET_SIGNER=NO
KEY_LOADING=NO
KEY_DERIVATION=NO
VAULT_UNLOCK=NO
PLATFORM_SEALER_UNSEAL=NO
WALLET_OR_LEDGER_MUTATION=NO
```

---

## Phase 9C — Native Proof Signing Acceptance

Phase 9C closed the native signing contract/adapter chain.

### What it proved

```text
proof signing contract boundary works
proof signing adapter works
signed payload envelope can be reviewed deterministically
unsafe signing states are rejected
contract-only/injected-adapter posture is preserved
```

### Phase 9 status

```text
PHASE9_NATIVE_PROOF_SIGNING_ACCEPTANCE=GREEN
PHASE9=COMPLETE_GREEN_PARKED
```

---

# Phase 10 work completed

## Phase 10A — Native Proof Verification Contract Boundary

Phase 10A added the proof verification contract boundary.

### Purpose

Review proof verification eligibility without creating live verification runtime authority.

### What it proved

```text
Phase 9B signed envelope is reused
signed payload metadata is reviewed
verification evidence contract is modeled
wrong domain/version/challenge/passport/device/proof/request/signature evidence is rejected
contract-only posture is preserved
```

### What it did not add

```text
LIVE_CRYPTO_VERIFIER_RUNTIME=NO
REPLAY_MUTATION=NO
CHALLENGE_CONSUMPTION=NO
CAPABILITY_ISSUANCE=NO
WALLET_OR_LEDGER_MUTATION=NO
```

---

## Phase 10B — Native Proof Verification Adapter

Phase 10B added an injected local proof verification adapter contract.

### Purpose

Model injected verification evidence and deterministic assessment without turning `svc-passport` into secret or runtime authority.

### What it proved

```text
adapter evidence is bound to signed payload
operation, authority, challenge, passport/device, proof/request/signature/evidence bindings are checked
verification acceptance/rejection is deterministic
unsafe verification adapter evidence is rejected
```

### What it did not add

```text
REPLAY_STORE_MUTATION=NO
CHALLENGE_CONSUMPTION=NO
CAPABILITY_ISSUANCE=NO
RUNTIME_STORAGE_MUTATION=NO
WALLET_OR_LEDGER_MUTATION=NO
```

---

## Phase 10C — Native Proof Verification Acceptance

Phase 10C closed the verification contract/adapter chain.

### What it proved

```text
proof verification boundary works
proof verification adapter works
signed proof evidence can be reviewed
replay/challenge/capability/runtime/wallet mutation remains absent
```

### Phase 10 status

```text
PHASE10_NATIVE_PROOF_VERIFICATION_ACCEPTANCE=GREEN
PHASE10=COMPLETE_GREEN_PARKED
```

---

# Phase 11 work completed

## Phase 11A — Replay and Challenge Consumption Contract

Phase 11A added a replay/challenge consumption contract.

### Files added/updated

```text
crates/svc-passport/src/native/replay_consumption.rs
crates/svc-passport/tests/native_passport_phase11a_replay_and_challenge_consumption_contract.rs
crates/svc-passport/src/native/mod.rs
```

### What it proved

```text
verified proof envelope can be reviewed for replay and challenge consumption
replay unseen state is required
challenge unconsumed state is required
idempotency binding is checked
operation/challenge/passport/device/proof/request/signature/evidence bindings are checked
contract-only posture is preserved
```

### What it did not add

```text
REPLAY_STORE_MUTATION=NO
CHALLENGE_CONSUMPTION_EXECUTION=NO
CAPABILITY_ISSUANCE=NO
RUNTIME_IO=NO
STORAGE_MUTATION=NO
```

---

## Phase 11B — Replay and Challenge Consumption Adapter

Phase 11B added an injected adapter contract for replay/challenge state application.

### Files added/updated

```text
crates/svc-passport/src/native/replay_consumption_adapter.rs
crates/svc-passport/tests/native_passport_phase11b_replay_and_challenge_consumption_adapter.rs
crates/svc-passport/src/native/mod.rs
```

### What it proved

```text
replay/challenge consumption decision can be applied through injected adapter evidence
state adapter labels and evidence are bound
replay/idempotency/challenge/passport/device/proof/request/signature bindings are checked
bad adapter evidence is rejected
```

### What it did not add

```text
BUILT_IN_REPLAY_STORE=NO
BUILT_IN_CHALLENGE_STORE=NO
CAPABILITY_ISSUANCE=NO
LIFECYCLE_RUNTIME=NO
SECRET_ACCESS=NO
```

---

## Phase 11C — Device-Bound Capability Contract

Phase 11C added the device-bound capability contract.

### Files added/updated

```text
crates/svc-passport/src/native/capability_contract.rs
crates/svc-passport/tests/native_passport_phase11c_device_bound_capability_contract.rs
crates/svc-passport/src/native/mod.rs
```

### What it proved

```text
capability contract domain/version is locked
capability is bound to device
capability scopes are reviewed
capability TTL/skew policy is enforced
request proof contract is reused
replay/challenge consumption envelope is reused
broad scopes and mismatches are rejected
contract-only posture is preserved
```

### What it did not add

```text
CAPABILITY_ISSUANCE_EXECUTION=NO
CAPABILITY_STORAGE=NO
CAPABILITY_CONSUMPTION=NO
RUNTIME_IO=NO
WALLET_OR_LEDGER_MUTATION=NO
```

---

## Phase 11D — Device-Bound Capability Adapter

Phase 11D added the injected adapter boundary for recording capability evidence.

### Files added/updated

```text
crates/svc-passport/src/native/capability_adapter.rs
crates/svc-passport/tests/native_passport_phase11d_device_bound_capability_adapter.rs
crates/svc-passport/src/native/mod.rs
```

### What it proved

```text
capability adapter evidence is bound to contract decision
device/capability/scope/replay/idempotency labels are checked
bad capability evidence is rejected
capability is device-bound
capability is not consumed during creation
contract-only/injected-adapter posture is preserved
```

### What it did not add

```text
CAPABILITY_LIFECYCLE_RUNTIME=NO
CAPABILITY_CONSUMPTION_RUNTIME=NO
RAW_CAPABILITY_TO_REACT=NO
SECRET_ACCESS=NO
KEY_LOADING=NO
VAULT_RUNTIME=NO
WALLET_OR_LEDGER_MUTATION=NO
```

---

## Phase 11E — Capability Acceptance

Phase 11E closed the device-bound capability/request-proof chain.

### Files added/updated

```text
crates/svc-passport/tests/native_passport_phase11e_capability_acceptance.rs
```

### What it proved

```text
device-bound capability contract works
device-bound capability adapter works
request-proof binding works
replay/idempotency/challenge bindings work
scope boundaries are enforced
bad capability evidence is rejected
```

### Phase 11 status

```text
PHASE11_DEVICE_BOUND_CAPABILITY=COMPLETE_GREEN_PARKED
```

---

# Phase 12 work completed

## Phase 12A — Private-Beta Username Registry Contract

Phase 12A added the private-beta username registry contract.

### Files added/updated

```text
crates/svc-passport/src/native/username_registry.rs
crates/svc-passport/tests/native_passport_phase12a_username_private_beta_registry_contract.rs
crates/svc-passport/src/native/mod.rs
```

### What it proved

```text
RonUsername / UsernameV1 canonical reuse is enforced
username transition contract domain/version is locked
claim/transfer/release transition kinds are modeled
one-primary-username policy is reviewed
reserved username rejection works
rate-limit policy rejection works
transfer/release require current owner proof
fresh proof transcript binding is checked
single-writer/private-beta posture is modeled
index projection is explicitly not authority
```

### What it did not add

```text
DURABLE_USERNAME_WRITER_EXECUTION=NO
INDEX_MUTATION=NO
NAMESPACE_ROUTES=NO
NATIVE_STORAGE_MUTATION=NO
USERNAME_AS_LEGAL_IDENTITY=NO
USERNAME_AS_RECOVERY_AUTHORITY=NO
USERNAME_AS_ROOT_AUTHORITY=NO
WALLET_OR_LEDGER_AUTHORITY=NO
SECRET_ACCESS=NO
```

---

## Phase 12B — Private-Beta Username Registry Adapter

Phase 12B added the injected adapter boundary for username registry transitions.

### Files added/updated

```text
crates/svc-passport/src/native/username_registry_adapter.rs
crates/svc-passport/tests/native_passport_phase12b_username_private_beta_registry_adapter.rs
crates/svc-passport/src/native/mod.rs
```

### What it proved

```text
claim/transfer/release adapter flow works
canonical handle binding works
transition hash binding works
idempotency key binding works
fresh proof transcript binding works
bad writer evidence rejection works
index projection remains non-authoritative
```

### What it did not add

```text
DURABLE_USERNAME_WRITER_INSIDE_NATIVE=NO
INDEX_PROJECTION_AUTHORITY=NO
NAMESPACE_ROUTES=NO
STORAGE_MUTATION_INSIDE_NATIVE=NO
SECRET_KEY_OR_VAULT_RUNTIME=NO
```

---

## Phase 12C — Username Private-Beta Index Projection

Phase 12C added the index projection contract.

### Files added/updated

```text
crates/svc-passport/src/native/username_index_projection.rs
crates/svc-passport/tests/native_passport_phase12c_username_private_beta_index_projection.rs
crates/svc-passport/src/native/mod.rs
```

### Initial failure

One test expected the wrong error type for unsafe projection authority flags.

The implementation distinguished:

```text
IndexProjectionTreatedAsAuthority
```

from broader runtime mutation/unsafe projection flags.

### Fix

The test was split into more specific cases:

```text
phase12c_rejects_index_authority_flags_with_specific_authority_error
phase12c_rejects_projection_runtime_mutation_flags_as_unsafe
```

### What Phase 12C proved

```text
exact lookup projection contract works
claim/transfer produce upsert projection
release produces delete projection
lookup key binding works
projection event binding works
transition hash and idempotency key bindings work
authoritative index projection is rejected
runtime mutation flags are rejected
unfinalized projection input is rejected
```

### What it did not add

```text
INDEX_WRITER_EXECUTION=NO
NAMESPACE_ROUTES=NO
STORAGE_MUTATION=NO
AUTHORITY_DRIFT=NO
SECRET_KEY_OR_VAULT_RUNTIME=NO
```

---

## Phase 12D — Username Private-Beta Acceptance

Phase 12D closed Phase 12.

### Files added/updated

```text
crates/svc-passport/tests/native_passport_phase12d_username_private_beta_acceptance.rs
```

### What it proved

```text
claim/transfer/release acceptance works
registry contract -> adapter -> projection flow works
Ron naming canonical reuse works
canonical handle binding works
lookup key binding works
projection event binding works
transition hash binding works
idempotency key binding works
fresh proof binding works
reserved username rejection works
stale proof rejection works
bad writer evidence rejection works
authoritative index rejection works
```

### Phase 12 status

```text
PHASE12_USERNAME_PRIVATE_BETA_REGISTRY=COMPLETE_GREEN_PARKED
```

---

# Phase 13 work completed

## Phase 13 — Local Status and Inspection

Phase 13 added the local status inspection surface.

### Files added/updated

```text
crates/svc-passport/src/native/local_status_inspection.rs
crates/svc-passport/tests/native_passport_phase13_local_status_and_inspection.rs
crates/svc-passport/src/native/mod.rs
```

### First attempt

The first Phase 13 patch aborted before writes because generated source validation found forbidden raw identifier field patterns.

### Final result

Phase 13 v2 completed green.

### What it proved

```text
local status inspection works
redacted local snapshot works
local-only review works
Phase 12 surface chain remains visible
passport identifier exposure is rejected
device identifier exposure is rejected
username handle exposure is rejected
proof material exposure is rejected
capability material exposure is rejected
vault material exposure is rejected
nonlocal status is rejected
```

### What it did not add

```text
ROUTES_ADDED=NO
RUNTIME_IO_ADDED=NO
STORAGE_MUTATION_INSIDE_NATIVE_ADDED=NO
INDEX_PROJECTION_AUTHORITY_ADDED=NO
WALLET_OR_LEDGER_MUTATION_ADDED=NO
SECRET_KEY_ACCESS_ADDED=NO
KEY_LOADING_ADDED=NO
KEY_DERIVATION_RUNTIME_ADDED=NO
VAULT_UNLOCK_ADDED=NO
VAULT_RUNTIME_ADDED=NO
PLATFORM_SEALER_UNSEAL_ADDED=NO
RUNTIME_AUTHORITY_CHANGED=NO
NATIVE_SECRET_IMPLEMENTATION_ADDED=NO
```

---

# Phase 14 work completed

## Phase 14A — Gateway/Omnigate Fixed Route Catalog Contract

Phase 14A added the fixed Gateway/Omnigate route catalog contract under native Passport surfaces.

### Files added/updated

```text
crates/svc-passport/src/native/gateway_omnigate_routes.rs
crates/svc-passport/tests/native_passport_phase14a_gateway_omnigate_route_contract.rs
crates/svc-passport/src/native/mod.rs
```

### Initial failure

The first Phase 14A run failed because `mutating_scope_allowed` was rejected as a generic route-shape mismatch instead of the dedicated mutating-scope escalation error.

### Fix

We moved the mutating-scope guard before route-shape review and removed `mutating_scope_allowed` from the generic route-shape mismatch branch.

### What Phase 14A proved

```text
gateway fixed route catalog is locked
Omnigate fixed route catalog is locked
downstream svc-passport route mapping is locked
body caps are reviewed
deadlines are reviewed
correlation IDs are reviewed
typed redacted problem envelope exists
raw body logging is disabled
mutating scope escalation is rejected
Gateway remains proxy only
Omnigate remains orchestration only
svc-passport remains identity authority
```

### What it did not add

```text
LIVE_GATEWAY_ROUTE_MOUNT_ADDED=NO
LIVE_OMNIGATE_ROUTE_MOUNT_ADDED=NO
STORAGE_MUTATION_INSIDE_GATEWAY_OR_OMNIGATE_ADDED=NO
WALLET_OR_LEDGER_MUTATION_ADDED=NO
SECRET_KEY_ACCESS_ADDED=NO
KEY_LOADING_ADDED=NO
KEY_DERIVATION_RUNTIME_ADDED=NO
VAULT_UNLOCK_ADDED=NO
VAULT_RUNTIME_ADDED=NO
PLATFORM_SEALER_UNSEAL_ADDED=NO
NATIVE_SECRET_IMPLEMENTATION_ADDED=NO
```

---

## Phase 14B — Omnigate Fixed Route Admission

Phase 14B added a contract-level Omnigate admission reviewer for the fixed Passport route set.

### Files added/updated

```text
crates/svc-passport/src/native/omnigate_fixed_route_admission.rs
crates/svc-passport/tests/native_passport_phase14b_omnigate_fixed_route_admission.rs
crates/svc-passport/src/native/mod.rs
```

### What Phase 14B proved

```text
Omnigate can only admit reviewed fixed Passport identity routes
fixed route catalog is reused
body caps are enforced
deadlines are enforced
correlation IDs are required
typed redacted problem envelope posture is required
fresh proof guards are preserved
device-bound capability guards are preserved
caller-controlled downstream URLs are rejected
mutating scope escalation is rejected
raw request/response body logging is disabled
identity authority does not move into Omnigate
```

### What it did not add

```text
LIVE_OMNIGATE_ROUTE_MOUNT=NO
STORAGE_MUTATION=NO
WALLET_OR_LEDGER_MUTATION=NO
SECRET_ACCESS=NO
```

---

## Phase 14C — Gateway Fixed Route Admission

Phase 14C added the Gateway-side admission contract that delegates through Omnigate admission.

### Files added/updated

```text
crates/svc-passport/src/native/gateway_fixed_route_admission.rs
crates/svc-passport/tests/native_passport_phase14c_gateway_fixed_route_admission.rs
crates/svc-passport/src/native/mod.rs
```

### What Phase 14C proved

```text
gateway fixed-route admission works
fixed gateway route catalog is reused
Omnigate fixed-route admission is reused
downstream svc-passport route mapping is reused
body cap gateway review works
deadline gateway review works
correlation ID gateway review works
device-bound capability requirement review works
fresh proof requirement review works
mutating scope escalation rejection works
caller-controlled downstream URL rejection works
typed redacted problem availability is required
request/response body logging disabled review works
Gateway remains proxy only
Omnigate remains orchestration only
svc-passport remains identity authority
```

### What it did not add

```text
LIVE_GATEWAY_ROUTE_MOUNT_ADDED=NO
LIVE_OMNIGATE_ROUTE_MOUNT_ADDED=NO
STORAGE_MUTATION_INSIDE_GATEWAY_OR_OMNIGATE_ADDED=NO
WALLET_OR_LEDGER_MUTATION_ADDED=NO
SECRET_KEY_ACCESS_ADDED=NO
NATIVE_SECRET_IMPLEMENTATION_ADDED=NO
```

---

## Phase 14D — Gateway/Omnigate Route Mount Acceptance

Phase 14D closed Phase 14 and ran a workspace check.

### Files added/updated

```text
crates/svc-passport/src/native/gateway_omnigate_route_mount_acceptance.rs
crates/svc-passport/tests/native_passport_phase14d_gateway_omnigate_route_mount_acceptance.rs
crates/svc-passport/src/native/mod.rs
```

### Initial issue

One Phase 14D test expected a drifted public route path to be classified as:

```text
UnknownFixedRouteDecision
```

But the implementation correctly found the fixed route kind and rejected the changed path as:

```text
RouteDecisionShapeMismatch
```

We fixed the test expectation, not the implementation.

### Final result

```text
NATIVE_PASSPORT_PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE=GREEN
PHASE14_GATEWAY_AND_OMNIGATE_INTEGRATION=COMPLETE_GREEN_PARKED
WORKSPACE_CHECK_AFTER_PHASE14D=GREEN
```

### What Phase 14D proved

```text
route mount acceptance contract works
fixed route catalog is reused
gateway admission chain is reused
Omnigate admission chain is reused
all fixed routes are covered
body cap acceptance works
deadline acceptance works
correlation ID acceptance works
typed redacted problem acceptance works
duplicate/missing route rejection works
dynamic route mount rejection works
caller-controlled downstream URL rejection works
request/response body logging rejection works
Gateway remains proxy only
Omnigate remains orchestration only
svc-passport remains identity authority
```

### What it did not add

```text
LIVE_GATEWAY_ROUTE_MOUNT_IMPLEMENTED=NO
LIVE_OMNIGATE_ROUTE_MOUNT_IMPLEMENTED=NO
STORAGE_MUTATION_INSIDE_GATEWAY_OR_OMNIGATE_ADDED=NO
WALLET_OR_LEDGER_MUTATION_ADDED=NO
SECRET_KEY_ACCESS_ADDED=NO
NATIVE_SECRET_IMPLEMENTATION_ADDED=NO
```

---

# Phase 15 work completed

Phase 15 is the CrabLink desktop integration phase.

Buildplan target for full Phase 15 runtime:

```text
desktop creates/backs up Passport
restart returns locked state
operational PIN unlock works without root unlock
root operations require separate native confirmation
proof/capability/username claim work
React never receives secrets
```

This session completed the **desktop command surface contract chain**, not full runtime desktop integration.

---

## Phase 15A — Desktop `passport_status` Command Contract

Phase 15A added the initial `passport_status` command contract.

### First failure

The first Phase 15A mutation aborted before writing because generated source validation found forbidden substrings like:

```text
seed_phrase:
```

The issue was neutral DTO field naming that still triggered strict source guards.

### Fix

We renamed fields to safer neutral forms:

```text
exposes_recovery_words
exposes_root_signing_material
exposes_device_signing_material
exposes_capability_material
exposes_vault_material
```

### Files added/updated

```text
crates/svc-passport/src/native/client_status_command.rs
crates/svc-passport/tests/native_passport_phase15a_client_status_command.rs
crates/svc-passport/src/native/mod.rs
```

### What Phase 15A proved

```text
passport_status command contract works
expected desktop command inventory is listed
forbidden desktop command inventory is listed
redacted status DTO to React is modeled
route acceptance is reused
local status inspection is reused
NoPassport / Locked / OperationalUnlocked / RootUnlocked shapes are reviewed
secret exposure is rejected
capability material exposure is rejected
vault material exposure is rejected
unlock runtime is rejected
arbitrary scope or policy disable is rejected
```

### What it did not add

```text
LIVE_TAURI_COMMAND_ADDED=NO
CREATE_RESTORE_UNLOCK_IMPLEMENTATION_ADDED=NO
PROOF_CAPABILITY_USERNAME_COMMAND_IMPLEMENTATION_ADDED=NO
SEED_TO_WEBVIEW_ADDED=NO
PRIVATE_KEY_EXPORT_ADDED=NO
DEVICE_KEY_EXPORT_ADDED=NO
RAW_CAPABILITY_EXPORT_ADDED=NO
PLATFORM_SEALER_UNSEAL_ADDED=NO
RUNTIME_IO_ADDED=NO
STORAGE_MUTATION_ADDED=NO
WALLET_OR_LEDGER_MUTATION_ADDED=NO
NATIVE_SECRET_IMPLEMENTATION_ADDED=NO
```

---

## Phase 15B — Desktop Command Inventory and Denylist

Phase 15B locked the desktop command inventory and denylist before any live Tauri wiring.

### Files added/updated

```text
crates/svc-passport/src/native/client_command_inventory.rs
crates/svc-passport/tests/native_passport_phase15b_desktop_command_inventory_and_denylist.rs
crates/svc-passport/src/native/mod.rs
```

### Expected command allowlist

```text
passport_status
passport_create_native
passport_restore_native
passport_unlock_operational
passport_unlock_root
passport_lock
passport_clear
passport_device_list
passport_device_authorize
passport_device_revoke
passport_prove
passport_capability_status
passport_capability_refresh
passport_capability_revoke
passport_username_status
passport_username_claim
passport_username_transfer
passport_username_release
```

### Forbidden commands

```text
passport_get_seed_to_webview
passport_export_private_key
passport_get_device_private_key
passport_get_raw_capability
passport_issue_arbitrary_scope
passport_disable_policy
```

### What Phase 15B proved

```text
desktop command inventory contract works
expected desktop commands are locked
forbidden desktop commands are locked
forbidden commands are absent from allowlist
redacted React DTO policy is required
status command contract is reused
missing/duplicate/unknown command rejection works
malformed command shape rejection works
live Tauri wiring rejection works
secret-to-React rejection works
arbitrary scope or policy disable rejection works
```

### Warning cleanup

Phase 15B initially emitted a harmless unused-import warning. Phase 15C cleaned it up.

---

## Phase 15C — Desktop Redacted Command DTO Adapter

Phase 15C added the adapter from reviewed status/inventory decisions into a React-facing redacted command DTO envelope.

### Initial failure

The Phase 15C mutation partially wrote files and then `cargo fmt` failed because the test contained invalid Rust slice type syntax for `exposure_mutators`.

### Fix

We fixed the test in place by changing the broken multi-line type into a valid slice type:

```rust
let exposure_mutators: &[(
    fn(&mut NativeClientRedactedCommandDtoAdapterDraftV1),
    NativeClientRedactedCommandDtoAdapterReviewError,
)] = &[
    ...
];
```

### Files added/updated

```text
crates/svc-passport/src/native/client_redacted_command_dto_adapter.rs
crates/svc-passport/tests/native_passport_phase15c_desktop_redacted_command_dto_adapter.rs
crates/svc-passport/tests/native_passport_phase15b_desktop_command_inventory_and_denylist.rs
crates/svc-passport/src/native/mod.rs
```

### What Phase 15C proved

```text
redacted command DTO adapter works
status command decision is reused
command inventory decision is reused
React DTO target is locked
forbidden command rejection works
unallowlisted command rejection works
unreviewed decision rejection works
unredacted identifier rejection works
secret material rejection works
capability material rejection works
vault material rejection works
unsafe status decision rejection works
unsafe inventory decision rejection works
Phase 15B warning cleanup is green
```

### What it did not add

```text
LIVE_TAURI_COMMAND_ADDED=NO
UNLOCK_RUNTIME_ADDED=NO
PLATFORM_SEALER_UNSEAL_ADDED=NO
RUNTIME_IO_ADDED=NO
STORAGE_MUTATION_ADDED=NO
WALLET_OR_LEDGER_MUTATION_ADDED=NO
NATIVE_SECRET_IMPLEMENTATION_ADDED=NO
```

---

## Phase 15D — Desktop Command Surface Acceptance

Phase 15D closed the desktop command surface contract chain and ran the workspace check.

### Files added/updated

```text
crates/svc-passport/src/native/client_command_surface_acceptance.rs
crates/svc-passport/tests/native_passport_phase15d_desktop_command_surface_acceptance.rs
crates/svc-passport/src/native/mod.rs
```

### Final verification

```text
SVC_PASSPORT_FORMAT=GREEN
PHASE15D_NATIVE_FEATURE_TEST=GREEN
PHASE15D_DEFAULT_FEATURE_GATE_TEST=GREEN
PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_REGRESSION=GREEN
PHASE15B_COMMAND_INVENTORY_REGRESSION=GREEN
PHASE15A_CLIENT_STATUS_COMMAND_REGRESSION=GREEN
PHASE14D_ROUTE_MOUNT_ACCEPTANCE_REGRESSION=GREEN
SVC_PASSPORT_DEFAULT_CHECK=GREEN
SVC_PASSPORT_NATIVE_FEATURE_CHECK=GREEN
WORKSPACE_CHECK_AFTER_PHASE15D=GREEN
```

### What Phase 15D proved

```text
desktop command surface contract chain is complete and parked
status command contract is reused
command inventory contract is reused
redacted DTO adapter is reused
all desktop commands are allowlisted
forbidden desktop commands are denied
React receives only redacted DTOs
command group acceptance works
unreviewed command rejection works
unredacted React DTO rejection works
unsafe status decision rejection works
unsafe inventory decision rejection works
unsafe redacted DTO rejection works
secret/capability/vault exposure rejection works
```

### What Phase 15D explicitly did not add

```text
LIVE_DESKTOP_COMMAND_WIRING_ADDED=NO
LIFECYCLE_RUNTIME_ADDED=NO
UNLOCK_RUNTIME_ADDED=NO
PROOF_CAPABILITY_USERNAME_RUNTIME_ADDED=NO
PLATFORM_SEALER_UNSEAL_ADDED=NO
RUNTIME_IO_ADDED=NO
STORAGE_MUTATION_ADDED=NO
WALLET_OR_LEDGER_MUTATION_ADDED=NO
NATIVE_SECRET_IMPLEMENTATION_ADDED=NO
PHASE15_DESKTOP_INTEGRATION_RUNTIME_COMPLETE=NO
```

---

# Final files changed during this session

All new source/test work was under:

```text
crates/svc-passport
```

## Phase 7 files

```text
crates/svc-passport/src/native/restore.rs
crates/svc-passport/tests/native_passport_phase7a_restore_contract_dto.rs

crates/svc-passport/src/native/enrollment.rs
crates/svc-passport/tests/native_passport_phase7b_new_device_enrollment_contract_dto.rs

crates/svc-passport/src/native/delegated_enrollment.rs
crates/svc-passport/tests/native_passport_phase7c_delegated_enrollment_contract_dto.rs

crates/svc-passport/tests/native_passport_phase7d_restore_new_device_delegated_enrollment_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

Note: exact module filenames should be verified in the repo before generating any automated changelog. The likely intent/ownership is accurate, but the next session can confirm exact final names with `ls crates/svc-passport/src/native`.

## Phase 8 files

```text
crates/svc-passport/src/native/challenge.rs
crates/svc-passport/src/native/proof.rs
crates/svc-passport/src/native/request_proof.rs

crates/svc-passport/tests/native_passport_phase8a_proof_challenge_contract_dto.rs
crates/svc-passport/tests/native_passport_phase8b_proof_contract_dto.rs
crates/svc-passport/tests/native_passport_phase8c_proof_request_contract_dto.rs
crates/svc-passport/tests/native_passport_phase8d_challenge_proof_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

## Phase 9 files

```text
crates/svc-passport/src/native/proof_signing.rs
crates/svc-passport/src/native/proof_signing_adapter.rs

crates/svc-passport/tests/native_passport_phase9a_native_proof_signing_contract_boundary.rs
crates/svc-passport/tests/native_passport_phase9b_native_proof_signing_adapter.rs
crates/svc-passport/tests/native_passport_phase9c_native_proof_signing_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

## Phase 10 files

```text
crates/svc-passport/src/native/proof_verification.rs
crates/svc-passport/src/native/proof_verification_adapter.rs

crates/svc-passport/tests/native_passport_phase10a_native_proof_verification_contract_boundary.rs
crates/svc-passport/tests/native_passport_phase10b_native_proof_verification_adapter.rs
crates/svc-passport/tests/native_passport_phase10c_native_proof_verification_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

## Phase 11 files

```text
crates/svc-passport/src/native/replay_consumption.rs
crates/svc-passport/src/native/replay_consumption_adapter.rs
crates/svc-passport/src/native/capability_contract.rs
crates/svc-passport/src/native/capability_adapter.rs

crates/svc-passport/tests/native_passport_phase11a_replay_and_challenge_consumption_contract.rs
crates/svc-passport/tests/native_passport_phase11b_replay_and_challenge_consumption_adapter.rs
crates/svc-passport/tests/native_passport_phase11c_device_bound_capability_contract.rs
crates/svc-passport/tests/native_passport_phase11d_device_bound_capability_adapter.rs
crates/svc-passport/tests/native_passport_phase11e_capability_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

## Phase 12 files

```text
crates/svc-passport/src/native/username_registry.rs
crates/svc-passport/src/native/username_registry_adapter.rs
crates/svc-passport/src/native/username_index_projection.rs

crates/svc-passport/tests/native_passport_phase12a_username_private_beta_registry_contract.rs
crates/svc-passport/tests/native_passport_phase12b_username_private_beta_registry_adapter.rs
crates/svc-passport/tests/native_passport_phase12c_username_private_beta_index_projection.rs
crates/svc-passport/tests/native_passport_phase12d_username_private_beta_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

## Phase 13 files

```text
crates/svc-passport/src/native/local_status_inspection.rs
crates/svc-passport/tests/native_passport_phase13_local_status_and_inspection.rs
crates/svc-passport/src/native/mod.rs
```

## Phase 14 files

```text
crates/svc-passport/src/native/gateway_omnigate_routes.rs
crates/svc-passport/src/native/omnigate_fixed_route_admission.rs
crates/svc-passport/src/native/gateway_fixed_route_admission.rs
crates/svc-passport/src/native/gateway_omnigate_route_mount_acceptance.rs

crates/svc-passport/tests/native_passport_phase14a_gateway_omnigate_route_contract.rs
crates/svc-passport/tests/native_passport_phase14b_omnigate_fixed_route_admission.rs
crates/svc-passport/tests/native_passport_phase14c_gateway_fixed_route_admission.rs
crates/svc-passport/tests/native_passport_phase14d_gateway_omnigate_route_mount_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

## Phase 15 files

```text
crates/svc-passport/src/native/client_status_command.rs
crates/svc-passport/src/native/client_command_inventory.rs
crates/svc-passport/src/native/client_redacted_command_dto_adapter.rs
crates/svc-passport/src/native/client_command_surface_acceptance.rs

crates/svc-passport/tests/native_passport_phase15a_client_status_command.rs
crates/svc-passport/tests/native_passport_phase15b_desktop_command_inventory_and_denylist.rs
crates/svc-passport/tests/native_passport_phase15c_desktop_redacted_command_dto_adapter.rs
crates/svc-passport/tests/native_passport_phase15d_desktop_command_surface_acceptance.rs

crates/svc-passport/src/native/mod.rs
```

---

# Important failure/fix history from this session

## Broad token scan lesson

Several failures came from broad source scans matching safe DTO names or comments.

Examples:

```text
seed_phrase:
mnemonic:
root_private_key:
device_private_key:
```

Lesson:

```text
Use runtime-shaped source guards when possible.
Avoid harmless DTO fields that produce forbidden source substrings.
Do not scan entire files for generic type names when checking unused imports.
Keep secret-like flags neutral: recovery_words, signing_material, capability_material, vault_material.
```

## Phase 14A error-order fix

Problem:

```text
mutating_scope_allowed was rejected as RouteShapeMismatch
```

Fix:

```text
move mutating scope guard before validate_route_shape
remove mutating_scope_allowed from generic route shape check
```

## Phase 14D test expectation fix

Problem:

```text
test expected UnknownFixedRouteDecision
implementation returned RouteDecisionShapeMismatch
```

Correct behavior:

```text
route kind was known, path shape drifted
therefore RouteDecisionShapeMismatch was correct
```

Fix:

```text
update test expectation
```

## Phase 15A forbidden substring abort

Problem:

```text
generated source validation caught seed_phrase:
repo stayed unchanged
```

Fix:

```text
rename DTO flags to neutral names
exposes_recovery_words
exposes_root_signing_material
exposes_device_signing_material
exposes_capability_material
exposes_vault_material
```

## Phase 15C partial mutation syntax fix

Problem:

```text
cargo fmt failed after files were written
invalid Rust slice type syntax in exposure_mutators
repo was partially mutated
```

Fix:

```rust
let exposure_mutators: &[(
    fn(&mut NativeClientRedactedCommandDtoAdapterDraftV1),
    NativeClientRedactedCommandDtoAdapterReviewError,
)] = &[
    ...
];
```

Result:

```text
Phase 15C green
Phase 15B warning cleanup green
```

---

# Current verification baseline

Latest confirmed final commands:

```text
cargo fmt -p svc-passport
cargo test -p svc-passport --features native-passport --test native_passport_phase15d_desktop_command_surface_acceptance
cargo test -p svc-passport --test native_passport_phase15d_desktop_command_surface_acceptance
cargo test -p svc-passport --features native-passport --test native_passport_phase15c_desktop_redacted_command_dto_adapter
cargo test -p svc-passport --features native-passport --test native_passport_phase15b_desktop_command_inventory_and_denylist
cargo test -p svc-passport --features native-passport --test native_passport_phase15a_client_status_command
cargo test -p svc-passport --features native-passport --test native_passport_phase14d_gateway_omnigate_route_mount_acceptance
cargo check -p svc-passport
cargo check -p svc-passport --features native-passport
cargo check --workspace
```

All passed at the end of this session.

---

# What must not be claimed yet

Do **not** claim:

```text
svc-passport can restore Passports
svc-passport can enroll devices at runtime
svc-passport can unlock native vaults
svc-passport can derive PIN keys
svc-passport can store Passport secrets
svc-passport can sign native Passport proofs using real keys
svc-passport can verify proofs with full runtime authority
svc-passport can issue live device capabilities
desktop Passport integration is runtime complete
React has integrated real Passport status command output
Tauri command wiring exists
```

Correct claim:

```text
svc-passport has feature-gated native Passport contract/DTO/review foundations through Phase 15D.
The desktop command surface contract chain is complete and workspace-green.
Full desktop runtime integration is still pending.
```

---

# Next session starting point

Start with the latest safe marker:

```text
NEXT_PATCH=NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION
```

Recommended first commands:

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo check --workspace
```

Then confirm the focused green baseline:

```bash
cargo test -p svc-passport \
  --features native-passport \
  --test native_passport_phase15d_desktop_command_surface_acceptance

cargo test -p svc-passport \
  --features native-passport \
  --test native_passport_phase15c_desktop_redacted_command_dto_adapter

cargo check -p svc-passport
cargo check -p svc-passport --features native-passport
```

---

# Next patch: Phase 15E

## Marker

```text
NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION
```

## Purpose

Inspect and contractually prepare for real desktop `passport_status` command wiring without yet adding unsafe runtime behavior.

This should bridge:

```text
Phase 15A passport_status command contract
Phase 15B command inventory and denylist
Phase 15C redacted command DTO adapter
Phase 15D desktop command surface acceptance
future CrabLink/Tauri command wiring
```

## Recommended implementation shape

Keep it small and bounded:

```text
new source:
  crates/svc-passport/src/native/client_status_command_wiring_inspection.rs

new test:
  crates/svc-passport/tests/native_passport_phase15e_desktop_status_command_wiring_inspection.rs

update:
  crates/svc-passport/src/native/mod.rs
```

## What Phase 15E should inspect/model

```text
passport_status is the first command to wire
passport_status must return the Phase 15C/15D redacted DTO shape
desktop command surface rejects raw/native secret return types
existing dev label compatibility remains explicitly dev-only
forbidden commands are not considered valid wiring targets
actual Tauri/CrabLink command crate anchors are not assumed unless inspected
```

## What Phase 15E should not do yet

```text
do not add #[tauri::command] unless exact app anchors are inspected and intentionally targeted
do not call platform sealer
do not unlock vaults
do not read/write runtime storage
do not create/restore real Passport
do not issue proof/capability
do not connect to Gateway/Omnigate
do not expose raw identifiers/secrets/capabilities
do not mutate wallet/ledger state
```

---

# Likely follow-up phases after 15E

## Phase 15F — Real `passport_status` Tauri wiring

Once command wiring location is confirmed, add the first live command:

```text
passport_status
```

It should return only a redacted status DTO and support safe states:

```text
NoPassport
Locked
OperationalUnlocked
RootUnlocked
```

`passport_status` must not unlock anything.

## Phase 15G — Desktop command registry hook

Wire the allowed command inventory into the Tauri command registry.

Expected behavior:

```text
allowed names exist
forbidden names absent
unimplemented commands return typed redacted not-ready problems
React receives stable DTO/problem shape
```

## Phase 15H — PlatformSealer/VaultStore adapter inspection

Before real vault work, inspect actual platform storage/sealing options and decide whether implementation lives in:

```text
crates/svc-passport
packages/crablink-platform
apps/crablink-tauri/src-tauri
```

## Phase 15I onward — Real runtime slices

Likely runtime order:

```text
passport_status wiring
passport_lock / passport_clear wiring
operational unlock contract bridge
root unlock confirmation bridge
create native command bridge
restore native command bridge
device list / authorize / revoke command bridge
proof / request-proof bridge
capability status / refresh / revoke bridge
username status / claim / transfer / release bridge
React DTO integration
desktop acceptance
workspace acceptance
```

---

# Phase completion picture

## Completed / parked

```text
Phase 7A  — Restore Contract DTO
Phase 7B  — New Device Enrollment Contract DTO
Phase 7C  — Delegated Enrollment Contract DTO
Phase 7D  — Restore/New Device/Delegated Enrollment Acceptance
Phase 8A  — Purpose-Bound Challenge Contract DTO
Phase 8B  — Proof Contract DTO
Phase 8C  — Proof Request Contract DTO
Phase 8D  — Challenge/Proof Acceptance
Phase 9A  — Native Proof Signing Contract Boundary
Phase 9B  — Native Proof Signing Adapter
Phase 9C  — Native Proof Signing Acceptance
Phase 10A — Native Proof Verification Contract Boundary
Phase 10B — Native Proof Verification Adapter
Phase 10C — Native Proof Verification Acceptance
Phase 11A — Replay and Challenge Consumption Contract
Phase 11B — Replay and Challenge Consumption Adapter
Phase 11C — Device-Bound Capability Contract
Phase 11D — Device-Bound Capability Adapter
Phase 11E — Capability Acceptance
Phase 12A — Private-Beta Username Registry Contract
Phase 12B — Private-Beta Username Registry Adapter
Phase 12C — Username Index Projection
Phase 12D — Username Private-Beta Acceptance
Phase 13  — Local Status and Inspection
Phase 14A — Gateway/Omnigate Route Contract
Phase 14B — Omnigate Fixed Route Admission
Phase 14C — Gateway Fixed Route Admission
Phase 14D — Gateway/Omnigate Route Mount Acceptance
Phase 15A — Client Status Command Contract
Phase 15B — Desktop Command Inventory and Denylist
Phase 15C — Desktop Redacted Command DTO Adapter
Phase 15D — Desktop Command Surface Acceptance
```

## In progress / next

```text
Phase 15E — Desktop Status Command Wiring Inspection
```

## Not complete yet

```text
Phase 15 full runtime desktop integration
Phase 16 TV delegated integration
Phase 17 Android/iOS integration
Phase 18 public namespace finality / hardening / final acceptance
```

---

# Recommended next-session opening note

Use this exact note to resume:

```text
We ended with Phase 15D green and workspace green. This session covered Phase 7A through Phase 15D of the Passport Native Buildplan. The next marker is NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION. Do not claim Phase 15 runtime complete yet; only the desktop command surface contract chain is complete. Start with cargo check --workspace, then build the Phase 15E bounded inspection/contract slice for desktop passport_status wiring. Keep it in svc-passport unless the actual CrabLink Tauri app anchors are present and inspected. No live Tauri command, no vault unlock, no runtime I/O, no storage mutation, no wallet/ledger mutation, and no secrets to React.
```



### END NOTE - JULY 23 2026 - 17:40 CST

### BEGIN NOTE - JULY 24 2026 - 16:30 CST

These notes begin at the exact next marker, Phase 15T, and preserve the session’s implementation details, failures, security decisions, files, and remaining roadmap.

# Native Passport Phase 15 — Comprehensive Carry-Over Notes

## 0. Exact next-session starting point

Resume from:

```text
NEXT_PATCH=NATIVE_PASSPORT_PHASE15T_OPERATIONAL_UNLOCK_COMMAND_LOCK_AND_STATUS_BRIDGE
```

Current final green marker:

```text
NATIVE_PASSPORT_PHASE15S_OPERATIONAL_UNSEAL_AND_LOCKED_VAULT_UNLOCK_RUNTIME=GREEN
```

Do not repeat Phase 15E through Phase 15S unless a new compiler failure proves a regression.

The next session should continue active implementation of the **Passport Native Buildplan**, specifically **Phase 15 — CrabLink Desktop Root-Capable Integration**.

The Phase 15 buildplan goal remains:

```text
Deliver complete desktop create/restore/unlock/device/username/proof behavior
without WebView secret custody.
```

The formal Phase 15 exit gate remains:

```text
Desktop creates/backs up Passport.
Restart returns locked state.
Operational PIN unlock works without root unlock.
Root operations require separate native confirmation.
Proof/capability/username claim work.
React never receives secrets.
```

The buildplan also requires the desktop client to provide lifecycle, unlock, device, proof, capability, and username commands while keeping all secret material outside React and JavaScript. 

---

# 1. Session scope

This session began at:

```text
NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION
```

The previous session had completed and parked:

```text
Phase 15A — passport_status command contract
Phase 15B — desktop command inventory and denylist
Phase 15C — redacted desktop command DTO adapter
Phase 15D — desktop command-surface acceptance
```

This session then completed the implementation chain from **Phase 15E through Phase 15S**.

Final session range:

```text
Phase 15E — desktop status-command wiring inspection
Phase 15F — real passport_status Tauri wiring and no-dev-KMS consumer repair
Phase 15H — desktop PlatformSealer/VaultStore implementation inspection
Phase 15I — shared platform-storage trait foundation
Phase 15J — desktop atomic VaultStore
Phase 15K — Windows atomic replacement and app-data VaultStore wiring
Phase 15L — macOS Keychain PlatformSealer
Phase 15M — Windows DPAPI PlatformSealer
Phase 15N — Linux Secret Service PlatformSealer
Phase 15O — runtime platform selection and AppState ownership
Phase 15P — truthful stored-vault status wiring
Phase 15Q — Native Passport vault cryptography and operational unlock primitive
Phase 15R — platform-bound vault creation orchestration
Phase 15S — stored-vault operational unseal and native session custody
```

There is no separate durable Phase 15G source/test marker in the current implementation. Do not invent Phase 15G work merely to fill the alphabetic gap.

---

# 2. Current completion judgment

Honest estimated Phase 15 completion after Phase 15S:

```text
Overall Phase 15 completion: approximately 62–65%
Security/storage/crypto foundation: approximately 85%
User-operable desktop Passport lifecycle: approximately 35–40%
```

The project now has most of the difficult vault, platform-storage, cryptographic, and native-session foundation.

It does **not** yet have the full user-facing desktop Passport experience required by the Phase 15 exit gate.

The main unfinished categories are:

```text
native secure PIN/recovery/root-confirmation input
public operational unlock command
public lock command
status projection of OperationalUnlocked
full Passport identity creation
recovery phrase backup and acknowledgement
restore/import
explicit root unlock
clear/reset
device list/authorize/revoke
proof/request-proof command transport
capability storage/status/refresh/revoke
username status/claim/transfer/release
real Passport label replacement
React redacted adapters and UI
restart and end-to-end desktop acceptance
```

---

# 3. Repository locations

RustyOnions:

```text
/Users/mymac/Desktop/RustyOnions
```

Canonical Passport crate:

```text
/Users/mymac/Desktop/RustyOnions/crates/svc-passport
```

CrabLink:

```text
/Users/mymac/Desktop/crablink
```

CrabLink Tauri Rust:

```text
/Users/mymac/Desktop/crablink/apps/crablink-tauri/src-tauri
```

The canonical ownership split is now:

```text
svc-passport
  owns shared Native Passport types
  owns vault crypto rules and codecs
  owns PlatformSealer/VaultStore contracts
  owns platform-bound vault envelope validation
  must remain the canonical Passport logic owner

crablink-tauri
  owns actual desktop OS adapters
  owns desktop filesystem persistence
  owns runtime platform selection
  owns AppState runtime objects
  owns native-only unlocked session custody
  owns future Tauri command bridges

React / TypeScript
  receives redacted DTOs only
  must never own PINs, phrases, root keys, device keys, VMKs, platform factors,
  or raw long-lived capabilities
```

This ownership division matches the buildplan’s requirement that `svc-passport` remain canonical while CrabLink acts as a native consumer rather than creating a second identity authority. 

---

# 4. Phase-by-phase work completed

## Phase 15E — Desktop status-command wiring inspection

### Purpose

Phase 15E inspected the intended first live desktop command before adding Tauri runtime authority.

The first command selected was:

```text
passport_status
```

### Added to `svc-passport`

```text
crates/svc-passport/src/native/client_status_command_wiring_inspection.rs
crates/svc-passport/tests/native_passport_phase15e_desktop_status_command_wiring_inspection.rs
```

Updated:

```text
crates/svc-passport/src/native/mod.rs
```

### What Phase 15E established

```text
passport_status is the first safe command to wire
the command must use the existing Phase 15C redacted DTO adapter
the command may project safe lock and capability labels
the command must not return raw Passport material
dev-label compatibility remains development-only
forbidden command names are not valid wiring targets
Tauri wiring must be inspected rather than assumed
```

### What remained absent

```text
no PlatformSealer operation
no VaultStore I/O
no vault unlock
no create/restore
no proof or capability issuance
no raw identity or capability return
no wallet or ledger mutation
```

The current `svc-passport` bundle contains the Phase 15E inspection module and test as part of the native feature-gated surface. 

---

## Phase 15F — Real `passport_status` Tauri wiring and no-dev-KMS consumer repair

Phase 15F had two closely related responsibilities.

### 15F-A — Real redacted Tauri status command

CrabLink Tauri received the first live Native Passport command implementation:

```text
passport_status
```

Primary Tauri files involved:

```text
apps/crablink-tauri/src-tauri/src/commands/passport.rs
apps/crablink-tauri/src-tauri/src/commands/mod.rs
apps/crablink-tauri/src-tauri/src/lib.rs
apps/crablink-tauri/src-tauri/Cargo.toml
```

The command added bounded serializable output/problem DTOs and reused the `svc-passport` status contract and redacted projection behavior.

Focused unit tests proved:

```text
truthful NoPassport output before runtime storage existed
safe projection of supported lock states
redaction of all potentially sensitive material
serialization of only the bounded DTO
```

The four command unit tests passed.

### 15F-B — Native consumer must compile without server DevKms

CrabLink depends on `svc-passport` as a native client with:

```toml
default-features = false
features = ["native-passport"]
```

It must not silently compile or gain the server development KMS.

Phase 15F repaired the KMS client layout so the trait remains available while the in-process `DevKms` implementation remains feature-gated.

Confirmed files:

```text
crates/svc-passport/src/kms/client.rs
crates/svc-passport/tests/native_passport_phase15f_no_dev_kms_consumer_boundary.rs
apps/crablink-tauri/src-tauri/Cargo.toml
```

The repair proved:

```text
KmsClient trait remains available without dev-kms
DevKms implementation remains behind #[cfg(feature = "dev-kms")]
CrabLink does not enable dev-kms
external KMS injection posture remains available
native Passport use does not gain service signing authority
```

The focused no-dev-KMS test passed 2/2. The exact repair and green test output are preserved in the session terminal history. 

### Phase 15F failure and fix

The first strict Clippy run failed because the new status command and DTO helpers were not yet reachable from the registered Tauri invoke surface, so Clippy treated them as dead code.

The final repair connected the command through the real Tauri command registry rather than suppressing the warnings broadly.

Do not reintroduce a crate-wide dead-code allowance.

---

## Phase 15H — Desktop PlatformSealer/VaultStore inspection

### Purpose

Phase 15H inspected the real desktop implementation requirements before adding OS or filesystem behavior.

Added to `svc-passport`:

```text
crates/svc-passport/src/native/desktop_platform_storage_inspection.rs
crates/svc-passport/tests/native_passport_phase15h_platform_sealer_vault_store_inspection.rs
```

Updated:

```text
crates/svc-passport/src/native/mod.rs
```

### Decisions locked

```text
svc-passport owns the portable contracts
CrabLink Tauri owns desktop OS and filesystem adapters
macOS uses Keychain
Windows uses DPAPI
Linux uses Secret Service
VaultStore writes must be atomic
plaintext fallback is forbidden
platform APIs must expose seal/unseal operations, not reusable device secrets
React must never receive sealed or unsealed secret material
```

No OS adapter or filesystem mutation was added inside `svc-passport`.

---

## Phase 15I — Shared PlatformSealer and VaultStore trait foundation

### Added to `svc-passport`

```text
crates/svc-passport/src/native/platform_storage.rs
crates/svc-passport/tests/native_passport_phase15i_platform_sealer_vault_store_trait_foundation.rs
```

Updated:

```text
crates/svc-passport/src/native/mod.rs
```

### Core types and traits added

```text
NativeSecretBytes
NativeSealedMaterialV1
NativeEncryptedVaultV1
NativePlatformSealer
NativeVaultStore
NativePlatformFamily
NativeSecureCompartment
NativePlatformStorageError
NativePlatformStorageOperation
NativeVaultRecoveryOutcome
NativeVaultRemovalOutcome
```

### Important behavior

`NativeSecretBytes`:

```text
bounded
zeroizing on drop
redacted Debug
no general serialization
```

`NativeSealedMaterialV1`:

```text
records platform family
records compartment
contains only bounded sealed bytes or an opaque platform reference
validates platform and compartment matching
```

`NativeEncryptedVaultV1`:

```text
bounded encrypted envelope only
rejects empty and oversized material
does not claim plaintext identity semantics
```

`NativeVaultStore`:

```text
load encrypted vault
atomic encrypted-vault write
recover interrupted write
remove encrypted vault
```

The atomic contract was modeled as a six-step process. The test suite proved bounded validation, redaction, mismatch rejection, test-sealer round trips, VaultStore load/write/recover/remove behavior, and backend failure redaction. 

No real OS or filesystem adapter was added in Phase 15I itself.

---

## Phase 15J — Desktop atomic VaultStore

### Added to CrabLink Tauri

```text
apps/crablink-tauri/src-tauri/src/passport_vault_store.rs
apps/crablink-tauri/src-tauri/tests/phase15j_desktop_atomic_vault_store.rs
```

Supporting integration involved:

```text
apps/crablink-tauri/src-tauri/src/passport_vault_runtime.rs
apps/crablink-tauri/src-tauri/src/lib.rs
apps/crablink-tauri/src-tauri/src/state.rs
```

### Behavior implemented

```text
bounded encrypted vault bytes only
dedicated vault directory and paths
temporary-file write
flush/sync before replacement
atomic replacement strategy
interrupted-write recovery
safe removal
no plaintext temporary vault
no secret logging
no identity parsing
```

The VaultStore is a persistence mechanism, not the owner of vault crypto or Passport state rules.

---

## Phase 15K — Windows replacement semantics and app-data wiring

### Main files

```text
apps/crablink-tauri/src-tauri/src/passport_vault_store.rs
apps/crablink-tauri/src-tauri/src/passport_vault_runtime.rs
apps/crablink-tauri/src-tauri/src/lib.rs
apps/crablink-tauri/src-tauri/src/state.rs
apps/crablink-tauri/src-tauri/tests/phase15k_windows_atomic_replace_and_app_data_store_wiring.rs
```

### Behavior established

```text
desktop app-data location is used for the Native Passport vault
Windows replacement semantics are handled separately from Unix rename behavior
AppState owns the initialized DesktopAtomicVaultStore
startup performs interrupted-write recovery
the VaultStore is managed once rather than reconstructed per command
```

The initial Phase 15K patch encountered an anchor mismatch while editing startup/AppState wiring. The repair used the actual current source structure and then passed its focused regressions.

---

## Phase 15L — macOS Keychain PlatformSealer

### Added

```text
apps/crablink-tauri/src-tauri/src/passport_platform_sealer.rs
apps/crablink-tauri/src-tauri/tests/phase15l_macos_keychain_platform_sealer.rs
```

Cargo changes included the exact macOS Keychain dependency:

```text
security-framework = 3.7.0
```

### Design

The adapter uses a fixed service namespace and distinct compartment accounts.

It maintains separate entries for:

```text
RecoveryRoot
DeviceKey / operational factor
```

The persisted vault stores only the bounded Keychain reference, not the plaintext factor.

### Verified behavior

```text
create/update and read behavior
independent root and operational compartment entries
reference mismatch rejection
platform mismatch rejection
redacted backend failures
redacted Debug
no secret material in errors
```

The current CrabLink codebundle includes the independent-compartment and mismatch tests. 

---

## Phase 15M — Windows DPAPI PlatformSealer

### Added

```text
apps/crablink-tauri/src-tauri/src/passport_platform_sealer_windows.rs
apps/crablink-tauri/src-tauri/tests/phase15m_windows_dpapi_platform_sealer.rs
```

### Behavior

```text
Windows-only adapter
uses DPAPI protection/unprotection
returns bounded sealed bytes
binds the sealed material to the expected platform and compartment
does not expose a reusable platform device secret
redacts OS/backend failure details
```

Unlike reference-based Keychain/Secret Service adapters, DPAPI may persist the protected bytes themselves inside the platform-bound vault envelope.

No Windows API behavior is compiled as active macOS runtime behavior.

---

## Phase 15N — Linux Secret Service PlatformSealer

### Added

```text
apps/crablink-tauri/src-tauri/src/passport_platform_sealer_linux.rs
apps/crablink-tauri/src-tauri/tests/phase15n_linux_secret_service_platform_sealer.rs
```

### Behavior

```text
Linux-only Secret Service adapter
fixed service/application namespace
distinct root and operational entries
opaque bounded references persisted in the vault
platform and compartment mismatch rejection
backend failures mapped to redacted storage errors
no plaintext fallback
```

This is what was previously referred to as the “Linux Secret Service crate”: it is the Rust interface used to interact with the Linux desktop secret-storage service, similar in role to macOS Keychain.

---

## Phase 15O — Platform selection and AppState wiring

### Added

```text
apps/crablink-tauri/src-tauri/src/passport_platform_runtime.rs
apps/crablink-tauri/src-tauri/tests/phase15o_platform_sealer_runtime_selection_and_app_state_wiring.rs
```

Updated:

```text
apps/crablink-tauri/src-tauri/src/lib.rs
apps/crablink-tauri/src-tauri/src/state.rs
```

### Runtime ownership after Phase 15O

`AppState` owns:

```text
DesktopAtomicVaultStore
Arc<dyn NativePlatformSealer>
```

Startup:

```text
selects exactly one supported desktop PlatformSealer
initializes the app-data VaultStore
runs interrupted-write recovery
places both objects in AppState
manages AppState through Tauri
```

The runtime selector contains the three desktop adapters:

```text
MacosKeychainPlatformSealer
WindowsDpapiPlatformSealer
LinuxSecretServicePlatformSealer
```

It does not call `seal`, `unseal`, decrypt, or unlock during startup. The Phase 15O tests explicitly guard those boundaries. 

---

## Phase 15P — Real stored status

### Added

```text
apps/crablink-tauri/src-tauri/src/passport_status_runtime.rs
apps/crablink-tauri/src-tauri/tests/phase15p_real_stored_status_app_state_wiring.rs
```

Updated:

```text
apps/crablink-tauri/src-tauri/src/commands/passport.rs
apps/crablink-tauri/src-tauri/src/lib.rs
```

### Status behavior

```text
VaultStore contains no vault:
  NoPassport

VaultStore contains a valid bounded encrypted vault:
  Locked

VaultStore read or validation fails:
  redacted unavailable problem
```

The status command reads only:

```text
state.passport_vault_store
```

It does not touch:

```text
state.passport_platform_sealer
seal/unseal
decryption
unlock
storage writes
capability material
wallet or ledger
```

The current Phase 15P tests confirm absence maps to `NoPassport`, a valid encrypted vault maps to `Locked`, malformed storage fails closed, and the command reads only the managed VaultStore. 

---

## Phase 15Q — Native vault cryptography and operational unlock primitive

### `svc-passport` dependency additions

The `native-passport` feature now activates:

```text
argon2
chacha20poly1305
hkdf
sha2
```

Added:

```text
crates/svc-passport/src/native/vault_crypto.rs
crates/svc-passport/tests/native_passport_phase15q_native_vault_crypto_and_operational_unlock.rs
```

Updated:

```text
crates/svc-passport/Cargo.toml
crates/svc-passport/src/native/mod.rs
```

### Cryptographic pipeline

```text
PIN
→ Argon2id PIN key
→ HKDF-SHA-256 with platform factor and compartment/domain separation
→ KEK
→ XChaCha20-Poly1305 wraps compartment VMK
```

### Runtime parameter posture

```text
Argon2 algorithm: Argon2id
Argon2 version: 1.3 / V0x13
derived key length: 32 bytes

recovery-root profile:
  memory: 64 MiB
  time cost: 3
  parallelism: 1

operational/device profile:
  memory: 32 MiB
  time cost: 3
  parallelism: 1

runtime salt: 16 bytes
XChaCha nonce: 24 bytes
authentication tag: 16 bytes
VMK: 32 bytes
platform factor: 32 bytes
```

### Phase 0I compatibility

The code preserves the existing Phase 0I authenticated vault-header fixture format and tests the exact canonical header builder against the locked JSON fixture.

Do not change the Phase 0I fixture values into production defaults. The fixture explicitly marks its sample salts/nonces as non-production material.

### Types and codec

```text
NativePinWrappedCompartmentVmkV1
NativePinWrappedVaultKeysV1
encode_native_pin_wrapped_vault_keys
decode_native_pin_wrapped_vault_keys
unlock_native_operational_vmk
```

The wrapped-key codec uses:

```text
magic: RONPVK01
version: 1
two required compartment records
bounded total size
strict truncation and trailing-byte rejection
```

### Authority boundary

A public operational unlock primitive exists.

A public recovery-root unlock primitive deliberately does not yet exist.

### `ron-kms` review and decision

The session reviewed `ron-kms` before adding vault cryptography.

Decision:

```text
ron-kms owns general Ed25519 key lifecycle, signing, verification,
rotation, attestation, and transient soft-seal behavior.

svc-passport owns the Native Passport persistent vault format,
PIN KDF, compartment KEKs, VMK wrapping, and vault acceptance rules.
```

The `ron-kms` `soft-seal` feature is transient/RAM-oriented and is not the persistent Passport vault format.

No direct `ron-kms` dependency was added to `svc-passport` for Phase 15Q.

A transitive `ron-kms` appearance in the workspace dependency tree is not the same as a direct Phase 15Q dependency.

### Verification issue and workflow change

The first Phase 15Q verification was too broad and took approximately 15–20 minutes.

It also had two verification-script defects:

```text
broken multiline shell arithmetic in failure counter
dependency scan treated transitive ron-kms presence as a direct dependency
```

The corrected focused closeout passed:

```text
6 Phase 15Q crypto tests
format check
native check
scoped strict Clippy
source/KMS boundary
direct dependency boundary
```

The project workflow was changed after this:

```text
ordinary patches:
  focused tests
  focused source boundary
  directly related package check

broad regression:
  crypto format changes
  dependency changes
  major milestones
  final Phase 15 acceptance
```

Do not return to 15–20 minute broad regressions after every small patch.

---

## Phase 15R — Platform-bound vault envelope and vault-create orchestration

Phase 15R touched both repositories.

### Added to `svc-passport`

```text
crates/svc-passport/src/native/platform_bound_vault.rs
crates/svc-passport/tests/native_passport_phase15r_platform_bound_vault_envelope.rs
```

Updated:

```text
crates/svc-passport/src/native/mod.rs
```

### Added to CrabLink Tauri

```text
apps/crablink-tauri/src-tauri/src/passport_vault_create_runtime.rs
apps/crablink-tauri/src-tauri/tests/phase15r_platform_factor_and_vault_create_orchestration.rs
```

Updated:

```text
apps/crablink-tauri/src-tauri/Cargo.toml
apps/crablink-tauri/src-tauri/Cargo.lock
apps/crablink-tauri/src-tauri/src/lib.rs
```

Direct Tauri dependency added:

```toml
getrandom = "0.3"
```

### Platform-bound envelope

`NativePlatformBoundVaultV1` persists:

```text
one platform family
sealed recovery-root factor
sealed operational factor
Phase 15Q PIN-wrapped recovery-root VMK
Phase 15Q PIN-wrapped operational VMK
```

It does not persist:

```text
PIN
plaintext platform factor
plaintext VMK
recovery phrase
root private key
device private key
raw capability
```

### Platform-family codes

Stable envelope codes are:

```text
1 = MacosKeychain
2 = WindowsDpapi
3 = LinuxSecretService
4 = IosKeychain
5 = AndroidKeystore
```

`UnknownLocal` is rejected from persistence.

### Vault creation orchestration

`create_desktop_native_passport_vault_with_random`:

```text
rejects an existing vault
validates PIN length before randomness or platform mutation
generates independent root and operational platform factors
generates independent root and operational VMKs
generates independent salts
generates independent nonces
wraps both VMKs through Phase 15Q
seals both platform factors through the selected PlatformSealer
encodes the platform-bound vault
writes it atomically through the VaultStore
returns CreatedLocked
```

### Important limitation

Phase 15R creates the cryptographic **vault container and key-wrapping foundation**.

It does not yet create the full Passport identity payload.

Still absent:

```text
recovery phrase generation
Passport root key generation/derivation
canonical Passport ID creation
device key generation
root-signed device authorization
encrypted root payload
encrypted operational/device payload
backup material presentation
```

Do not describe Phase 15R as a complete `passport_create_native` implementation.

### Phase 15R failures and repairs

The first compile failed because `platform_code` matched only the three desktop platform variants while `NativePlatformFamily` also contains:

```text
IosKeychain
AndroidKeystore
UnknownLocal
```

Repair V1 failed before mutation because its regex assumed a rustfmt layout that did not match the real file.

Repair V2 produced a valid staged edit but its own exact-string validation rejected rustfmt’s wrapped `UnknownLocal` arm. It copied nothing.

The verification block used broken multiline arithmetic and therefore did not stop after the first failure.

Repair V3 used exact current-source replacement followed by formatting-insensitive semantic validation.

V3 succeeded and the final focused tests passed:

```text
svc-passport platform-bound envelope: 6/6
CrabLink vault-create orchestration: 6/6
source boundary: green
```

Final Phase 15R posture:

```text
platform envelope green
secure OS randomness green
sealed root factor green
sealed operational factor green
independent VMKs/salts/nonces green
existing-vault overwrite rejection green
invalid-PIN no-side-effect path green
randomness-failure no-side-effect path green
atomic encrypted write green
created state locked
```

---

## Phase 15S — Operational unseal and locked-vault unlock runtime

Phase 15S changed only CrabLink.

### Added

```text
apps/crablink-tauri/src-tauri/src/passport_operational_unlock_runtime.rs
apps/crablink-tauri/src-tauri/tests/phase15s_operational_unseal_and_locked_vault_unlock_runtime.rs
```

### Updated

```text
apps/crablink-tauri/src-tauri/src/lib.rs
apps/crablink-tauri/src-tauri/src/state.rs
```

### AppState addition

`AppState` now owns:

```text
DesktopOperationalVaultSessionStore
```

Current Native Passport runtime owners in `AppState` are therefore:

```text
DesktopAtomicVaultStore
Arc<dyn NativePlatformSealer>
DesktopOperationalVaultSessionStore
```

### Session states

```text
Locked
Unlocking
OperationalUnlocked
```

### Unlock sequence

```text
validate PIN bounds
reject concurrent or duplicate unlock
load encrypted vault from NativeVaultStore
decode NativePlatformBoundVaultV1
read the platform family
select only the operational sealed factor
unseal only NativeSecureCompartment::DeviceKey
use the PIN + operational factor to unlock only the operational VMK
install the zeroizing operational VMK in native-only session custody
transition to OperationalUnlocked
```

### Failure behavior

```text
invalid PIN length:
  rejected before VaultStore load or platform unseal

no stored vault:
  NoStoredVault
  session returns Locked

wrong PIN:
  generic UnlockRejected
  session returns Locked

wrong operational platform factor:
  generic UnlockRejected
  session returns Locked

missing/unavailable platform material:
  generic UnlockRejected
  session returns Locked

already unlocked:
  rejected without a second platform unseal

concurrent unlock:
  rejected through Unlocking state

explicit lock:
  drops the NativeSecretBytes operational VMK
  returns session to Locked
```

### Root boundary

Phase 15S does not:

```text
access recovery_root_factor()
request NativeSecureCompartment::RecoveryRoot
unseal the recovery-root factor
unlock the recovery-root VMK
create a root session
```

### Frontend boundary

Phase 15S does not add:

```text
#[tauri::command]
PIN serialization
secret DTO
VMK return
React custody
TypeScript custody
root unlock
capability issuance
wallet mutation
ledger mutation
```

### Final Phase 15S verification

```text
format check: green
focused operational-unlock tests: 6/6
Tauri consumer check: green
source boundary: green
```

The six tests proved:

```text
operational-only posture
successful operational unlock
explicit lock drops session material
wrong PIN rejection
wrong platform-factor rejection
missing platform-material rejection
invalid PIN before side effects
absent-vault rejection
native-only AppState ownership
no root or frontend secret surface
```

Final terminal marker:

```text
NATIVE_PASSPORT_PHASE15S_OPERATIONAL_UNSEAL_AND_LOCKED_VAULT_UNLOCK_RUNTIME=GREEN
```

---

# 5. Final architecture at session close

## Persisted on disk

The atomic VaultStore contains one bounded platform-bound vault envelope.

That envelope currently contains:

```text
platform family
sealed root platform factor
sealed operational platform factor
PIN-wrapped root VMK
PIN-wrapped operational VMK
authenticated headers and crypto metadata
```

It does not contain plaintext:

```text
PIN
platform factors
VMKs
recovery phrase
root private key
device private key
```

## Persisted in OS secure storage

Depending on platform:

```text
macOS:
  Keychain entries referenced by the persisted envelope

Windows:
  DPAPI-protected factor bytes may be persisted in the envelope

Linux:
  Secret Service entries referenced by the persisted envelope
```

## Held in memory while locked

```text
no operational VMK
no root VMK
```

## Held in memory after Phase 15S operational unlock

```text
operational VMK only
inside DesktopOperationalVaultSessionStore
inside Rust/native memory
zeroizing on drop
```

Still absent:

```text
decrypted operational compartment payload
device private key session
capability session
recovery-root session
```

---

# 6. Security invariants preserved throughout the session

The following remained true from Phase 15E through Phase 15S:

```text
React does not receive PINs.
React does not receive recovery phrases.
React does not receive root keys.
React does not receive device private keys.
React does not receive VMKs.
React does not receive platform factors.
React does not receive raw capabilities.

No PIN is persisted.
No plaintext platform factor is persisted.
No plaintext VMK is persisted.
No plaintext vault fallback exists.
No raw platform device secret API exists.

Ordinary unlock does not touch recovery-root material.
Root unlock requires a later separate explicit confirmation flow.

svc-passport remains the canonical Passport logic owner.
CrabLink Tauri remains the desktop platform/runtime adapter.

ron-kms key lifecycle was not duplicated.
ron-kms transient soft-seal was not reused as the Passport vault.

No wallet mutation was added.
No ledger mutation was added.
No spend authority was added.
No capability issuance was added.
No fake success/finality was added.
```

These rules align with the buildplan’s core invariant that React/JavaScript must never own recovery phrases, PINs, root keys, device private keys, vault keys, or raw long-lived capabilities. 

---

# 7. Files changed during this session

## RustyOnions / `svc-passport`

Confirmed new or updated Phase 15 files:

```text
crates/svc-passport/Cargo.toml

crates/svc-passport/src/kms/client.rs

crates/svc-passport/src/native/mod.rs
crates/svc-passport/src/native/client_status_command_wiring_inspection.rs
crates/svc-passport/src/native/desktop_platform_storage_inspection.rs
crates/svc-passport/src/native/platform_storage.rs
crates/svc-passport/src/native/vault_crypto.rs
crates/svc-passport/src/native/platform_bound_vault.rs

crates/svc-passport/tests/native_passport_phase15e_desktop_status_command_wiring_inspection.rs
crates/svc-passport/tests/native_passport_phase15f_no_dev_kms_consumer_boundary.rs
crates/svc-passport/tests/native_passport_phase15h_platform_sealer_vault_store_inspection.rs
crates/svc-passport/tests/native_passport_phase15i_platform_sealer_vault_store_trait_foundation.rs
crates/svc-passport/tests/native_passport_phase15q_native_vault_crypto_and_operational_unlock.rs
crates/svc-passport/tests/native_passport_phase15r_platform_bound_vault_envelope.rs
```

The latest attached `svc-passport` bundle contains the Phase 15E, 15F, 15H, 15I, 15Q, and initial 15R files. 

Important: the attached bundle was generated before the successful Phase 15R V3 platform-family repair. It is stale for:

```text
src/native/platform_bound_vault.rs
tests/native_passport_phase15r_platform_bound_vault_envelope.rs
```

Regenerate it before the next session.

## CrabLink Tauri

Confirmed new or updated Phase 15 files:

```text
apps/crablink-tauri/src-tauri/Cargo.toml
apps/crablink-tauri/src-tauri/Cargo.lock

apps/crablink-tauri/src-tauri/src/commands/passport.rs
apps/crablink-tauri/src-tauri/src/commands/mod.rs
apps/crablink-tauri/src-tauri/src/lib.rs
apps/crablink-tauri/src-tauri/src/state.rs

apps/crablink-tauri/src-tauri/src/passport_vault_runtime.rs
apps/crablink-tauri/src-tauri/src/passport_vault_store.rs
apps/crablink-tauri/src-tauri/src/passport_platform_sealer.rs
apps/crablink-tauri/src-tauri/src/passport_platform_sealer_windows.rs
apps/crablink-tauri/src-tauri/src/passport_platform_sealer_linux.rs
apps/crablink-tauri/src-tauri/src/passport_platform_runtime.rs
apps/crablink-tauri/src-tauri/src/passport_status_runtime.rs
apps/crablink-tauri/src-tauri/src/passport_vault_create_runtime.rs
apps/crablink-tauri/src-tauri/src/passport_operational_unlock_runtime.rs

apps/crablink-tauri/src-tauri/tests/phase15j_desktop_atomic_vault_store.rs
apps/crablink-tauri/src-tauri/tests/phase15k_windows_atomic_replace_and_app_data_store_wiring.rs
apps/crablink-tauri/src-tauri/tests/phase15l_macos_keychain_platform_sealer.rs
apps/crablink-tauri/src-tauri/tests/phase15m_windows_dpapi_platform_sealer.rs
apps/crablink-tauri/src-tauri/tests/phase15n_linux_secret_service_platform_sealer.rs
apps/crablink-tauri/src-tauri/tests/phase15o_platform_sealer_runtime_selection_and_app_state_wiring.rs
apps/crablink-tauri/src-tauri/tests/phase15p_real_stored_status_app_state_wiring.rs
apps/crablink-tauri/src-tauri/tests/phase15r_platform_factor_and_vault_create_orchestration.rs
apps/crablink-tauri/src-tauri/tests/phase15s_operational_unseal_and_locked_vault_unlock_runtime.rs
```

The previously attached CrabLink bundle includes the Phase 15J–15P files and runtime ownership model. 

It was generated before Phase 15R and Phase 15S and is therefore stale. Regenerate it before the next session.

---

# 8. Codebundles required for the next session

Generate fresh bundles after this session.

## Fresh `svc-passport` bundle

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/make_crate_codex.sh \
  -c svc-passport \
  --force
```

Expected output:

```text
/Users/mymac/Desktop/RustyOnions/crates/svc-passport/CODEBUNDLE.md
```

## Fresh CrabLink bundle

```bash
cd /Users/mymac/Desktop/crablink

bash scripts/make_codebundle.sh
```

Expected output:

```text
/Users/mymac/Desktop/crablink/CODEBUNDLE_TAURI_APP.md
```

## Attach next session

```text
1. These carry-over notes
2. PASSPORT_NATIVE_BUILDPLAN.MD
3. Fresh svc-passport CODEBUNDLE.md
4. Fresh CrabLink CODEBUNDLE_TAURI_APP.md
5. Latest terminal output showing Phase 15S green
```

A separate CrabLink `Cargo.toml` attachment should not be necessary if the fresh CrabLink bundle includes it.

---

# 9. Next patch — Phase 15T

Exact marker:

```text
NATIVE_PASSPORT_PHASE15T_OPERATIONAL_UNLOCK_COMMAND_LOCK_AND_STATUS_BRIDGE
```

## Immediate goal

Connect the internal Phase 15S session runtime to bounded command/status behavior.

The likely command surfaces are:

```text
passport_unlock_operational
passport_lock
passport_status
```

## Critical design warning

Do not implement this unsafe command shape:

```rust
#[tauri::command]
fn passport_unlock_operational(pin: String, ...)
```

That would send the PIN through the WebView/JavaScript/Tauri serialization boundary and violate the buildplan.

The next patch must inspect and use the existing native secure-surface contract or introduce a real native secure PIN-entry adapter.

Acceptable approaches include:

```text
native dialog collects the PIN directly in Rust/platform code
native secret-surface port returns NativeSecretBytes
Tauri command triggers the native prompt but does not receive the PIN
tests inject a fake NativeSecretSurfacePort without exposing production secrets
```

The production command should return only a redacted result such as:

```text
OperationalUnlocked
AlreadyUnlocked
NoPassport
UnlockRejected
Unavailable
```

It must not return:

```text
PIN
VMK
platform factor
sealed factor
vault bytes
root state details beyond safe redacted lock posture
```

## `passport_lock`

`passport_lock` can safely become a public Tauri command because it requires no secret input.

It should:

```text
call DesktopOperationalVaultSessionStore::lock()
drop the zeroizing operational VMK
return a redacted lock DTO
be idempotent or clearly report AlreadyLocked
not modify the encrypted vault
not delete platform secure-storage entries
```

## `passport_status`

Update the existing status command so it considers both:

```text
persistent VaultStore state
in-memory operational session state
```

Expected safe states:

```text
NoPassport
Locked
OperationalUnlocked
```

`RootUnlocked` must remain unavailable until a real separate root session and confirmation path exists.

The status command must not infer operational unlock merely because a vault exists.

## Phase 15T focused tests

The next test should prove:

```text
NoPassport when no vault exists
Locked when a vault exists and session is locked
OperationalUnlocked when a valid session exists
passport_lock drops the session and returns to Locked
wrong PIN returns one generic redacted failure
no recovery-root unseal occurs
PIN never appears in output, errors, logs, Debug, Serialize, or source DTO
VMK never appears in output
forbidden commands remain absent
```

---

# 10. Work required after Phase 15T

The following order is recommended.

These are functional work slices, not authoritative letter names unless the next session explicitly assigns them.

## A. Native secure input implementation

Complete the real native secret-entry surface for:

```text
PIN entry
PIN confirmation
recovery phrase display
recovery phrase import
root-operation confirmation
```

Required properties:

```text
no React/TypeScript secret ownership
no clipboard by default
no URL/deep-link secret input
no localStorage/sessionStorage/indexedDB
redacted cancellation and failure
screen-capture protections where available
```

## B. Full Passport creation

Current Phase 15R creates only the vault/key-wrapping container.

A complete `passport_create_native` still needs:

```text
recovery entropy
recovery phrase
deterministic root key derivation
root public key
canonical Passport ID
random device key
canonical Device ID
root-signed device authorization
encrypted root payload
encrypted operational/device payload
backup presentation and acknowledgement
atomic final vault write
truthful created/locked result
```

Creation must not claim success until the recovery backup acknowledgement and atomic vault write are complete.

## C. Explicit root unlock and confirmation

Add a separate root session:

```text
ordinary operational unlock must not imply root unlock
root factor unseal happens only after explicit native confirmation
root VMK has a shorter timeout
root VMK is dropped immediately after high-risk operation or timeout
root status remains redacted
```

High-risk actions include:

```text
authorize device
revoke privileged device
restore/export backup
clear Passport
username transfer/release
other root/admin transitions
```

## D. Restore/import and backup

Implement:

```text
restore phrase through native secure input
same phrase → same root and Passport ID
new device → new device key and Device ID
new platform factors, salts, nonces, and ciphertext
new local PIN may differ
copied vault fails on the wrong platform binding
backup acknowledgement and cancellation
```

## E. Clear/reset

`passport_clear` should:

```text
require explicit root/native confirmation
drop operational/root sessions
remove encrypted vault atomically
delete both platform secure-storage entries
clear active capability/session metadata
return a redacted result
fail closed on partial deletion
```

## F. Device commands

Still required:

```text
passport_device_list
passport_device_authorize
passport_device_revoke
```

The existing Phase 3/7 contracts should be reused rather than duplicated.

## G. Proof transport

Still required:

```text
passport_prove
challenge fetch/verification
root or device signing through native custody
request-proof creation
replay/idempotency binding
gateway/omnigate transport
redacted errors
```

Existing Phase 8–10 `svc-passport` proof contracts and adapters must be called rather than reimplemented in CrabLink.

## H. Capability lifecycle

Still required:

```text
passport_capability_status
passport_capability_refresh
passport_capability_revoke
```

Capability material must be stored only in the operational encrypted compartment.

React may receive only redacted status, expiry, and safe scope summaries.

## I. Username lifecycle

Still required:

```text
passport_username_status
passport_username_claim
passport_username_transfer
passport_username_release
```

The existing canonical `ron-naming` parser and `svc-passport` username contracts must be reused.

`@username` remains:

```text
optional public handle
not Passport ID
not a legal name
not wallet authority
not recovery material
```

## J. Real Passport label replacement and React integration

After a real Passport is active:

```text
replace the development identity label
preserve dev compatibility behind an explicit feature
add redacted Tauri/TypeScript adapters
update Passport UI
show NoPassport / Locked / OperationalUnlocked truthfully
never place secret material in React state
```

## K. Final desktop acceptance

Before Phase 15 can close:

```text
create and backup on desktop
restart and return Locked
operational unlock without root unlock
explicit lock and auto-lock
separate root confirmation
wrong PIN failure
missing platform key failure
corrupt/truncated vault failure
restore same Passport on second device
different device key/ID
device authorization and revocation
proof and request proof
capability issue/refresh/revoke
username claim/status
React no-secret scan
forbidden command scan
desktop build/check
focused Clippy
final milestone regression
```

---

# 11. Expected and forbidden command inventory

The buildplan’s expected desktop commands remain:

```text
passport_status
passport_create_native
passport_restore_native
passport_unlock_operational
passport_unlock_root
passport_lock
passport_clear
passport_device_list
passport_device_authorize
passport_device_revoke
passport_prove
passport_capability_status
passport_capability_refresh
passport_capability_revoke
passport_username_status
passport_username_claim
passport_username_transfer
passport_username_release
```

The permanently forbidden commands remain:

```text
passport_get_seed_to_webview
passport_export_private_key
passport_get_device_private_key
passport_get_raw_capability
passport_issue_arbitrary_scope
passport_disable_policy
```

These inventories are part of the Phase 15 buildplan and must remain enforced through final acceptance. 

---

# 12. Verification workflow for the next session

Use the focused lane.

After a normal Phase 15T patch, run only:

```text
format check for touched crate/repo
new Phase 15T test
direct Phase 15S regression if the session store changed
Tauri consumer check
source/secret boundary
```

Do not automatically run:

```text
cargo check --workspace
all svc-passport tests
all Tauri tests
full TV acceptance
desktop production build
armv7 Android check
full codebundle regeneration between every small patch
workspace-wide Clippy
```

Run broad regressions only when:

```text
vault format changes
crypto parameters change
Cargo dependency graph changes
shared public contract changes broadly
a major Phase 15 milestone closes
final Phase 15 acceptance begins
```

Patch style must remain:

```text
plain-text paste-ready Bash
no base64/gzip/opaque payloads
separate mutation and verification
repository unchanged on preflight abort
restore files after failed final copy
real rustfmt before mutation
stop verification after first failure
compact valid shell arithmetic:
  failure_count=$((failure_count + 1))
```

Do not reuse the broken form:

```bash
failure_count=$(
  (
    failure_count + 1
  )
)
```

---

# 13. Safe claims at the end of this session

Correct:

```text
Phase 15E through Phase 15S are green.
CrabLink desktop has real atomic encrypted-vault persistence.
macOS, Windows, and Linux PlatformSealer adapters exist.
AppState owns the VaultStore and selected PlatformSealer.
Stored-vault status truthfully reports NoPassport or Locked.
svc-passport implements PIN-derived compartment VMK wrapping.
A platform-bound vault envelope is persisted.
Vault creation generates independent random factors, VMKs, salts, and nonces.
Operational unlock can recover only the operational VMK.
The operational VMK can be held in zeroizing native-only session custody.
Explicit lock drops the operational VMK.
Recovery-root material remains sealed during ordinary unlock.
React receives no secret material.
```

Incorrect:

```text
Phase 15 is complete.
passport_create_native is complete.
A full Passport identity is created.
Recovery phrase backup exists.
Restore exists.
Root unlock exists.
Device private-key payload decryption exists.
The public operational-unlock Tauri command exists.
React can enter or submit a PIN safely.
Proof/capability/username desktop flows are complete.
The app is security-audited or production-ready.
```

---

# 14. Final next-session opener

Use this text at the start of the next session:

```text
We are continuing the Passport Native Buildplan in Phase 15.

The previous session began at Phase 15E and completed Phase 15E through
Phase 15S.

Latest green marker:

NATIVE_PASSPORT_PHASE15S_OPERATIONAL_UNSEAL_AND_LOCKED_VAULT_UNLOCK_RUNTIME=GREEN

The next exact patch is:

NATIVE_PASSPORT_PHASE15T_OPERATIONAL_UNLOCK_COMMAND_LOCK_AND_STATUS_BRIDGE

Do not rerun the full Phase 15E–15S regression chain unless a direct
regression appears.

Phase 15T must connect the native operational session to passport_status and
passport_lock, and must design passport_unlock_operational without passing a
plaintext PIN through React/TypeScript/Tauri serialization.

Use the fresh svc-passport and CrabLink codebundles as source of truth.
Keep svc-passport as canonical crypto/contract owner and CrabLink Tauri as the
desktop adapter/runtime owner.
```

---

# 15. Session close summary

```text
SESSION_START=PHASE15E
SESSION_END=PHASE15S
PHASE15E_THROUGH_PHASE15S=GREEN
NEXT_PATCH=PHASE15T
FULL_PHASE15_COMPLETE=NO
ESTIMATED_PHASE15_COMPLETION=62_TO_65_PERCENT
CRYPTO_STORAGE_FOUNDATION=STRONG
USER_OPERABLE_LIFECYCLE=STILL_INCOMPLETE
REACT_SECRET_CUSTODY=ABSENT
ROOT_ORDINARY_UNLOCK=ABSENT
WALLET_OR_LEDGER_MUTATION=ABSENT
BROAD_REGRESSION_RERUN=NOT_NEEDED_AT_SESSION_CLOSE
FRESH_CODEBUNDLES_REQUIRED=YES
```

The fresh bundles should be generated before opening the next implementation session so Phase 15T starts from the exact Phase 15S source state.


### END NOTE - JULY 24 2026 - 16:30 CST



### BEGIN NOTE - JULY 25 2026 - 23:30 CST


Below are comprehensive carry-over notes you can paste into **CrabLink** and **RustyOnions** notes/changelog. They document the CrabLink-only work, the Phase 15 desktop Passport closeout, the live-app findings, and why the next session must start with `ONBOARDING_BUILDPLAN.md` before moving to Passport Native Phase 16. The current codebundle confirms the active repo/app surface is CrabLink desktop/Tauri plus TV sources, with the new Phase 15 Passport drawer/adapter/boundary files present in `apps/crablink-tauri`. 

````markdown
# CrabLink / RustyOnions Carry-Over + Changelog Notes
## Session: Native Passport Desktop Closeout → Onboarding Buildplan Start
## Scope changed this session: CrabLink only

## High-level session summary

This session closed the CrabLink desktop side of the Native Passport Phase 15 work and exposed the next real product problem: the Passport/native command bridge now compiles and tests green, but CrabLink still boots with development-era identity assumptions and leftover local/dev artifacts. The app can now run as a desktop Tauri app and show the Native Passport drawer, but the next necessary lane is not Android TV yet. We need a clean first-run onboarding flow before attempting Phase 16 of the Passport Native Buildplan.

The new build lane is now named:

`ONBOARDING_BUILDPLAN.md`

This onboarding buildplan must be worked before continuing into Passport Native Phase 16 because Phase 16 is about CrabLink TV Passport integration, and we need to prove the desktop user onboarding lifecycle first. Desktop is where we can validate the new wallet-like Passport flow, remove baked dev identities, prove clean first-run state, prove user-chosen username entry, prove local Passport creation, prove recovery/PIN/profile flow, and make sure the existing Passport work actually behaves like a real user flow rather than a developer test panel.

Only the CrabLink repo/app surface changed this session. RustyOnions crates such as `svc-passport` were compiled as dependencies during verification, but were not edited in this session.

---

## What changed in CrabLink

Primary changed surface:

`apps/crablink-tauri`

This includes the desktop Tauri React app, the Tauri platform boundary, the Native Passport adapter, the Passport drawer, and the Phase 15 source/regression tests.

Files changed or added during the session include:

- `apps/crablink-tauri/src/adapters/passportAdapter.js`
- `apps/crablink-tauri/src/adapters/passportAdapter.test.mjs`
- `apps/crablink-tauri/src/app/shell/PassportDrawer.jsx`
- `apps/crablink-tauri/src/app/shell/PassportDrawer.nativePassport.source.test.mjs`
- `apps/crablink-tauri/src/app/shell/PassportDrawer.nativePassport.statusAcceptance.source.test.mjs`
- `apps/crablink-tauri/src/app/shell/PassportDrawer.nativePassport.commandAcceptance.source.test.mjs`
- `apps/crablink-tauri/src/app/shell/PassportDrawer.nativePassport.manualAcceptance.source.test.mjs`
- `apps/crablink-tauri/src/app/shell/PassportDrawer.nativePassport.phase15FinalAcceptance.source.test.mjs`
- `apps/crablink-tauri/src/platform/tauriPlatform.js`
- `apps/crablink-tauri/src/platform/tauriPlatform.passportBoundary.source.test.mjs`
- `ONBOARDING_BUILDPLAN.md`

The Phase 15 work gave the desktop app a real, test-covered Native Passport drawer path. The drawer now uses a safe adapter rather than raw Tauri invocation. The adapter maps only fixed Native Passport commands. The React drawer has no PIN argument path, no seed phrase path, no root export path, no wallet or ledger mutation path, and no capability/username mutation path.

The new root buildplan file `ONBOARDING_BUILDPLAN.md` was created to define the next product lane.

---

## Native Passport Phase 15 desktop status

Phase 15 desktop final acceptance is green.

The final Phase 15 acceptance output confirmed:

- `NATIVE_PASSPORT_PHASE15_FINAL_ACCEPTANCE=GREEN`
- `PHASE15AG_FINAL_ACCEPTANCE=GREEN`
- `RUST_COMMAND_BRIDGE_FINAL_ACCEPTANCE=GREEN`
- `PASSPORT_ADAPTER_FINAL_ACCEPTANCE=GREEN`
- `PASSPORT_DRAWER_FINAL_ACCEPTANCE=GREEN`
- `PASSPORT_DTO_SAFETY_FINAL_ACCEPTANCE=GREEN`
- `PASSPORT_PHASE15_RUNTIME_COMMANDS=FIXED`
- `PASSPORT_PHASE15_DRAWER_STATUS_TRUTH=GREEN`
- `PASSPORT_PHASE15_MANUAL_ACCEPTANCE=GREEN`
- `PASSPORT_PHASE15_REACT_BUILD=GREEN`
- `PASSPORT_PHASE15_TAURI_CONSUMER_CHECK=GREEN`
- `PASSPORT_PHASE15_PIN_ARGUMENTS=NO`
- `PASSPORT_PHASE15_SECRET_CUSTODY=NO`
- `PASSPORT_PHASE15_ROOT_EXPORT=NO`
- `CAPABILITY_ISSUANCE=NOT_ADDED`
- `USERNAME_MUTATION=NOT_ADDED`
- `WALLET_OR_LEDGER_MUTATION=NO`

This means the desktop Native Passport bridge is now accepted at the source/test/build level.

The Rust command bridge, Tauri handler registration, JS adapter command map, Passport drawer command buttons, status truth rows, manual acceptance checklist, and redacted DTO posture are all covered by tests.

The final acceptance lane also ran relevant regressions:

- Phase 15AB Passport adapter regression
- Phase 15AC drawer native command regression
- Phase 15AD native status acceptance regression
- Phase 15AE native command acceptance regression
- Phase 15AF manual acceptance regression
- Phase 15AA clear command bridge regression
- Phase 15Z root confirmation regression
- Phase 15Y create/status command regression
- Phase 15T Passport command unit regression
- React production build
- Tauri consumer `cargo check`

---

## Phase 15AD completed: Native status truth rows

Phase 15AD added explicit Native Passport status truth rows to the Passport drawer.

What it added:

- visible Native Passport status truth section
- redacted/absent Passport identifier display
- redacted/absent device identifier display
- redacted/absent username handle display
- redacted/absent capability material display
- unsafe status flag row
- native runtime readiness row
- command DTO schema row
- command safety rows

Security posture:

- display-only
- no raw invoke
- no callTauri in the drawer
- no React PIN argument
- no secret custody
- no root material exposure
- no wallet/ledger mutation
- no username mutation
- no capability issuance

A regression issue appeared when later manual acceptance work accidentally removed the “Last native command truth” block. That was repaired and the Phase 15AD regression returned green.

---

## Phase 15AE completed: Native command acceptance

Phase 15AE added a cross-surface command acceptance test proving the current command chain agrees from Rust to adapter to drawer.

It covered:

- Rust command declarations
- Tauri handler registration
- JS `passportAdapter.js` fixed command mapping
- Passport drawer button wiring through adapter functions only
- no duplicated Tauri command strings in the drawer
- no dynamic raw command dispatch
- no PIN/password/secret/string argument in the public command chain
- no unsafe exports such as seed/private key/root/VMK
- no capability issuance
- no username mutation
- no wallet/ledger mutation

The accepted fixed command set is:

- `passport_status`
- `passport_create`
- `passport_lock`
- `passport_unlock_operational`
- `passport_unlock_root`
- `passport_clear`

There were two test expectation repairs during this phase:

1. The test initially looked for the wrong Phase 15AD label.
2. The test initially looked for the wrong Rust operational unlock constant name.

Both were repaired without changing runtime behavior.

---

## Phase 15AF completed: Manual acceptance checklist

Phase 15AF added a visible manual acceptance checklist to the Passport drawer.

The checklist is intended for human app testing and includes:

- runtime boundary
- create path
- operational unlock path
- lock path
- root confirmation path
- clear path
- React secret boundary
- outcome checkpoint

The checklist explicitly reminds that:

- PIN, password, recovery words, and root material are never entered in React
- root-sensitive confirmation stays redacted
- clear should return the drawer to `no_passport`
- create/unlock should use native secure surfaces, not WebView inputs

A source-test whitespace false positive occurred around the `nativeRuntimeAvailable ?` ternary. That was repaired by rewriting the test to allow formatting whitespace.

A second regression occurred because the manual checklist insertion displaced the Phase 15AD “Last native command truth” block. That was repaired by restoring the block while preserving the manual checklist.

Final Phase 15AF verification passed:

- manual acceptance source tests green
- Phase 15AD status acceptance regression green
- Phase 15AE command acceptance regression green
- Phase 15AC drawer native command regression green
- Phase 15AB Passport adapter regression green
- React build green
- Tauri consumer check green

---

## Phase 15AG completed: Desktop Phase 15 final acceptance

Phase 15AG added the final source-level acceptance test for the entire desktop Passport Phase 15 lane.

It proves:

- final acceptance files and phase labels are present
- command chain remains fixed from Rust to adapter to drawer
- React surfaces do not carry PINs
- React surfaces do not call raw Tauri invoke directly
- React surfaces do not duplicate native command authority
- status truth rows are display-only
- manual acceptance rows are display-only
- Rust regression coverage anchors remain present
- unsafe Passport commands are not exposed

This was the formal closeout of Desktop Native Passport Phase 15.

Result:

`NATIVE_PASSPORT_PHASE15_FINAL_ACCEPTANCE=GREEN`

---

## Live app test observation

After building/running CrabLink as a macOS desktop app, the Passport drawer appeared in the UI and showed the Native Passport runtime/status/manual acceptance surfaces. This is good: it proves the app build has the drawer and the user-visible Passport surface.

However, the live app revealed a product/runtime issue: the JS CrabLink Tauri boundary policy was rejecting Native Passport commands before they reached Rust. The toasts showed errors such as:

- `passport_create` not authorized by CrabLink boundary policy
- `passport_status` not authorized by CrabLink boundary policy
- `passport_unlock_operational` not authorized by CrabLink boundary policy

This was not a Rust/native Passport failure. It was the JS `tauriPlatform.js` command boundary refusing the new fixed Passport command names.

A Phase 15AH boundary-admission patch was prepared to admit the accepted fixed command set into the CrabLink boundary allowlist, with a source test:

- `apps/crablink-tauri/src/platform/tauriPlatform.passportBoundary.source.test.mjs`

The current codebundle includes this boundary test file. At the start of the next session, confirm whether Phase 15AH verification has already been run green. If not, run the Phase 15AH boundary admission verification and rebuild/relaunch the app before or alongside Onboarding Phase 0.

The intended Phase 15AH acceptance markers are:

- `NATIVE_PASSPORT_PHASE15AH_BOUNDARY_ADMISSION=GREEN`
- `PASSPORT_NATIVE_COMMAND_BOUNDARY=GREEN`
- `PASSPORT_NATIVE_COMMANDS_ADMITTED_FOR_APP_TEST=YES`
- `CRABLINK_MACOS_APP_REBUILT=YES`
- no raw invoke added
- no PIN arguments added
- no secret custody added
- no capability issuance
- no username mutation
- no wallet/ledger mutation

---

## Why we are not moving to Passport Native Phase 16 yet

The Passport Native Buildplan originally points next toward Phase 16, which is CrabLink TV Passport integration.

We should not start Phase 16 yet.

Reason: desktop has to prove the real first-run Passport/onboarding flow before we port/integrate it into Android TV.

The work we just finished proves the command bridge and drawer surfaces, but it does not yet prove a clean user experience. The app still contains leftover development assumptions and visible dev artifacts. If we move straight to TV now, we risk copying the wrong behavior to Android TV:

- baked dev Passports
- baked visitor identity
- baked/current username such as `@skinnycrabby`
- Passport A / Passport B concepts showing as if they are user choices
- home/dashboard booting before a real first-run identity decision
- local proof/cache/dev data being confused with user account state

The desktop app needs a real onboarding lifecycle first:

1. clean first launch
2. welcome page
3. username entry
4. username availability check or dev-only bypass
5. local Passport creation
6. recovery phrase ceremony
7. PIN setup through native secure input
8. profile setup or skip
9. completion marker
10. home handoff

Once that flow works on desktop, then Phase 16 can safely adapt the same state model and Passport semantics to Android TV.

---

## New buildplan created: ONBOARDING_BUILDPLAN.md

A new buildplan was created:

`ONBOARDING_BUILDPLAN.md`

Purpose:

Build a clean CrabLink first-run onboarding flow, first on desktop, later portable to TV/mobile/tablet.

The buildplan explicitly defines:

- problem statement
- product goal
- identity model
- PIN/recovery phrase security rules
- desired first-run state machine
- desktop first-run routing behavior
- cleanup targets
- expected files to touch
- phases 0 through 12
- manual desktop test checklist
- what is not in scope

Key security correction in the plan:

The user should see a seed phrase / recovery phrase, but the raw phrase must not pass through React/WebView. The secure recovery phrase ceremony should be native/platform-handled. React should only get a redacted acknowledgement DTO such as:

- `recoveryPhraseShown: true`
- `recoveryPhraseAcknowledged: true`
- `redacted: true`
- optional redacted fingerprint/checksum

React must not receive:

- phrase words
- seed bytes
- root material
- VMK
- private key
- PIN
- platform sealer material

---

## Current onboarding problem to solve

CrabLink currently has development-era identity artifacts and assumptions that must be cleaned up or quarantined before beta.

Known problem examples:

- dev Passport sessions
- Passport A / Passport B
- `passport:main:visitor-b`
- `passport:main:dev`
- `acct_dev`
- current/default username like `@skinnycrabby`
- local proof/profile/cache data being presented too much like real account state
- home route opening before first-run onboarding completes
- starter/local grant displays that could look like real account state
- dev tools mixed with normal user flow

These may remain for tests/dev surfaces, but must not appear as normal first-run user state.

---

## Desired new-user onboarding flow

For a clean install or clean local state:

1. User opens CrabLink.
2. Welcome screen appears.
3. User picks a username.
4. App checks username availability.
5. App provides a development-only bypass button for availability checks.
6. User creates a local wallet-like Passport.
7. User is shown a recovery phrase / seed phrase through a native secure recovery surface.
8. User confirms they wrote down the recovery phrase.
9. User is asked to set a PIN through native secure input.
10. User can fill out basic profile info or skip.
11. App marks onboarding complete.
12. User is routed to `crab://home`.

Important: this must be the eventual shared first-use flow for desktop, phone, tablet, TV, and future clients, but the immediate proof target is desktop.

---

## Onboarding buildplan phase order

### Phase 0 — Inventory and baseline proof

Next session should begin here.

Goal: find every baked dev identity/default and add an inventory source test before mutating runtime behavior.

Inventory terms include:

- `skinnycrabby`
- `passport:main:visitor`
- `visitor-b`
- `passport:main:dev`
- `acct_dev`
- Passport A
- Passport B
- starter grant defaults
- dev Passport session labels

Acceptance:

- `ONBOARDING_PHASE0_INVENTORY=GREEN`
- no behavior changed
- no dev artifact removed yet
- clear list of runtime cleanup targets printed

### Phase 1 — Onboarding model foundation

Add pure onboarding state machine and tests.

States include:

- `not_started`
- `welcome`
- `username_entry`
- `username_checking`
- `username_available`
- `username_bypassed_for_dev`
- `passport_create_requested`
- `passport_created_locked`
- `recovery_phrase_required`
- `recovery_phrase_acknowledged`
- `pin_setup_required`
- `pin_setup_complete`
- `profile_setup`
- `profile_skipped`
- `profile_saved`
- `complete`
- `blocked`
- `error`

Acceptance:

- complete state requires username decision, Passport created/locked, recovery acknowledged, PIN setup complete, and profile skipped/saved
- model never stores PIN/seed/root/private material
- `ONBOARDING_PHASE1_MODEL=GREEN`

### Phase 2 — Onboarding storage adapter

Persist only safe redacted onboarding state.

Acceptance:

- clean install reads as not started/welcome
- corrupted state fails safe to onboarding
- reset clears onboarding state
- no secret-shaped field accepted
- `ONBOARDING_PHASE2_STORAGE=GREEN`

### Phase 3 — First-run route gate

Route clean installs to onboarding instead of home.

Acceptance:

- clean app starts on onboarding
- completed app starts on home
- explicit dev bypass visible and marked dev-only
- no silent Passport A/B/visitor fallback
- `ONBOARDING_PHASE3_ROUTE_GATE=GREEN`

### Phase 4 — Welcome and username step

Implement visible welcome and username selection.

Acceptance:

- username is user-entered, not baked in
- `@skinnycrabby` is not defaulted
- Passport A/B not presented as identity choices
- bypass is clearly dev-only
- `ONBOARDING_PHASE4_USERNAME=GREEN`

### Phase 5 — Desktop Passport create handoff

Connect onboarding to accepted Phase 15 native Passport create path.

Acceptance:

- no React PIN input
- no create PIN argument
- native command bridge remains fixed
- created Passport is locked
- redacted failure shown on cancel/unavailable
- `ONBOARDING_PHASE5_DESKTOP_CREATE=GREEN`

### Phase 6 — Recovery phrase ceremony

Add one-time recovery phrase acknowledgement flow without exposing recovery material to React.

Acceptance:

- user can complete “write it down” acknowledgement
- React receives only redacted acknowledgement DTO
- no seed phrase words in React state, props, local storage, or logs
- no root export
- `ONBOARDING_PHASE6_RECOVERY_CEREMONY=GREEN`

### Phase 7 — PIN setup confirmation

Complete PIN setup as native-only step.

Acceptance:

- no WebView PIN input
- native prompt path used
- wrong/cancel/unavailable fails closed
- completion stored only as boolean/redacted status
- `ONBOARDING_PHASE7_PIN_SETUP=GREEN`

### Phase 8 — Profile setup or skip

Collect safe public profile draft fields or allow skip.

Acceptance:

- profile can be skipped
- profile can be saved as local draft
- no baked `@skinnycrabby`
- no backend-confirmed claim without gateway truth
- `ONBOARDING_PHASE8_PROFILE=GREEN`

### Phase 9 — Completion and home handoff

Complete onboarding and land on clean home.

Acceptance:

- clean first-run path reaches home
- status remains honest/local until backend confirms
- no dev identity defaults appear in normal completed flow
- `ONBOARDING_PHASE9_HOME_HANDOFF=GREEN`

### Phase 10 — Dev artifact quarantine

Keep dev helpers but make them impossible to confuse with real onboarding.

Acceptance:

- dev fixtures still available for testing
- new users never see Passport A/B as normal choices
- `@skinnycrabby` not in first-run defaults
- no automatic visitor-b
- `ONBOARDING_PHASE10_DEV_QUARANTINE=GREEN`

### Phase 11 — Desktop final acceptance

Prove desktop onboarding end-to-end.

Acceptance:

- `ONBOARDING_DESKTOP_FINAL_ACCEPTANCE=GREEN`
- app opens to onboarding on clean state
- app opens to home after completion
- no React PIN
- no React seed phrase
- no baked username
- no Passport A/B normal flow
- no wallet/ledger mutation
- no capability issuance
- no fake backend-confirmed username/profile

### Phase 12 — Cross-platform onboarding contract

Freeze portable model for TV/phone/tablet.

Acceptance:

- desktop behavior reusable
- TV/mobile can implement same state machine
- local custody invariant preserved
- `ONBOARDING_CROSS_PLATFORM_CONTRACT=GREEN`

---

## Next session starting point

Start next session with:

`ONBOARDING_PHASE0_DEV_ARTIFACT_INVENTORY`

Do not start with Phase 16 yet.

Recommended first patch:

1. Search/inventory baked dev identity artifacts.
2. Add an inventory source test.
3. Print cleanup targets.
4. Do not mutate runtime behavior yet.
5. Run focused tests/build.

Suggested verification shape:

```bash
node --test apps/crablink-tauri/src/onboarding/onboardingDevArtifactInventory.source.test.mjs
npm --prefix apps/crablink-tauri run build
````

Depending on whether Phase 15AH has been verified, also run or confirm:

```bash
node --test apps/crablink-tauri/src/platform/tauriPlatform.passportBoundary.source.test.mjs
npm --prefix apps/crablink-tauri run build
TAURI_CONFIG="$(cat apps/crablink-tauri/src-tauri/tauri.macos.dev-media.conf.json)" cargo check --manifest-path apps/crablink-tauri/src-tauri/Cargo.toml
```

---

## Sequencing after onboarding

The correct sequence from here is:

1. Finish `ONBOARDING_BUILDPLAN.md` desktop proof.
2. Confirm desktop first-run onboarding works in the built CrabLink app.
3. Then return to Passport Native Buildplan Phase 16 for CrabLink TV Passport integration.
4. After Phase 16 is finished, pause Passport Native if needed because iOS and Android app builds have not started yet.
5. Build out iOS and Android app foundations.
6. Return to remaining Passport Native phases after the mobile foundations exist.

Rationale:

* Phase 16 is TV-specific integration.
* TV should not inherit broken desktop onboarding assumptions.
* iOS/Android phases cannot be meaningfully completed until those app surfaces exist.
* The shared onboarding model should be built once and then reused across desktop, TV, phone, and tablet.

---

## What not to do next

Do not:

* start Android TV Passport integration before desktop onboarding is proven
* keep `@skinnycrabby` as a normal default
* show Passport A/B as normal user identity choices
* silently use visitor-b/dev Passport as a new user
* route clean installs straight to home as if the user already exists
* move PIN entry into React
* show raw recovery phrase in React unless a later audited native/secure-display exception intentionally changes the rule
* add cloud login
* add cloud recovery
* add server custody
* add backend-issued Passport identity
* add wallet/ledger mutation
* add live settlement/mint/burn
* add capability issuance as part of onboarding

---

## Crates/packages changed this session

Changed:

* `apps/crablink-tauri`

  * npm package: `@crablink/crablink-tauri`
  * Rust app crate under it: `crablink-tauri`

Not changed directly:

* `svc-passport`
* `crablink-native-core`
* `ron-*` crates
* `apps/crablink-tv`
* iOS app
* Android phone/tablet app

Important distinction:

`svc-passport` and related RustyOnions crates compiled during Tauri checks because they are dependencies, but they were not edited in this session. These notes can be pasted into RustyOnions as carry-over context, but the actual code changes were CrabLink-side.

---

## Session outcome

The session successfully moved CrabLink from “Native Passport backend/bridge exists” to “desktop app has accepted Native Passport drawer/adapter/status/manual/final acceptance surfaces.” The live app now reveals the correct next product layer: onboarding and dev artifact cleanup.

The project is entering a new phase:

`CrabLink Onboarding`

This must be completed before Passport Native Phase 16 so the TV app receives a real, clean, wallet-like Passport onboarding model rather than inheriting development leftovers.

```
```



### END NOTE - JULY 25 2026 - 23:30 CST