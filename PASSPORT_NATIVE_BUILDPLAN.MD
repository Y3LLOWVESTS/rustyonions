---
title: NATIVE_PASSPORT_BUILDPLAN
version: 0.3.0
status: active-build-plan
last-updated: 2026-07-22
audience: RustyOnions contributors, CrabLink native-app engineers, security reviewers, private-beta testers
repos:
  - RustyOnions
  - crablink
canonical-passport-owner:
  - svc-passport
primary-rustyonions-owners:
  - svc-passport
  - ron-proto
  - ron-auth
  - ron-kms
  - ron-naming
  - ron-policy
  - svc-index
  - svc-storage
  - svc-gateway
  - omnigate
  - ron-audit
primary-crablink-consumers:
  - crates/crablink-native-core
  - packages/crablink-platform
  - apps/crablink-tauri
  - apps/crablink-tv
future-platforms:
  - Android
  - iOS
authoritative-product-reference:
  - CRABLINK_TV_BUILDPLAN
supersedes:
  - NATIVE_PASSPORT_BUILDPLAN v0.2.0
---

# NATIVE_PASSPORT_BUILDPLAN.md — CrabLink Native Passport, Device Authority, Local Vault, Proof, Capability, and `@username` Plan

RO:WHAT — Extends the existing `svc-passport` package into the single canonical Passport owner and builds a wallet-like, client-custodied CrabLink identity that uses a recovery-root key to authorize separate device keys, stores secrets in native encrypted vault compartments, proves device possession to RustyOnions, receives narrow device-bound capabilities, and binds an optional unique `@username` network handle to the Passport through a durable namespace state machine.

RO:WHY — CrabLink needs a decentralized identity model based on local custody and cryptographic proof rather than cloud login, server-held passwords, backend-issued identity, copied root keys on every device, bearer-only long-lived sessions, or platform-specific identity implementations. RustyOnions already has the fixed 33-crate architecture and the canonical `svc-passport` owner, so this work must strengthen those boundaries rather than create a duplicate crate or a second identity authority.

RO:INTERACTS — `svc-passport` native-client and server surfaces, `ron-proto`, `ron-auth`, `ron-kms`, `ron-naming`, `ron-policy`, `svc-index`, `svc-storage`, `svc-gateway`, `omnigate`, `ron-audit`, CrabLink native Rust, Tauri commands, Android Keystore, Apple Keychain/Secure Enclave, desktop secure-storage adapters, capability consumers, receipt and entitlement routes, service-node quorum/checkpoint infrastructure, and user-node replay verification.

RO:INVARIANTS — The recovery phrase and Passport root private key are client-custodied. The root key is not used as the ordinary daily device key. Each device owns a separate device key authorized by the Passport root. A local PIN unlocks a device vault and is never a network password. Proofs are purpose-bound, challenge-bound, signed by authorized keys, and replay-resistant. Capabilities are narrow, expiring, revocable, audience/environment-bound, and bound to an authorized device key. An `@username` is an optional public network handle and never a real/legal human name, cryptographic Passport ID, recovery input, or wallet authority. React and JavaScript never own recovery phrases, PINs, root keys, device private keys, vault keys, or raw long-lived capabilities.

RO:METRICS — Create/import/unlock/lock outcomes, device authorization/revocation, failed unlock classes, proof accept/deny/replay, capability issue/refresh/revoke/request-proof denial, namespace claim/transfer/release outcomes, secure-storage backend posture, vault corruption, redaction events, recovery tests, and cross-platform conformance. No secret, Passport ID, `@username`, raw public key, device ID, challenge nonce, capability, PIN, or site name may become an unbounded or high-cardinality metric label.

RO:CONFIG — Derivation scheme version, root/device signature profiles, vault format version, KDF floors and calibrated device profiles, auto-lock timeout, root-compartment timeout, PIN/passcode policy, secure-storage backend, device-class scope policy, capability scope allowlists, TTL ceilings, clock skew, challenge TTL, replay-store TTL, namespace claim policy, release cooldown, reserved-handle policy, environment/audience/network identifiers, and feature flags.

RO:SECURITY — Standard reviewed primitives only; no custom cipher or signature algorithm. Root and device secrets remain Rust-side wherever possible and are zeroized where feasible. Platform keystores perform device-bound sealing/wrapping operations rather than exporting reusable platform secrets. Recovery phrase and PIN entry use native secure surfaces before production. No clipboard-by-default, recovery QR, secret URL, browser storage, log, diagnostic, crash-dump secret, cloud backup, server recovery, or arbitrary caller-supplied Passport subject.

RO:TEST — Golden root/device derivation vectors, mnemonic checksum tests, device authorization vectors, two-compartment vault round-trips, wrong-PIN/corruption/rollback tests, restart tests, second-device restore, delegated TV enrollment, proof transcript parity, request-proof verification, replay rejection, device revocation, scope/expiry/revocation tests, namespace concurrency/conflict tests, no-secret frontend boundaries, fuzz/property tests, destructive storage tests, and reproducible two-device plus delegated-device acceptance demonstrations.

---

# 0. Decision and Safe Current Label

```text
Native Passport build plan: READY FOR IMPLEMENTATION.
Native Passport build plan version: 0.3.0.
Canonical Passport package owner: svc-passport.
New Passport crate: NOT ALLOWED / NOT NEEDED.
Native client custody implementation: NOT STARTED.
Root/device authority hierarchy: REQUIRED.
Device-bound capability posture: REQUIRED.
Two-compartment vault posture: REQUIRED.
@username namespace state machine: REQUIRED.
Existing svc-passport service/capability/profile foundation: PRESENT BUT REQUIRES RECONCILIATION.
Current CrabLink Passport labels/dev sessions: DISPLAY/DEVELOPMENT ONLY.
First implementation action: Phase 0 source and authority reconciliation.
```

Version 0.3.0 supersedes version 0.2.0.

The central architecture remains:

```text
Do not create a separate CrabLink Passport crate.
Do not add a 34th RustyOnions crate.
Extend svc-passport.
Keep client secrets client-custodied.
Keep service identity/capability signing in service KMS.
Keep economic truth in svc-wallet and ron-ledger.
Keep naming syntax in ron-naming.
Keep content manifests in svc-storage.
Keep read projections in svc-index.
```

Version 0.3.0 adds and corrects:

```text
recovery root key separated from routine device keys
authorization and revocation for each device
root-capable and delegated-device classes
device-bound proof-of-possession capabilities
purpose-bound signed challenges with operation-body binding
two encrypted vault compartments
operation-based platform sealer abstraction
native secure input surfaces moved into early implementation
one canonical Passport ID type and one canonical username parser
explicit Passport registry, @username registry, and site namespace model
private-beta durable writer followed by quorum/checkpoint namespace finality
commit-reveal reservation for public username claims
explicit anti-Sybil/governance decision gate
removal of real/legal human names from identifier terminology
service KMS injection and dev-kms removal from production defaults
production gating of legacy arbitrary-subject and debug routes
```

The correct high-level identity chain is:

```text
recovery phrase
→ Passport root private/public key
→ canonical Passport ID
→ root-signed DeviceAuthorizationV1
→ device private/public key
→ device-signed challenge proof
→ device-bound narrow capability
→ authorized request proof
```

The correct network-name chain is:

```text
canonical Passport ID
→ signed @username claim transition
→ durable namespace finalization
→ @username read projection
→ public profile manifest pointer
```

The `@username` is a user-selected network handle, such as:

```text
@alice
@crabcreator
@music-lab
```

It is not:

```text
a legal name
a birth name
a government identity
a mandatory display name
a cryptographic key
a recovery secret
a wallet account number
```

Real or legal human names may appear only as optional public profile content chosen by the user. They are non-unique, non-authoritative, and never used to derive or prove ownership of a Passport.

---

# 1. Product Thesis and Terminology

## 1.1 Product thesis

CrabLink Passport is a wallet-like identity/account system:

```text
create locally
recover locally
unlock locally
prove control cryptographically
receive temporary capabilities
carry the same Passport across platforms
keep secrets in self custody
```

It is not a cloud account system:

```text
no email/password identity source of truth
no backend-created private identity
no server-held recovery phrase
no password reset that recreates the Passport
no cloud-stored login secret
no server custody disguised as decentralization
```

The network stores and serves mostly public-facing creator content. The Passport provides cryptographic ownership and authorization for actions around that public content; it does not require the network to collect private identity documents or real human names.

## 1.2 Canonical terms

### Passport

The complete cryptographic identity rooted in a recovery phrase and root public key.

### Passport root

The rare-use root authority derived deterministically from the recovery phrase. It authorizes devices and high-risk identity transitions.

### Passport ID

A versioned cryptographic identifier derived from the Passport root public key. It is stable across devices and independent of PIN, `@username`, display name, or site name.

### Device key

A random per-device signing key used for ordinary network proof and request authorization. It is authorized by the Passport root and can be revoked independently.

### Root-capable device

A device whose encrypted root compartment contains recovery-root material. Desktop and personal mobile devices may be root-capable when the user restores or creates the Passport there.

### Delegated device

A device that has only a device key plus a root-signed authorization. It cannot reveal or reconstruct the recovery root. Android TV should use this posture by default.

### Local PIN/passcode

A device-local unlock secret that protects encrypted local vault material. It is not sent to RustyOnions and does not derive the Passport ID.

### `@username` / handle

An optional unique user-selected network identifier beginning with `@`. It points to a Passport ID after a finalized signed claim. It is not an actual human name and is not the cryptographic identity.

### Display name

Optional public profile text. It may contain a real name, stage name, organization name, or nickname if the user chooses. It is non-unique and carries no authority.

### Site name

A user-selected CrabLink site identifier that points to a signed mutable site record whose current content is an immutable BLAKE3-addressed manifest.

### Capability

A narrow, expiring, policy-gated authority token issued after accepted proof. Production-sensitive capabilities are bound to a device key and require fresh request proof.

## 1.3 Identity separation

The following must remain independent:

```text
Passport ID
@username
optional display name
site names
ROC wallet/account
node reward recipient binding
```

No field silently becomes another authority.

Examples:

```text
Passport ID:
  passport:v1:main:ed25519:b3:<64 lowercase hex>

@username:
  @alice

Display name:
  Alice's Art Studio

Site name:
  alice-art

ROC wallet binding:
  separate explicit wallet/account authorization
```

## 1.4 Meaning of “login” in product UI

The product may use familiar wording such as “Unlock Passport” or “Use Passport.” The protocol operation is not password login.

Correct sequence:

```text
unlock local device vault
→ prove possession of authorized device key
→ receive temporary device-bound capability
```

Incorrect model:

```text
send username and PIN to server
→ server authenticates account password
```

---

# 2. Non-Negotiable Authority Boundaries

## 2.1 Client authority

The native client may:

```text
generate recovery entropy
derive the Passport root
generate device keys
encrypt/decrypt local vaults
sign root device authorizations
sign proof challenges
sign capability request proofs
show recovery material through native secure UI
lock and clear local state
```

The native client must not:

```text
mutate ron-ledger directly
mint ROC
invent receipts or entitlements
self-approve capability scopes
claim namespace finality from local cache
create service-node quorum truth
```

## 2.2 svc-passport authority

`svc-passport` owns:

```text
Passport registration state
device authorization/revocation state
challenge issuance and one-time consumption
proof review orchestration
capability issue/refresh/revoke
@username claim/transfer/release state
public Passport status/profile references
identity and namespace transition audit emission
```

It does not own:

```text
client recovery phrases
client PINs
client root private keys
client device private keys
client vault keys
ROC balances or ledger mutation
site/content bytes
```

## 2.3 Economic authority

```text
svc-wallet:
  approved economic mutation front door

ron-ledger:
  durable economic truth
```

Passport authentication can authorize a request to the wallet service later, but Passport is not itself the ROC wallet and `svc-passport` does not mutate balances.

## 2.4 Namespace authority is not economic ledger authority

The network does need durable registries for Passports, `@usernames`, and sites. Those registries are not `ron-ledger`.

Correct separation:

```text
Passport registry:
  identity and device state

@username registry:
  handle ownership transitions

site namespace registry:
  site ownership and manifest-pointer transitions

ron-ledger:
  ROC economic truth only
```

An optional ROC claim fee or refundable bond may be recorded economically through `svc-wallet` and `ron-ledger`, but it does not turn the namespace itself into the economic ledger.

## 2.5 React/JavaScript boundary

React and JavaScript may receive:

```text
absent/locked/unlocked state
redacted Passport ID
confirmed @username
optional display name
redacted device list
capability scope summary
capability expiry
proof result
namespace claim status
redacted typed errors
```

React and JavaScript must never receive or persist:

```text
recovery phrase
mnemonic entropy
root private key
device private key
PIN/passcode
vault master key
platform device factor
raw long-lived capability
unredacted request-proof signing material
```

---

# 3. Existing Repository Posture and Required Reconciliation

## 3.1 Fixed crate canon

RustyOnions uses a fixed 33-crate architecture. Native Passport must stay within the existing owners.

Primary identity/key owners:

```text
ron-kms
ron-auth
svc-passport
```

Related owners:

```text
ron-proto
ron-naming
ron-policy
svc-index
svc-storage
svc-gateway
omnigate
ron-audit
```

A new `native-passport-core`, `crablink-passport`, or other identity crate is forbidden unless the fixed canon is deliberately reopened through a separate reviewed architecture decision. This plan does not reopen it.

## 3.2 Current svc-passport foundation

The existing package already provides useful service and library behavior around:

```text
capability envelopes
verification/preflight
KMS integration
profile/username claim prototypes
HTTP routes
health/readiness/metrics
rotation posture
tests and fuzz targets
```

This foundation must be extended and reconciled rather than discarded.

## 3.3 Current limitations

The existing source does not yet prove the complete Native Passport model:

```text
no recovery phrase generation/restore
no root/device hierarchy
no device authorization/revocation registry
no two-compartment client vault
no device-bound capability proof
no production durable challenge replay store
no production durable global @username state machine
no cross-platform native secure input contract
```

## 3.4 Existing capability and route drift

Phase 0 must inventory and reconcile:

```text
multiple token encodings
legacy capability envelopes
macaroon stubs or incomplete verification paths
service-generated subjects
in-memory username/profile state
DevKms construction in runtime assembly
public/debug/admin route exposure
```

No Native Passport cryptographic implementation begins until the project chooses one canonical V1 encoding, verifier path, KMS injection path, and legacy compatibility posture.

## 3.5 Legacy/dev identities

Markers such as:

```text
passport:main:dev
header-derived identity
dev bearer token
bootstrap subject
URL-provided subject
```

remain development/display compatibility only.

They must never parse as a production `PassportIdV1`.

## 3.6 Current CrabLink posture

Useful current boundaries remain:

```text
crablink-native-core:
  pure non-secret review

packages/crablink-platform:
  platform adapter contracts

Tauri Rust:
  privileged native operations

React:
  display and user intent only
```

Native Passport must not turn `crablink-native-core` into a secret-bearing identity implementation. The secret-bearing implementation stays in `svc-passport::native` and platform-native adapters.

---

# 4. Canonical Crate and Module Ownership

## 4.1 `ron-proto`

Owns strict wire/domain types only:

```text
PassportIdV1
PassportKindV1
PassportRootPublicKeyV1
DeviceIdV1
DeviceClassV1
DevicePublicKeyV1
DeviceAuthorizationV1
DeviceRevocationV1
PassportChallengeRequestV1
PassportChallengeResponseV1
PassportProofV1
PassportProofReviewV1
PassportCapabilityRequestV1
PassportCapabilitySummaryV1
PassportCapabilityRefreshRequestV1
PassportCapabilityRevokeRequestV1
PassportRequestProofV1
UsernameCommitV1
UsernameRevealV1
UsernameTransitionV1
SiteNamespaceTransitionV1
```

Rules:

```text
#[serde(deny_unknown_fields)]
bounded strings and vectors
explicit versions and algorithms
no mnemonic/PIN/private key/vault types
no IO
no policy effects
```

## 4.2 `ron-naming`

Owns canonical parsing and normalization for:

```text
RonUsername / @username
site names
reserved namespace syntax
```

No second username parser may be created in `svc-passport`, CrabLink, Android, or iOS.

`ron-naming` does not decide uniqueness or persist ownership.

## 4.3 `ron-auth`

Owns pure deterministic verification:

```text
root registration transcript
DeviceAuthorizationV1 transcript
DeviceRevocationV1 transcript
challenge transcript
Passport/device proof verification
request-proof verification
Passport ID/root-key binding
Device ID/device-key binding
capability caveat verification
namespace transition signature verification
```

It owns no HTTP, database, vault, challenge store, or namespace persistence.

## 4.4 `ron-kms`

Owns server-side service key management:

```text
challenge-signing service keys
capability-signing service keys
key IDs and versions
rotation and historical verification keys
attestation/sealing traits
zeroization posture
```

It never receives or derives client recovery material.

## 4.5 `svc-passport::native`

Owns portable native client behavior:

```text
mnemonic generation/validation
root derivation
Passport ID derivation
device-key generation
root-signed device authorization
root/device secret wrappers
two-compartment vault format
PIN/passcode KDF and wrapping
lock state
challenge validation before signing
root/device proof signing
request-proof signing
restore and local migration
platform sealing/storage traits
```

## 4.6 `svc-passport` server

Owns:

```text
Passport registration
root-key status and epoch metadata
device authorization/revocation registry
challenge issuance/consume
proof review orchestration
capability issue/refresh/revoke
@username ownership state
public profile pointer state
redacted identity audit events
HTTP/internal service routes
```

## 4.7 `ron-policy`

Owns declarative rules:

```text
device-class scope allowlists
TV read-only posture
challenge and capability TTL ceilings
root-required operations
fresh-proof requirements
reserved @usernames
username length/claim limits through ron-naming types
claim cooldowns
transfer/release rules
anti-Sybil posture
optional ROC fee/bond policy
site namespace policy
```

It does not mutate identity, namespace, wallet, or ledger state.

## 4.8 `svc-storage`

Stores immutable public content:

```text
profile manifests
site manifests
creator content
public device-label metadata only when explicitly intended
```

It does not decide Passport or username ownership.

## 4.9 `svc-index`

Owns read-optimized projections and site namespace runtime where applicable:

```text
@username → Passport/profile pointer projection
site name → current site record projection
content/provider lookup
```

The `@username` projection is derived from finalized `svc-passport` handle transitions and is not the uniqueness authority.

Site-name transition persistence may live in `svc-index` if that crate's reviewed runtime contract is extended for transactional namespace records. Ownership authorization still depends on `svc-passport` proof/capability and `ron-policy`. Phase 0 must freeze the exact persistence owner before implementation.

## 4.10 `ron-audit`

Owns redacted append-only evidence for:

```text
Passport registration
device authorization/revocation
proof denial classes
capability lifecycle
@username claim/transfer/release
site ownership transition
policy denial
```

It must never contain recovery phrases, PINs, private keys, raw capabilities, challenge nonces, or unbounded identifiers as metric labels.

## 4.11 `svc-gateway` and `omnigate`

```text
svc-gateway:
  ingress, rate limits, body caps, route allowlist, error forwarding

omnigate:
  orchestration, policy context, hydrated redacted responses
```

Neither issues identity from arbitrary input, stores client secrets, or decides namespace finality.

## 4.12 CrabLink ownership

```text
crablink-native-core:
  redacted result review only

packages/crablink-platform:
  platform operation contracts

Tauri/Android/iOS native shells:
  native secure input, platform sealer, atomic storage, lifecycle lock

React:
  display and user intent only
```

---
# 5. Root and Device Authority Model

## 5.1 Why the root cannot be the daily key

The recovery-derived root key is the most important Passport secret. Copying and routinely using the same root private key on desktop, Android, iOS, and TV would mean:

```text
one compromised device compromises the entire Passport
lost devices cannot be independently revoked
TV has the same authority as the owner's primary device
routine proof operations repeatedly decrypt the root
root rotation and device inventory become unclear
```

Therefore:

```text
The root key authorizes devices.
Device keys perform ordinary proofs.
```

## 5.2 Root derivation

```text
validated recovery phrase
→ BIP-39 seed
→ versioned HKDF-SHA-256 derivation
→ Ed25519 Passport root key
→ root public key
→ versioned domain-separated BLAKE3 Passport ID
```

The root key is deterministic so the same recovery phrase restores the same Passport ID.

## 5.3 Device-key generation

Each device generates a random device signing key from the operating-system CSPRNG.

```text
Device A:
  random device key A

Device B:
  random device key B

TV:
  random device key TV
```

A device key must not be deterministically derived from:

```text
recovery phrase
PIN
@username
device serial number
advertising ID
MAC address
timestamp
```

Random independent device keys provide revocation and compartmentalization.

## 5.4 DeviceAuthorizationV1

The root authorizes a device by signing a canonical record:

```text
DeviceAuthorizationV1 {
  version,
  network_id,
  environment,
  passport_id,
  root_key_epoch,
  device_id,
  device_public_key,
  device_class,
  authorized_scope_ceiling,
  authorization_nonce,
  issued_at_ms,
  expires_at_ms?,
  root_signature
}
```

The scope ceiling is not itself a live capability. It is the maximum class of capabilities the device may later request.

## 5.5 Device classes

Initial closed enum:

```text
root_admin_desktop
root_admin_mobile
personal_desktop
personal_mobile
tv_read_only
recovery_only
test_harness
```

Policy, not the client, determines allowed scope ceilings.

Suggested posture:

```text
root_admin_desktop:
  device administration and identity management

root_admin_mobile:
  device administration with mobile secure UI

personal_desktop/mobile:
  normal identity/content/publish scopes as later authorized

tv_read_only:
  identity/catalog/content/entitlement/receipt read only

recovery_only:
  no ordinary network capability

test_harness:
  local/test environment only
```

## 5.6 Root-capable and delegated devices

### Root-capable

Created or restored directly from the recovery phrase. Contains a separately encrypted root compartment.

### Delegated

Generated a local device key and received a root-signed authorization from an already root-capable/admin device. Contains no mnemonic or root private key.

Default platform posture:

```text
desktop:
  root-capable or delegated

regular Android/iOS:
  root-capable or delegated

Android TV:
  delegated by default
  recovery-phrase restore deferred/advanced only
```

## 5.7 Device revocation

A root-capable/admin device creates a signed revocation:

```text
DeviceRevocationV1 {
  version,
  network_id,
  passport_id,
  root_key_epoch,
  device_id,
  reason_code,
  effective_at_ms,
  revocation_nonce,
  root_signature
}
```

Server behavior:

```text
persist revocation
reject future device proofs
reject capability refresh
revoke active capabilities bound to device
emit redacted audit evidence
publish updated device status
```

Revocation does not remotely erase a lost device, so short capability TTL and local secure storage remain required.

## 5.8 Root-key epoch and future rotation

V1 records include `root_key_epoch` so a reviewed future root-rotation protocol can exist.

Private-beta V1 may support only epoch `0`.

Do not implement ad hoc root rotation. A later version must define:

```text
old-root authorization of new root
recovery-phrase behavior after rotation
device reauthorization
namespace ownership continuity
capability invalidation
checkpoint finality
```

---

# 6. Recovery Phrase and PIN Semantics

## 6.1 Recovery phrase

Initial profile:

```text
format: BIP-0039-compatible mnemonic
language: English in V1
default: 12 words / 128-bit entropy
advanced: 24 words / 256-bit entropy
checksum: required
normalization: exact V1 profile frozen in vectors
entropy source: operating-system CSPRNG
```

Rules:

```text
no server-generated phrase
no username-derived phrase
no PIN-derived phrase
no analytics/logging
no automatic clipboard
no recovery phrase QR
no cloud sync
no hidden screenshot
```

## 6.2 PIN/passcode

The local PIN/passcode:

```text
protects a device vault
may differ on every device
may be changed without changing Passport ID
is never transmitted to RustyOnions
is never a capability
is never wallet/ledger authority
```

Initial policy:

```text
minimum numeric PIN: 6 digits
recommended: longer PIN or alphanumeric passcode
leading zeroes preserved
bounded input length
no plaintext storage
```

## 6.3 Recovery outcomes

```text
Lost PIN, phrase retained:
  restore root on the device or a new device and choose a new PIN.

Lost phrase, existing authorized device retained:
  continue using that device within its authorization.
  migration may be possible through reviewed device authorization.

Lost delegated device:
  revoke its device key from a root-capable/admin device.

Lost every device and recovery phrase:
  Passport is unrecoverable.
```

No support agent or server process may override this by recreating the Passport.

## 6.4 New-device restore

```text
enter recovery phrase through native secure UI
→ derive same root and Passport ID
→ generate new random device key
→ create root-signed device authorization
→ choose new local PIN
→ create new vault compartments
→ register/refresh device state with network
```

The new device does not copy another device private key.

## 6.5 Delegated-device enrollment

Preferred TV and convenience flow:

```text
new device generates key and pairing request
→ shows public ephemeral pairing code/QR
→ root-capable/admin device reviews device class and scopes
→ root signs DeviceAuthorizationV1
→ encrypted/authenticated authorization transfer
→ new device stores its own key and authorization
```

The pairing payload may include:

```text
ephemeral session public key
device public key
device class request
nonce
expiration
network/environment
short verification code
```

It must not include:

```text
recovery phrase
root private key
PIN
raw long-lived capability
```

---

# 7. Cryptographic Profile V1

Phase 0 freezes exact bytes and produces golden vectors before authority is added.

## 7.1 Algorithms

```text
root signature: Ed25519
device signature: Ed25519
KDF for root derivation: HKDF-SHA-256
Passport/Device ID digest: BLAKE3-256
vault AEAD: XChaCha20-Poly1305
PIN/passcode KDF: Argon2id v1.3
wrapping key combination: HKDF-SHA-256
randomness: operating-system CSPRNG
```

No custom cryptographic primitive is allowed.

## 7.2 Domain separation

Exact byte strings must be frozen. Proposed domains:

```text
rustyonions.native-passport.root-seed.v1
rustyonions.native-passport.root-key.v1
rustyonions.native-passport.passport-id.v1
rustyonions.native-passport.device-id.v1
rustyonions.native-passport.device-authorization.v1
rustyonions.native-passport.device-revocation.v1
rustyonions.native-passport.root-proof.v1
rustyonions.native-passport.device-proof.v1
rustyonions.native-passport.request-proof.v1
rustyonions.native-passport.username-commit.v1
rustyonions.native-passport.username-transition.v1
rustyonions.native-passport.site-transition.v1
rustyonions.native-passport.operational-vault.v1
rustyonions.native-passport.root-vault.v1
rustyonions.native-passport.backup.v1
```

A released domain string cannot change silently.

## 7.3 Passport ID

Recommended V1 text form:

```text
passport:v1:main:ed25519:b3:<64 lowercase hex>
```

Digest input is structured and length-prefixed:

```text
domain
version
Passport kind
root algorithm ID
root public key bytes
```

`@username` and display name are never included.

## 7.4 Device ID

Recommended V1 text form:

```text
device:v1:ed25519:b3:<64 lowercase hex>
```

Digest input:

```text
domain
version
device algorithm ID
device public key bytes
```

## 7.5 Main and alt posture

Main Passport acceptance comes first.

Future alt identities require separate domain-separated root children and must not reveal a public main→alt relationship.

Alt work must not delay:

```text
main create
main restore
device authorization
proof
capability
@username claim
```

## 7.6 Binary/text encoding

Phase 0 selects one canonical encoding for all V1 DTO fields.

Recommended:

```text
public IDs: structured lowercase text forms
binary fields in JSON: base64url without padding
canonical transcripts: raw length-prefixed bytes, never JSON bytes
integer times: unsigned milliseconds, no floats
```

Legacy token encodings remain explicitly versioned compatibility paths.

## 7.7 Golden vectors

Required vectors include:

```text
mnemonic → root seed
root seed → root public key
root public key → Passport ID
device key → Device ID
DeviceAuthorization transcript/signature
DeviceRevocation transcript/signature
root registration proof
device challenge proof
request proof
username commit/reveal/transition
site transition
```

Fixtures must be language-neutral and consumed by all platforms.

---

# 8. Two-Compartment Vault Architecture

## 8.1 Root compartment

Contains rare-use authority:

```text
mnemonic entropy or recoverable root material
root private key
root public metadata
root-key epoch
root administration metadata
authorized-device inventory cache
```

Opened only for:

```text
backup confirmation
new-device authorization
device revocation
high-risk username transfer/release
root administration
recovery/migration
```

It must not unlock automatically merely because the user opens CrabLink.

## 8.2 Operational compartment

Contains routine device authority:

```text
device private key
DeviceAuthorizationV1
Passport ID/root public key
short-lived encrypted capability material
local device metadata
non-secret redacted account metadata
```

Ordinary app unlock opens this compartment only.

## 8.3 Why compartments are separate

```text
routine compromise should not expose recovery root
TV should not possess recovery root
normal proof should not decrypt mnemonic/root key
root operations should require explicit native confirmation
```

## 8.4 Vault master keys

Each compartment uses an independent random Vault Master Key:

```text
Root VMK
Operational VMK
```

Compartment content is encrypted using XChaCha20-Poly1305 with a fresh nonce per write.

## 8.5 PIN and device-bound wrapping

Recommended construction:

```text
PIN/passcode
→ Argon2id(PIN, per-vault salt, authenticated parameters)
→ PIN key

random device factor
→ sealed/unsealed by platform non-exportable key

HKDF(PIN key, device factor, compartment/domain)
→ Key Encryption Key

Key Encryption Key
→ wraps compartment VMK
```

Changing a PIN rewraps the VMK instead of re-encrypting all secret content.

A copied vault file without the platform-protected device factor should not support ordinary offline PIN guessing.

## 8.6 Operation-based platform sealer

Do not define a trait that exports a reusable raw platform secret.

Preferred trait responsibilities:

```text
create_platform_key
seal_device_factor
unseal_device_factor
delete_platform_key
platform_key_status
supports_user_presence
supports_non_exportable_key
```

Possible conceptual trait:

```rust
pub trait PlatformSealer {
    fn create_key(&self, key_id: &PlatformKeyId) -> Result<PlatformKeyInfo, PlatformSealError>;
    fn seal(&self, key_id: &PlatformKeyId, plaintext: SecretBytes<'_>) -> Result<SealedBlob, PlatformSealError>;
    fn unseal(&self, key_id: &PlatformKeyId, blob: &SealedBlob) -> Result<SecretVec, PlatformSealError>;
    fn delete_key(&self, key_id: &PlatformKeyId) -> Result<(), PlatformSealError>;
    fn status(&self, key_id: &PlatformKeyId) -> Result<PlatformKeyStatus, PlatformSealError>;
}
```

Exact API is frozen in Phase 1.

## 8.7 Vault header

Authenticated `VaultHeaderV1` includes bounded metadata:

```text
format version
compartment kind
KDF algorithm/version/parameters
salt
AEAD algorithm
wrapped VMK metadata
platform key identifier/reference
generation counter
created/updated timestamps
payload length
```

It contains no plaintext secret.

## 8.8 Atomic persistence

Required write sequence:

```text
serialize bounded plaintext in memory
→ encrypt
→ write temporary file
→ fsync file
→ fsync parent where supported
→ atomic rename
→ update platform-protected generation marker where available
→ remove stale temporary file
```

No plaintext temporary file is permitted.

## 8.9 Rollback posture

Use:

```text
authenticated monotonic generation
platform-protected current-generation marker where feasible
explicit rollback detection error
safe recovery flow
```

Do not claim universal anti-rollback guarantees where the platform cannot provide protected monotonic state.

## 8.10 Backup behavior

```text
exclude vault files from cloud/device backup by default
or
restored vault detects missing platform key and reports DeviceBindingUnavailable
```

The recovery phrase remains the canonical portable recovery mechanism.

An optional `.crabpassport` encrypted backup package comes later and uses a dedicated high-entropy backup passphrase, not the short local PIN alone.

## 8.11 Memory and auto-lock

```text
no global unlocked singleton without lifecycle controls
lock on explicit request
lock after inactivity
lock on background according to platform policy
lock root compartment sooner than operational compartment
zeroize secret buffers where feasible
do not cache PIN
```

---

# 9. Native Secure UX Boundary

## 9.1 Early requirement

Native secure input is an architectural prerequisite, not final polish.

Calling Rust after collecting the recovery phrase in a WebView still exposes the phrase to JavaScript memory. Therefore production create/import/unlock cannot depend on React text fields.

## 9.2 NativeSecretSurface contract

Required operations:

```text
show_generated_recovery_phrase
confirm_recovery_phrase
enter_recovery_phrase
enter_pin
confirm_pin
unlock_operational_compartment
unlock_root_compartment
confirm_device_authorization
confirm_device_revocation
confirm_username_transfer_or_release
confirm_backup_export
```

Only non-secret results return to React:

```text
completed
cancelled
invalid_checksum
unlock_failed
locked
policy_denied
```

## 9.3 Platform implementations

```text
Android / Android TV:
  native Kotlin activity/view or reviewed native plugin

iOS:
  native Swift view controller

macOS:
  native secure window/dialog

Windows:
  native secure window/dialog

Linux:
  reviewed native secure input and keyring posture
```

## 9.4 Screen and clipboard policy

```text
screen-capture protection where supported
no automatic clipboard
manual copy requires warning and time-limited clipboard clearing where supported
no recovery phrase QR
no recovery phrase deep link
no TV recovery phrase display during ordinary use
```

## 9.5 TV posture

Private-beta TV onboarding defaults to delegated enrollment.

TV may show:

```text
public pairing code
public device label
requested read-only scopes
redacted Passport ID/@username after authorization
```

TV must not show or request the recovery phrase in the initial production path.

A later advanced restore path requires:

```text
native secure input
keyboard support
explicit shoulder-surfing warning
root-capable-device warning
separate security acceptance
```

---

# 10. Challenge, Proof, and Request Authorization Protocol

## 10.1 Registration flow

Initial Passport registration:

```text
client derives root locally
→ client generates first device key
→ root signs DeviceAuthorizationV1
→ client requests registration challenge
→ client verifies signed challenge
→ root signs registration transcript
→ client submits root proof + first DeviceAuthorizationV1
→ svc-passport verifies through ron-auth
→ svc-passport atomically registers Passport and device
```

The backend never generates the root key.

## 10.2 Routine proof flow

```text
client requests challenge for a precise purpose
→ svc-passport creates and signs one-time challenge
→ client validates challenge fields and service signature
→ unlocked authorized device signs canonical transcript
→ svc-passport verifies device authorization and proof
→ challenge is atomically consumed
→ policy decides capability scope/TTL
→ capability is issued bound to device key
```

## 10.3 PassportChallengeV1

Required fields:

```text
version
challenge_id
network_id
environment
audience
issuing_service_id
service_key_id
purpose
requested_scopes
passport_id?
device_id?
operation_body_hash?
nonce
issued_at_ms
expires_at_ms
service_signature
```

`purpose` is a closed enum:

```text
register_root
authorize_device
revoke_device
prove_session
issue_capability
refresh_capability
claim_username
transfer_username
release_username
publish_profile
register_site
update_site
wallet_authorization_request_reserved
```

The wallet purpose remains reserved until a separate economic authorization plan explicitly enables it.

## 10.4 Client pre-sign validation

The native client rejects before signing when any of these are wrong:

```text
network
environment
audience
issuing service identity
service challenge signature
purpose
requested scopes
Passport ID
device ID
operation body hash
expiry
protocol version
algorithm
```

The signing UI must accurately describe the operation.

## 10.5 Canonical transcript

Never sign ordinary JSON bytes.

Use deterministic length-prefixed binary transcripts with:

```text
domain tag
version
network/environment/audience
purpose
Passport ID
root/device IDs and keys as applicable
challenge ID
nonce
sorted scopes
operation body hash presence/value
issued/expires fixed-endian integers
```

Every field mutation must fail verification.

## 10.6 Replay store

Challenge states:

```text
issued
consumed
expired
cancelled/revoked
```

Proof acceptance is atomic:

```text
load issued challenge
→ validate proof
→ compare all bound fields
→ consume once
→ issue result/capability
→ append redacted audit event
```

Concurrent duplicate submissions accept at most one.

Production replay state must survive restart. Memory-only replay state is allowed only for explicit development/amnesia profiles with correspondingly constrained capabilities.

## 10.7 Device authorization review

Routine proof verification checks:

```text
Passport root record active
DeviceAuthorizationV1 signature valid
device authorization belongs to Passport
root epoch matches
device not revoked
device class allowed for purpose
device scope ceiling includes requested scopes
device authorization expiry valid
```

## 10.8 Device-bound capability

Capability contains or cryptographically commits to:

```text
Passport subject
device-key thumbprint / Device ID
audience
environment
scope list
issued/expiry
capability ID
policy version
optional root epoch
```

The capability is not useful without the device private key for protected operations.

## 10.9 PassportRequestProofV1

For protected requests:

```text
version
capability_hash or capability_id
request_method
canonical_path
canonical_query_hash
body_hash
timestamp_ms
server/request nonce
device_id
device_signature
```

Verification rejects:

```text
wrong device
wrong method/path/query/body
stale timestamp
replayed nonce
revoked capability
revoked device
scope mismatch
audience/environment mismatch
```

## 10.10 Bearer-only exception

A narrowly scoped private-beta read-only capability may temporarily be bearer-only only when explicitly marked:

```text
capability_binding = bearer_private_beta
```

It is forbidden for:

```text
profile mutation
username claim/transfer/release
publishing
wallet/economic authorization
operator controls
device administration
```

The target production posture is device-bound proof of possession.

---
# 11. Capability Lifecycle and Policy

## 11.1 Capability properties

Capabilities must be:

```text
narrow
short-lived
audience-bound
environment-bound
scope-bounded
device-bound for sensitive operations
revocable
refreshable only under current policy
redacted in logs/UI
```

## 11.2 Initial scope sets

### TV read-only

```text
identity.read
catalog.read
content.read
entitlement.read
receipts.read
confirmed_roc.read
capability.revoke_self
```

### Desktop/mobile initial identity

```text
identity.read
identity.profile.read
identity.profile.update
identity.device.list
catalog.read
content.read
entitlement.read
receipts.read
confirmed_roc.read
capability.revoke_self
```

Username and device-administration scopes require fresh root/admin proof:

```text
identity.device.authorize
identity.device.revoke
identity.username.claim
identity.username.transfer
identity.username.release
```

Explicitly absent until separate phases authorize them:

```text
wallet.spend
wallet.transfer
ledger.write
reward.issue
node.control
operator.admin
capability.delegate_unbounded
bridge.*
staking.*
```

## 11.3 Scope ceilings

Effective scopes are the intersection of:

```text
device authorization ceiling
requested scopes
platform/device-class policy
account/Passport state
route/purpose policy
server configuration
```

The client cannot widen them.

## 11.4 Refresh

Refresh requires:

```text
unexpired or policy-approved grace posture
active Passport root
active device authorization
non-revoked capability
fresh device request proof
current scope policy
```

Refresh may only preserve or narrow scopes unless a new challenge and explicit approval authorize widening.

## 11.5 Revocation

Support:

```text
self capability revoke
all capabilities for device revoke
all capabilities for Passport emergency revoke
policy-driven revoke
root-epoch invalidate reserved
```

Revocation state must be durable for production-sensitive capabilities.

## 11.6 Capability storage

Native storage rules:

```text
operational vault compartment only
encrypted at rest
never browser storage
never URL
never ordinary logs
never exported to React as raw token
```

React receives only a summary:

```text
active/inactive
scope names
expiry time
device class
refresh available
```

---

# 12. Passport Registry and Device Registry

## 12.1 Passport record

Authoritative server-side public record:

```text
PassportRecordV1 {
  passport_id,
  kind,
  root_algorithm,
  root_public_key,
  root_key_epoch,
  status,
  registered_at_ms,
  updated_at_ms,
  primary_username?,
  public_profile_manifest_cid?,
  transition_sequence
}
```

No private client material is stored.

## 12.2 Passport states

```text
active
restricted
revoked
rotation_pending_reserved
```

Private beta primarily uses `active`, with explicit policy-denied/restricted posture as needed.

## 12.3 Device record

```text
DeviceRecordV1 {
  passport_id,
  device_id,
  device_public_key,
  device_class,
  authorized_scope_ceiling,
  root_key_epoch,
  authorization_hash,
  status,
  issued_at_ms,
  expires_at_ms?,
  revoked_at_ms?,
  transition_sequence
}
```

Device labels shown to users must be locally chosen or explicitly public/redacted. Do not automatically publish hardware serials, OS account names, or personally identifying device names.

## 12.4 Persistence

Private-beta production posture requires:

```text
durable transactional Passport registration
durable device authorization/revocation
durable challenge/replay state
durable capability revocation
durable username transition log
restart tests
idempotency keys
```

In-memory stores remain test/dev only.

## 12.5 Enumeration/privacy

Public lookup should be designed around known IDs/handles, not unrestricted bulk enumeration.

Policy may expose:

```text
lookup Passport by exact Passport ID
lookup profile by exact confirmed @username
verify exact device authorization hash where needed
```

Policy should avoid:

```text
public full Passport directory dump
public full device inventory
device IP or residential network metadata
private main/alt relationships
```

---

# 13. `@username` Registry and User Identity Namespace

## 13.1 Terminology correction

This plan uses `@username`, `username`, or `handle`.

It does not use actual human names as identifiers.

Optional real/legal names may exist only inside user-controlled public profile content and have no uniqueness, proof, recovery, or ownership role.

## 13.2 `@username` relationship

```text
Passport ID:
  cryptographic source of identity

@username:
  optional unique public network pointer to Passport ID

Display name:
  optional non-unique profile text
```

A Passport may exist without an `@username`.

A user may change display name without changing `@username` or Passport ID.

## 13.3 Canonical parser

Use `ron_naming::RonUsername` or its canonical successor.

No duplicate constants or parser in:

```text
svc-passport native
svc-passport server
CrabLink JS
Android Kotlin
iOS Swift
```

Frontend validation may provide a user hint, but the Rust canonical parser decides validity.

## 13.4 Initial ownership rule

Recommended V1:

```text
one primary active @username per main Passport
one active owner per @username
ownership changes only through signed finalized transition
Passport may have no username
```

Additional aliases are deferred.

## 13.5 Username states

```text
available
committed
reveal_pending
active
transfer_pending
released_cooldown
reserved
revoked_by_governance
```

Private beta may omit commit/reveal execution initially while preserving DTO/version posture.

## 13.6 Claim flow: private beta

```text
client checks availability hint
→ user creates/unlocks Passport
→ client requests claim challenge
→ service binds challenge to normalized username and Passport ID
→ authorized device/root signs claim transcript
→ policy checks reserved names/rate limits/eligibility
→ authoritative writer atomically finalizes ownership
→ audit event emitted
→ svc-index projection updated
→ UI reports confirmed ownership
```

Availability is only a hint until finalization.

## 13.7 Claim flow: public commit-reveal

To reduce front-running:

```text
commit_hash = BLAKE3(
  domain
  || network_id
  || normalized_username
  || passport_id
  || random_claim_secret
)
```

Commit transition:

```text
commit hash
Passport ID
expiry
policy context
signed request
```

Reveal transition:

```text
normalized username
Passport ID
random claim secret
commit reference
fresh proof/signature
```

Finalization must deterministically order conflicting valid reveals.

## 13.8 Transfer

Transfer requires:

```text
current owner authorization
recipient Passport ID
fresh operation-body-bound challenge
policy checks
recipient acceptance where policy requires
atomic finalized transition
cooldown/reversal posture explicit
```

The server cannot transfer an `@username` merely because someone knows a display name or claims to have lost a recovery phrase.

## 13.9 Release

Release requires fresh root/admin authorization.

Released usernames enter a configured cooldown/tombstone period before becoming claimable again.

This reduces impersonation and accidental immediate takeover.

## 13.10 Reserved and abusive usernames

`ron-policy` owns:

```text
system/reserved list
confusable policy
abuse/report posture
impersonation review
length/rate ceilings
claim limits
```

Governance intervention must be:

```text
explicit
signed/audited
reason-coded
appealable where policy supports it
not silently rewritten
```

## 13.11 Anti-Sybil decision

“One username per Passport” does not prevent squatting because Passports are free to generate.

Before public registration, the project must explicitly select one or more:

```text
invite/allowlist for private beta
per-device/network rate limits
proof-of-work or delay
refundable ROC bond
modest ROC claim fee
reputation/age gate
human review for reserved/high-risk names
```

No crate silently invents the economic rule.

If a fee or bond is used:

```text
svc-wallet requests mutation
ron-ledger records economic truth
svc-passport consumes only verified receipt/status
namespace finalization remains separate from ron-ledger
```

## 13.12 Private-beta finalization

Initial honest posture:

```text
single authoritative svc-passport namespace writer
durable append-only transition log
transactional uniqueness
signed requests
idempotency
restart persistence
redacted audit
svc-index read projection
```

This is acceptable for private beta if it is labeled centralized finalization.

## 13.13 Decentralized production finalization

Target posture:

```text
signed namespace transitions
→ deterministic service-node quorum ordering
→ finalized namespace checkpoint
→ user-node replay verification/challenge
→ svc-index projection
→ optional DHT distribution of finalized records
```

No single service node may invent final ownership in the decentralized profile.

## 13.14 DHT limitation

The DHT may distribute:

```text
finalized username records
checkpoint hashes
profile/provider locations
```

It must not decide:

```text
first valid owner
conflicting claim order
transfer finality
release finality
```

Eventual discovery is not a uniqueness algorithm.

---

# 14. Site Namespace and Public Content

## 14.1 Site identifiers

Site names are user-selected CrabLink network identifiers, not actual human names.

Example:

```text
crab://alice-art
```

## 14.2 Site record

Recommended record:

```text
SiteNameRecordV1 {
  site_name,
  owner_passport_id,
  current_manifest_cid,
  sequence,
  status,
  updated_at_ms,
  authorization_hash,
  finalized_checkpoint?
}
```

## 14.3 Immutable content, mutable pointer

```text
site content/manifest
→ immutable b3:<digest> object in svc-storage

site name record
→ signed mutable pointer to current manifest CID
```

Updating a site:

```text
create new immutable manifest
→ obtain fresh purpose/body-bound challenge
→ authorized device signs site transition
→ policy verifies owner/scope
→ namespace transition finalizes
→ svc-index projection updates
```

## 14.4 Ownership

Site ownership is a Passport ID, not an `@username` string. The UI may display the owner's current `@username`, but changing the username does not silently change site ownership.

## 14.5 Site transfer

Future site transfer requires an explicit signed transition and recipient acceptance where policy requires. It must not be inferred from profile edits.

## 14.6 Namespace persistence owner gate

Phase 0 must choose and document one authoritative implementation owner for site transitions:

```text
Option A:
  transactional namespace module in svc-index

Option B:
  namespace transition owner in svc-passport with svc-index projection
```

Selection criteria:

```text
existing crate contract
transactional uniqueness
quorum/checkpoint reuse
no duplicate state machines
clear audit ownership
```

Until frozen, no site namespace mutation code is added.

---

# 15. Public Profile and Data Model

## 15.1 Public profile manifest

Public profile content is stored as a BLAKE3-addressed manifest:

```text
ProfileManifestV1 {
  passport_id,
  confirmed_username?,
  display_name?,
  bio?,
  avatar_cid?,
  creator_links?,
  published_at_ms,
  sequence,
  device/request authorization reference
}
```

`display_name` is optional non-authoritative content.

## 15.2 Pointer ownership

`svc-passport` stores/validates the authoritative profile pointer or transition reference. `svc-storage` stores immutable manifests. `svc-index` hydrates exact handle lookups.

## 15.3 Privacy

Public profile data is opt-in.

Do not automatically publish:

```text
email
phone
legal name
birth date
home address
residential IP
device list
recovery hints
main/alt relationship
```

---

# 16. Package Feature Model and Stable Native API

## 16.1 Target feature posture

Recommended final Cargo posture:

```toml
[features]
default = ["server", "with-metrics"]

native-client = [
  "dep:bip39",
  "dep:argon2",
  "dep:hkdf",
  "dep:sha2",
  "dep:chacha20poly1305",
  "dep:secrecy",
  "dep:zeroize",
  "dep:ed25519-dalek",
  "dep:rand_core",
  "dep:getrandom",
  "dep:ron-proto",
  "dep:ron-auth",
]

server = [
  "dep:tokio",
  "dep:axum",
  "dep:tower",
  "dep:tower-http",
  "dep:http",
  "dep:hyper",
  "dep:hyper-util",
  "dep:prometheus",
  "dep:tracing",
  "dep:tracing-subscriber",
  "dep:ron-kernel",
  "dep:ron-policy",
]

with-metrics = ["server"]
dev-kms = ["server", "dep:ed25519-dalek", "dep:rand_core"]
bus-rpc = ["server"]
dalek-batch = ["server"]
parallel-batch = ["server"]
```

`dev-kms` must not be a production default.

Binary target:

```toml
required-features = ["server"]
```

## 16.2 Build guarantees

```text
cargo check -p svc-passport
  normal service

cargo check -p svc-passport --no-default-features --features native-client
  portable client without HTTP/server runtime

cargo check -p svc-passport --all-features
  source compatibility
```

Native-client dependency tree must exclude:

```text
Axum
Hyper
Tower
service bootstrap
HTTP router
Prometheus server
service KMS rotation tasks
```

## 16.3 Module layout

Recommended:

```text
crates/svc-passport/src/native/
├── mod.rs
├── error.rs
├── mnemonic.rs
├── derivation.rs
├── identity.rs
├── device.rs
├── authorization.rs
├── secret.rs
├── pin.rs
├── vault.rs
├── platform.rs
├── proof.rs
├── request_proof.rs
├── status.rs
└── backup.rs
```

Service additions:

```text
src/registry/passport.rs
src/registry/device.rs
src/proof/challenge.rs
src/proof/store.rs
src/capability/device_binding.rs
src/namespace/username.rs
src/namespace/store.rs
```

Exact layout follows current source conventions; do not refactor unrelated modules merely to match this tree.

## 16.4 Stable native API

Candidate public task-oriented types:

```text
PassportIdV1
DeviceIdV1
RecoveryPhrase
RecoveryPhraseStrength
LockedOperationalVault
LockedRootVault
UnlockedOperationalPassport
UnlockedRootAuthority
VaultHeaderV1
DeviceAuthorizationV1
NativePassportStatus
PlatformSealer
VaultStore
NativeSecretSurfacePort
```

Candidate operations:

```text
generate_recovery_phrase
validate_recovery_phrase
create_root_identity
generate_device_key
authorize_device
create_operational_vault
create_root_vault
unlock_operational_vault
unlock_root_vault
lock_vaults
restore_root_on_new_device
sign_registration_challenge
sign_device_challenge
sign_request_proof
redacted_status
```

## 16.5 Secret type rules

```text
no secret-bearing Debug
no secret-bearing Display
no Serialize for mnemonic/PIN/private keys
no Clone for unlocked root authority where avoidable
zeroize on drop
bounded inputs
redacted error taxonomy
```

## 16.6 Cross-repository dependency

Development may use a sibling path dependency:

```toml
svc-passport = {
  path = "../../../../RustyOnions/crates/svc-passport",
  default-features = false,
  features = ["native-client"]
}
```

Release uses:

```text
pinned git revision
or internal Cargo registry package
or exact release-vendored source artifact
```

No floating branch and no copied modules.

---

# 17. Public and Internal Route Plan

## 17.1 svc-passport routes

Proposed internal/service routes:

```text
POST /v1/passport/register/challenge
POST /v1/passport/register
POST /v1/passport/challenge
POST /v1/passport/prove
POST /v1/passport/device/authorize
POST /v1/passport/device/revoke
GET  /v1/passport/device/list
POST /v1/passport/capability/refresh
POST /v1/passport/capability/revoke
GET  /v1/passport/status/:passport_id
POST /v1/passport/username/commit
POST /v1/passport/username/reveal
POST /v1/passport/username/claim
POST /v1/passport/username/transfer
POST /v1/passport/username/release
GET  /v1/passport/profile/:username
```

Private-beta claim may use direct claim while commit/reveal routes remain disabled until the public phase.

## 17.2 Omnigate routes

```text
POST /v1/identity/passport/register/challenge
POST /v1/identity/passport/register
POST /v1/identity/passport/challenge
POST /v1/identity/passport/prove
POST /v1/identity/passport/device/authorize
POST /v1/identity/passport/device/revoke
GET  /v1/identity/passport/device/list
POST /v1/identity/passport/capability/refresh
POST /v1/identity/passport/capability/revoke
GET  /v1/identity/passport/status
POST /v1/identity/passport/username/claim
POST /v1/identity/passport/username/transfer
POST /v1/identity/passport/username/release
```

Omnigate applies policy context and returns redacted DTOs. It does not sign as the client or finalize arbitrary identity.

## 17.3 Gateway public routes

```text
POST /identity/passport/register/challenge
POST /identity/passport/register
POST /identity/passport/challenge
POST /identity/passport/prove
POST /identity/passport/device/authorize
POST /identity/passport/device/revoke
GET  /identity/passport/device/list
POST /identity/passport/capability/refresh
POST /identity/passport/capability/revoke
GET  /identity/passport/status
POST /identity/passport/username/claim
POST /identity/passport/username/transfer
POST /identity/passport/username/release
```

Gateway enforces:

```text
body caps
rate limits
timeouts
route allowlist
header filtering
structured errors
no raw body logging
```

## 17.4 Legacy route classification

Phase 0 classifies existing issue/profile/debug/admin routes as:

```text
production protected
trusted-service only
loopback-only
development feature only
removed
```

Arbitrary public caller-supplied `sub` issuance is forbidden.

---
# 18. Build Discipline

## 18.1 Working style

Use the established QuickChain-style implementation balance:

```text
real Rust behavior
focused tests beside behavior
compile/test immediately
fix first failure before expanding
small behavior patches
no placeholder-only scaffold theater
no broad unrelated refactor
```

## 18.2 Canon discipline

```text
No new crate.
No duplicate Passport implementation.
No duplicate @username parser.
No second vault format.
No platform-specific derivation algorithm.
No copied root private key as ordinary device key.
No bearer-only sensitive capability.
```

## 18.3 Repository order

```text
A. source/authority reconciliation
B. svc-passport feature isolation and KMS injection
C. ron-proto canonical IDs/DTOs
D. ron-auth canonical transcripts/verifiers
E. svc-passport native root/device/vault behavior
F. svc-passport server registry/challenge/capability
G. ron-policy device/namespace rules
H. durable @username and site namespace
I. gateway/omnigate exposure
J. desktop integration
K. TV delegated integration
L. Android integration
M. iOS integration
N. decentralized namespace model
O. hardening/final acceptance
```

## 18.4 Baseline commands

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo check --workspace
```

Focused current owners:

```bash
cargo check -p svc-passport
cargo test -p svc-passport
cargo clippy -p svc-passport --all-targets --no-deps -- -D warnings

cargo check -p ron-proto
cargo test -p ron-proto
cargo check -p ron-auth
cargo test -p ron-auth
cargo check -p ron-kms
cargo test -p ron-kms
cargo check -p ron-naming
cargo test -p ron-naming
cargo check -p ron-policy
cargo test -p ron-policy
```

After feature isolation:

```bash
cargo check -p svc-passport --no-default-features --features native-client
cargo test -p svc-passport --no-default-features --features native-client
cargo check -p svc-passport --all-features
```

CrabLink begins only after the native-client feature is green:

```bash
cd /Users/mymac/Desktop/crablink
cargo test --manifest-path crates/crablink-native-core/Cargo.toml
npm --prefix apps/crablink-tauri run build
npm --prefix apps/crablink-tv run build
```

## 18.5 Focused verification rule

Ordinary patch lane:

```text
format/syntax
new focused tests
new focused boundary check
direct predecessor tests
directly affected crate/app check
```

Broad acceptance only when touching:

```text
Cargo feature graph
public IDs/DTOs
cryptographic transcripts
vault format
service routes
durable registry schema
cross-repo dependency pin
platform packaging
phase/final acceptance
```

## 18.6 Patch format

For user-executed repository patches:

```text
plain-text paste-ready Bash
no base64/gzip/opaque payload
validate before mutation
repository unchanged on abort
APPLIED banner
separate verification block
no `exit` that closes interactive shell
exact focused commands after patch
```

---

# 19. Phase Plan

## Native Passport Phase 0 — Source, Authority, and Cryptographic Spec Reconciliation

### Goal

Freeze the actual V1 authority model and reconcile current source drift before adding new secret authority.

### Work

```text
- Run RustyOnions workspace baseline and focused identity-owner checks.
- Confirm current CrabLink desktop/TV build baselines.
- Install v0.3.0 as the authoritative Native Passport plan.
- Inventory current svc-passport issue/verify/profile/debug/admin routes.
- Inventory all token encodings and canonicalize V1 encoding.
- Decide legacy capability compatibility path.
- Replace direct production DevKms construction with an injection plan.
- Classify arbitrary-subject and debug routes.
- Freeze Passport ID and Device ID text/binary formats.
- Freeze all transcript domains and byte layouts.
- Freeze BIP-39 V1 normalization/passphrase profile.
- Freeze root/device algorithms.
- Freeze site namespace persistence owner.
- Freeze private-beta username finalization posture.
- Freeze device classes and initial scope ceilings.
- Add language-neutral vector inventory files.
- Add no runtime authority beyond tests/spec fixtures.
```

Suggested fixtures:

```text
crates/svc-passport/tests/vectors/native_passport_root_v1.json
crates/svc-passport/tests/vectors/native_passport_device_v1.json
crates/svc-passport/tests/vectors/native_passport_proof_v1.json
crates/svc-passport/tests/vectors/native_passport_namespace_v1.json
```

### Exit gate

```text
Both repos green.
No new crate.
No unresolved duplicate encoding/verifier path.
Production/dev route classification recorded.
Exact public IDs and transcript bytes frozen.
Root/device hierarchy and namespace owner frozen.
No runtime secret authority added.
```

### Next

```text
NATIVE_PASSPORT_PHASE1_FEATURE_ISOLATION_AND_KMS_INJECTION
```

---

## Native Passport Phase 1 — svc-passport Feature Isolation and KMS Injection

### Owners

```text
svc-passport
ron-kms integration boundary
```

### Goal

Make `svc-passport` safely consumable as a portable native library while preserving the server and removing production dependence on development KMS defaults.

### Work

```text
- Add `server` and `native-client` feature boundaries.
- Make server-only dependencies optional.
- Make binary require `server`.
- Remove `dev-kms` from production defaults.
- Inject service KMS through bootstrap/state construction.
- Feature-gate HTTP/bootstrap/metrics/server state.
- Add `svc_passport::native` module root.
- Add real redacted ID/status/error/platform trait behavior.
- Update docs/API.MD and public API snapshot.
- Add feature/dependency boundary tests.
```

Minimum real behavior:

```text
canonical PassportIdV1 parser/formatter consumption
DeviceIdV1 parser/formatter consumption
NativePassportStatus redaction
PlatformSealer trait
VaultStore trait
typed native error taxonomy
```

### Commands

```bash
cargo check -p svc-passport
cargo test -p svc-passport
cargo check -p svc-passport --no-default-features --features native-client
cargo test -p svc-passport --no-default-features --features native-client
cargo check -p svc-passport --all-features
cargo clippy -p svc-passport --all-targets --no-deps -- -D warnings
```

### Exit gate

```text
Service behavior remains green.
Native-client compiles independently.
Native-client tree excludes HTTP/server runtime.
Production default excludes dev-kms.
KMS is injected.
Stable native surface documented/snapshot-tested.
```

### Next

```text
NATIVE_PASSPORT_PHASE2_CANONICAL_IDS_AND_NAMING
```

---

## Native Passport Phase 2 — ron-proto Canonical IDs and ron-naming Reuse

### Owners

```text
ron-proto
ron-naming
svc-passport consumers
```

### Goal

Create exactly one public ID model and one username/site-name parser.

### Work

```text
- Implement PassportIdV1 and DeviceIdV1 strict types.
- Implement algorithm/version/kind enums.
- Reuse/export canonical RonUsername and site-name types.
- Add root/device public key DTOs.
- Add DeviceClassV1.
- Add strict size/count/encoding validation.
- Add legacy subject type that cannot become production PassportIdV1.
- Delete or deprecate duplicate parser constants in svc-passport.
```

### Commands

```bash
cargo check -p ron-proto
cargo test -p ron-proto
cargo check -p ron-naming
cargo test -p ron-naming
cargo check -p svc-passport --no-default-features --features native-client
```

### Exit gate

```text
One Passport ID type.
One Device ID type.
One canonical @username parser.
Legacy dev labels cannot parse as production IDs.
No secret-bearing DTO.
```

### Next

```text
NATIVE_PASSPORT_PHASE3_ROOT_DEVICE_DTO_AND_TRANSCRIPTS
```

---

## Native Passport Phase 3 — Root/Device DTOs and Pure Authorization Verification

### Owners

```text
ron-proto
ron-auth
```

### Goal

Define and verify root registration, device authorization, and device revocation before local vault work.

### Work

```text
- Add DeviceAuthorizationV1 and DeviceRevocationV1 DTOs.
- Implement exact canonical transcripts in ron-auth.
- Implement root signature verification.
- Validate Passport ID/root key binding.
- Validate Device ID/device key binding.
- Validate root epoch, device class, scope ceiling, times, network/environment.
- Add every-field mutation rejection tests.
- Match Phase 0 vectors.
```

### Commands

```bash
cargo check -p ron-auth
cargo test -p ron-auth --test native_passport_device_vectors
cargo test -p ron-auth --test native_passport_device_rejections
```

### Exit gate

```text
Golden device authorization and revocation verify.
Every bound-field mutation rejects.
Verification is pure and IO-free.
```

### Next

```text
NATIVE_PASSPORT_PHASE4_NATIVE_ROOT_AND_DEVICE_KEYS
```

---

## Native Passport Phase 4 — Native Recovery Root and Device-Key Foundation

### Owners

```text
svc-passport::native
ron-auth vector consumer
```

### Goal

Implement deterministic recovery root and random per-device keys with secret-safe wrappers.

### Work

```text
- Generate 12/24-word recovery phrases from OS CSPRNG.
- Validate checksum and normalization.
- Derive V1 root key and Passport ID.
- Generate random device key and Device ID.
- Sign DeviceAuthorizationV1 with root.
- Sign DeviceRevocationV1 with root.
- Add zeroizing root/device secret wrappers.
- Add redacted public metadata.
- Reserve alt derivation domains without implementing alt product UX.
```

### Commands

```bash
cargo test -p svc-passport --no-default-features --features native-client --test native_root_vectors
cargo test -p svc-passport --no-default-features --features native-client --test native_device_vectors
cargo test -p svc-passport --no-default-features --features native-client --test native_secret_boundary
```

### Exit gate

```text
Same phrase = same root and Passport ID.
Different devices = different device keys/IDs.
Root authorization verifies through ron-auth.
No secret leaks through Debug/Display/Serialize/errors.
```

### Next

```text
NATIVE_PASSPORT_PHASE5_PLATFORM_SEALER_AND_SECURE_SURFACE_PORTS
```

---

## Native Passport Phase 5 — Platform Sealer and Native Secure Surface Contracts

### Owners

```text
svc-passport::native traits
packages/crablink-platform contracts
platform test adapters
```

### Goal

Freeze operation-based platform protection and native secret-entry boundaries before vault implementation.

### Work

```text
- Implement PlatformSealer trait and typed posture reporting.
- Implement VaultStore atomic persistence trait.
- Define NativeSecretSurfacePort result contract.
- Add in-memory/test sealer that does not enter production features.
- Add unsupported/degraded platform errors.
- Add backup-exclusion and missing-platform-key posture.
- Prove no raw platform secret API exists.
```

### Exit gate

```text
Portable core requests seal/unseal operations only.
No exported device-secret trait.
Native secret UI contract exists before React integration.
Test adapters prove lifecycle/error behavior.
```

### Next

```text
NATIVE_PASSPORT_PHASE6_TWO_COMPARTMENT_VAULT
```

---

## Native Passport Phase 6 — Two-Compartment Vault and PIN Unlock

### Owners

```text
svc-passport::native
```

### Goal

Implement independent operational and root vault compartments with PIN + platform-bound wrapping.

### Work

```text
- Define authenticated VaultHeaderV1.
- Implement Argon2id parameter floors/calibration bounds.
- Implement device-factor sealing/unsealing.
- Implement independent Root VMK and Operational VMK.
- Implement XChaCha20-Poly1305 seal/unseal.
- Implement PIN change by VMK rewrap.
- Implement bounded parsing and generic unlock failure.
- Implement atomic writes and generation metadata.
- Implement root vs operational lock state and timeouts.
- Add wrong PIN, wrong platform key, corruption, truncation, rollback tests.
```

### Commands

```bash
cargo test -p svc-passport --no-default-features --features native-client --test native_vault_roundtrip
cargo test -p svc-passport --no-default-features --features native-client --test native_vault_corruption
cargo test -p svc-passport --no-default-features --features native-client --test native_vault_rollback
cargo test -p svc-passport --no-default-features --features native-client --test native_wrong_pin
```

### Exit gate

```text
Ordinary unlock opens operational compartment only.
Root requires separate explicit unlock.
Correct PIN + platform binding unlocks.
Wrong PIN/platform key/corruption fails.
Vault bytes contain no plaintext phrase/private key.
Restart works.
```

### Next

```text
NATIVE_PASSPORT_PHASE7_RESTORE_AND_DELEGATED_ENROLLMENT
```

---

## Native Passport Phase 7 — Restore, New Device, and Delegated Enrollment

### Owners

```text
svc-passport::native
Rust harness
```

### Goal

Prove same-Passport restore with new device keys and delegated TV-style enrollment without root transfer.

### Work

```text
- Device A create with root + device A key.
- Restart/unlock device A.
- Restore phrase on device B with new PIN and device B key.
- Prove same Passport ID and different Device IDs.
- Prove independent vault salts/ciphertexts/platform bindings.
- Implement public pairing request DTO.
- Authorize delegated device C without sending root material.
- Verify device C cannot open root compartment or authorize another device.
- Add clear/reinstall/missing platform key behavior.
```

### Exit gate

```text
Phrase restores same Passport, not same device key.
Delegated device works without root material.
Copied vault does not unlock on another device.
Different PINs preserve same Passport ID.
```

### Next

```text
NATIVE_PASSPORT_PHASE8_CHALLENGE_AND_PROOF_DTOS
```

---

## Native Passport Phase 8 — Purpose-Bound Challenge and Proof Foundation

### Owners

```text
ron-proto
ron-auth
svc-passport native consumer
```

### Goal

Implement signed challenge DTOs, canonical transcripts, and pure root/device proof verification.

### Work

```text
- Add PassportChallengeV1 and purpose enum.
- Add operation_body_hash and service signature.
- Add root registration proof and device proof DTOs.
- Implement service challenge verification.
- Implement canonical proof transcripts.
- Implement audience/environment/network/purpose/body/scope/time validation.
- Add every-field mutation and unknown-field rejection tests.
```

### Exit gate

```text
Client can verify challenge before signing.
Root/device proof vectors match.
Wrong purpose/body/audience/environment rejects.
No JSON-byte signing.
```

### Next

```text
NATIVE_PASSPORT_PHASE9_NATIVE_PROOF_SIGNING
```

---

## Native Passport Phase 9 — Native Root/Device Proof and Request-Proof Signing

### Owners

```text
svc-passport::native
ron-auth transcript builders
```

### Goal

Sign only validated operations with the correct unlocked authority.

### Work

```text
- Root signs registration/device-admin operations only.
- Device signs routine challenge proofs.
- Device signs PassportRequestProofV1.
- Enforce locked-state denial.
- Enforce purpose-to-authority mapping.
- Return proof only to native transport adapter.
- Add golden signature tests and secret-boundary tests.
```

### Exit gate

```text
Operational unlock cannot perform root-only action.
Root unlock is explicit.
Locked vault cannot sign.
Wrong challenge rejects locally.
Proof/request-proof contain no secret key material.
```

### Next

```text
NATIVE_PASSPORT_PHASE10_DURABLE_REGISTRY_AND_REPLAY
```

---

## Native Passport Phase 10 — Durable Passport/Device Registry and Challenge Replay

### Owners

```text
svc-passport server
ron-auth
ron-audit
```

### Goal

Create durable client-rooted registration, device state, and atomic one-time challenge behavior.

### Work

```text
- Add Passport and Device persistence traits/implementation.
- Register Passport only after root proof.
- Register first/subsequent device authorization idempotently.
- Persist device revocation.
- Add signed one-time challenge issuance.
- Add issued/consumed/expired/cancelled state.
- Add atomic consume under concurrent submissions.
- Add restart persistence tests.
- Emit redacted audit evidence.
- Remove production arbitrary-subject registration.
```

### Exit gate

```text
Backend identity originates from client root proof.
Device authorization/revocation persists across restart.
Replay rejects.
Concurrent duplicate accepts at most one.
Backend cannot recreate private keys.
```

### Next

```text
NATIVE_PASSPORT_PHASE11_DEVICE_BOUND_CAPABILITY
```

---

## Native Passport Phase 11 — Device-Bound Capability and Request Proof

### Owners

```text
ron-auth
ron-policy
svc-passport
ron-kms
```

### Goal

Issue narrow capabilities bound to authorized devices and verify fresh request proof.

### Work

```text
- Add device thumbprint/Device ID caveat.
- Add policy device-class scope ceilings.
- Add capability TTL/audience/environment policy.
- Add request-proof nonce/timestamp store.
- Verify method/path/query/body binding.
- Add refresh and self-revoke.
- Revoke all device capabilities upon device revocation.
- Add redacted capability summary.
- Preserve trusted service capability paths explicitly/versioned.
```

### Exit gate

```text
Stolen capability without device key fails sensitive request.
TV cannot request mutating scopes.
Scope escalation/TTL excess rejects.
Device/capability revocation rejects.
Raw capability never reaches React.
```

### Next

```text
NATIVE_PASSPORT_PHASE12_USERNAME_PRIVATE_BETA_REGISTRY
```

---

## Native Passport Phase 12 — Private-Beta `@username` Registry

### Owners

```text
ron-naming
ron-proto
ron-auth
ron-policy
svc-passport
svc-index projection
ron-audit
```

### Goal

Implement honest durable private-beta username uniqueness bound to proven Passport identity.

### Work

```text
- Use canonical RonUsername.
- Add signed UsernameTransitionV1.
- Add direct private-beta claim challenge/flow.
- Enforce one primary username per main Passport.
- Add reserved-name and rate-limit policy.
- Add transactional uniqueness and idempotency.
- Add transfer and release with root/admin fresh proof.
- Add release cooldown/tombstone.
- Update svc-index exact lookup projection.
- Add restart and conflicting-concurrency tests.
- Label finalization as single-writer/private-beta.
```

### Exit gate

```text
@username is bound to Passport ID, not display/real name.
Concurrent claim has one winner.
Restart preserves ownership.
Transfer/release require current owner proof.
Index is projection, not authority.
```

### Next

```text
NATIVE_PASSPORT_PHASE13_PROFILE_AND_SITE_NAMESPACE
```

---

## Native Passport Phase 13 — Public Profile and Site Namespace

### Owners

```text
svc-passport
svc-storage
svc-index
ron-naming
ron-policy
omnigate consumers
```

### Goal

Bind public profile and site pointers to authorized Passport actions while preserving immutable content.

### Work

```text
- Store ProfileManifestV1 in svc-storage.
- Persist authoritative profile manifest pointer/sequence.
- Add optional non-authoritative display_name.
- Implement chosen site namespace transition owner.
- Store immutable site manifests in svc-storage.
- Update signed mutable site pointer transactionally.
- Add owner, sequence, body-hash, replay, transfer-reserved tests.
```

### Exit gate

```text
Profile/site content is BLAKE3-addressed.
Profile display name carries no authority.
Site owner is Passport ID.
Pointer update requires authorized device proof.
No economic ledger misuse.
```

### Next

```text
NATIVE_PASSPORT_PHASE14_GATEWAY_OMNIGATE_ROUTES
```

---

## Native Passport Phase 14 — Gateway and Omnigate Integration

### Owners

```text
svc-gateway
omnigate
svc-passport
```

### Goal

Expose fixed safe client routes without moving identity authority into ingress/orchestration.

### Work

```text
- Add register/challenge/prove/device/capability/status routes.
- Add username claim/transfer/release routes.
- Add fixed downstream clients.
- Add body caps, rate limits, deadlines, correlation IDs.
- Add typed redacted problem envelopes.
- Disable raw request/response body logging.
- Add local-stack smoke and replay/revocation scenarios.
```

### Exit gate

```text
Native client registers, proves, receives device-bound read capability, uses protected route, revokes, and is denied.
Gateway remains proxy/admission only.
Omnigate remains orchestration only.
No secret echo.
```

### Next

```text
NATIVE_PASSPORT_PHASE15_DESKTOP_INTEGRATION
```

---

## Native Passport Phase 15 — CrabLink Desktop Root-Capable Integration

### Owners

```text
svc-passport native-client
packages/crablink-platform
apps/crablink-tauri/src-tauri
crablink-native-core redacted review
```

### Goal

Deliver complete desktop create/restore/unlock/device/username/proof behavior without WebView secret custody.

### Work

```text
- Pin exact svc-passport revision.
- Add macOS/Windows/Linux PlatformSealer adapters.
- Add atomic VaultStore adapter.
- Add native recovery/PIN/root confirmation surfaces.
- Add create/restore/unlock/lock/clear/status commands.
- Add device list/authorize/revoke commands.
- Add challenge/proof/request-proof transport.
- Add username availability hint/claim/status.
- Store capability in operational vault only.
- Replace dev label when real Passport active.
- Keep dev compatibility behind explicit feature.
- Return redacted DTOs to React.
```

Expected commands:

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

Forbidden commands:

```text
passport_get_seed_to_webview
passport_export_private_key
passport_get_device_private_key
passport_get_raw_capability
passport_issue_arbitrary_scope
passport_disable_policy
```

### Exit gate

```text
Desktop creates/backs up Passport.
Restart returns locked state.
Operational PIN unlock works without root unlock.
Root operations require separate native confirmation.
Proof/capability/username claim work.
React never receives secrets.
```

### Next

```text
NATIVE_PASSPORT_PHASE16_TV_DELEGATED_INTEGRATION
```

---

## Native Passport Phase 16 — Android TV Delegated Integration

### Owners

```text
svc-passport native-client
apps/crablink-tv
Android Keystore adapter
TV native pairing/PIN UI
```

### Goal

Authorize TV as a revocable read-only delegated device without copying the root.

### Work

```text
- Consume exact pinned svc-passport revision.
- Repair/consume authoritative TV Passport successor marker.
- Add Android Keystore PlatformSealer.
- Generate TV device key locally.
- Add public pairing request and short verification code.
- Authorize from desktop/mobile root-admin device.
- Store only TV device key/authorization/capability.
- Add native TV PIN prompt.
- Add redacted Passport/@username screen.
- Request only TV read scopes.
- Prove background/restart/lock/revoke behavior.
```

### Exit gate

```text
TV joins Passport without recovery phrase/root key.
TV proof/capability works.
TV cannot authorize devices, claim/transfer username, publish, spend, or operate nodes.
Revoking TV immediately blocks refresh/proof-sensitive access.
```

### Next

```text
NATIVE_PASSPORT_PHASE17_ANDROID_IOS_INTEGRATION
```

---

## Native Passport Phase 17 — Regular Android and iOS Integration

### Owners

```text
svc-passport native-client
future Android app
future iOS app
Android Keystore
Apple Keychain/Secure Enclave
```

### Goal

Reuse the exact Rust identity/vault/proof state machine with native platform protection and secret UI.

### Work

```text
- Add Android and iOS PlatformSealer adapters.
- Add native recovery/PIN/root confirmation UI.
- Support root-capable restore and delegated enrollment.
- Add background auto-lock and screen-capture protections.
- Prove same vectors and Passport ID across platforms.
- Prove different device keys/vault ciphertext.
- Prove device revocation and capability binding.
```

### Exit gate

```text
No Kotlin/Swift Passport state machine exists.
Same phrase = same Passport ID.
Each platform/device = separate device key.
PIN remains device-local.
Proof/capability behavior matches.
```

### Next

```text
NATIVE_PASSPORT_PHASE18_PUBLIC_NAMESPACE_AND_HARDENING
```

---

## Native Passport Phase 18 — Public Namespace Finality, Hardening, and Final Acceptance

### Owners

```text
all touched identity/naming/ingress/platform owners
service-node quorum/checkpoint integration
user-node replay verification
```

### Goal

Harden all surfaces and move namespace claims from honest private-beta writer to reviewed decentralized finality when the network infrastructure is ready.

### Work

```text
- Implement username commit-reveal.
- Implement deterministic conflict ordering.
- Integrate signed namespace transitions with service-node quorum/checkpoints.
- Add user-node replay/challenge model.
- Publish finalized records/checkpoints for projection/discovery.
- Select and implement explicit anti-Sybil policy.
- Fuzz all parsers/transcripts/vault headers.
- Property-test restore/device/namespace invariants.
- Run concurrent proof/request/claim replay tests.
- Run interrupted write/rollback/reinstall/device-loss tests.
- Run clock-skew and revocation-race tests.
- Scan logs, metrics, URLs, browser storage, crash diagnostics.
- Run dependency audit/deny/vet/SBOM posture.
- Run cross-platform live demonstrations.
```

### Final exit gate

```text
Passport root remains self-custodied.
Routine devices use separate revocable keys.
TV is delegated/read-only by default.
Proof replay and request replay reject.
Capabilities are narrow, expiring, revocable, device-bound.
@username ownership is signed, durable, and finality-labeled truthfully.
Actual human names are never identifiers or authority.
Site names point to signed mutable records and immutable manifests.
No cloud/server recovery material exists.
No React/browser secret custody exists.
No new Passport crate exists.
svc-passport remains canonical owner.
```

---
# 20. Required Test Matrix

## 20.1 Recovery root and IDs

```text
valid 12-word vector
valid 24-word vector
invalid checksum
invalid word
whitespace normalization
Unicode normalization posture
same phrase = same root/Passport ID
one-word change = different root/Passport ID
username/display/site changes do not change Passport ID
version mismatch
algorithm mismatch
main/alt domain separation reservation
```

## 20.2 Device keys and authorization

```text
random device keys differ
Device ID binding
valid root authorization
wrong root signature
wrong Passport ID
wrong device key/ID
wrong device class
scope ceiling mutation
network/environment mutation
expired authorization
root epoch mismatch
revocation happy path
revocation replay
revoked device proof rejection
```

## 20.3 Vault

```text
create root and operational compartments
save/restart/unlock
operational unlock does not unlock root
root unlock explicit
wrong PIN
wrong platform key
missing platform key
corrupt header
corrupt nonce/ciphertext
truncated/oversized file
old format migration
atomic-write interruption
rollback attempt
PIN change/VMK rewrap
clear/reinstall
auto-lock/background lock
root timeout shorter than operational timeout
```

## 20.4 Native secure boundary

Search/assert absence of:

```text
recovery phrase in React/JS
PIN in React/JS
private key in command DTO
raw capability in frontend state
secret in localStorage/sessionStorage/indexedDB
secret URL/deep link
recovery QR
automatic secret clipboard
secret Debug/Display/error/log/crash report
```

## 20.5 Challenge and proof

```text
root registration happy path
device proof happy path
wrong key
wrong Passport ID
wrong Device ID
wrong challenge/nonce
wrong network/environment/audience
wrong issuing service/signature
wrong purpose
wrong operation body hash
wrong scopes/order/canonicalization
extra/missing scope
expired/not-yet-valid/clock skew
unknown version/algorithm
oversized DTO
signature mutation
replay
concurrent replay
```

## 20.6 Request proof

```text
valid method/path/query/body
wrong method
wrong path
query normalization mismatch
wrong body hash
stale timestamp
nonce replay
wrong capability hash
wrong device
revoked device
revoked capability
scope mismatch
```

## 20.7 Capability

```text
happy path
scope denial
scope ceiling intersection
TTL ceiling
expiry
refresh
refresh with scope narrowing
refresh after device revoke
refresh after capability revoke
self revoke
other-subject revoke denial
audience/environment mismatch
bearer-only sensitive route rejection
raw token redaction
restart persistence
```

## 20.8 `@username`

```text
canonical valid username
invalid syntax/confusable/reserved
availability hint is non-final
private-beta claim happy path
same Passport second primary denial
concurrent conflicting claim one winner
idempotent retry
restart persistence
unauthorized transfer/release
recipient acceptance policy
release cooldown
tombstone lookup
commit/reveal match
front-running reveal without secret denial
commit expiry
quorum/checkpoint replay
index projection consistency
actual display/real name has no ownership effect
```

## 20.9 Site namespace

```text
valid site name
owner Passport binding
manifest CID validation
sequence monotonicity
body hash binding
unauthorized update
replay
concurrent update conflict
site transfer reserved behavior
index/storage pointer consistency
```

## 20.10 Cross-platform

```text
desktop create → desktop restore
desktop create → Android restore
desktop create → iOS restore
desktop root-admin → TV delegated enrollment
Android create → desktop restore
same phrase → same Passport ID everywhere
different device keys/IDs everywhere
different PINs → same Passport ID
different ciphertext/salts/platform bindings
same proof vectors
TV revocation blocks TV without affecting other devices
```

---

# 21. Threat Model and Required Mitigations

## 21.1 Copied vault file

Mitigation:

```text
Argon2id
platform-sealed random device factor
wrapped compartment VMKs
AEAD
bounded parsing
```

## 21.2 Weak PIN

Mitigation:

```text
6-digit minimum
recommend longer passcode
KDF calibration/floors
platform-bound factor
no claim that PIN protects a fully malware-controlled device
```

## 21.3 Compromised ordinary device

Mitigation:

```text
separate device key
root compartment not routinely open
short capabilities
device revocation
platform keystore
auto-lock
```

## 21.4 Compromised root-capable device

Mitigation:

```text
separate root compartment
explicit root unlock/confirmation
short root-unlock timeout
platform sealing
minimal root operations
future root-rotation reservation
honest warning that active malware can defeat client security
```

## 21.5 Lost TV

Mitigation:

```text
delegated read-only key
no root material
device revoke
short capability TTL
```

## 21.6 Server compromise

Mitigation:

```text
server has public keys and service signing keys only
no client recovery/root/device private keys
short capabilities
revocation
audit
KMS rotation
```

## 21.7 Malicious/fake challenge

Mitigation:

```text
service-signed challenge
verified service key/identity
network/environment/audience/purpose/body binding
native confirmation for high-risk operations
```

## 21.8 Proof replay

Mitigation:

```text
random nonce
short expiry
atomic one-time consume
restart-durable replay state
```

## 21.9 Capability theft

Mitigation:

```text
device binding
fresh request proof
method/path/query/body binding
short TTL
revocation
native encrypted storage
```

## 21.10 Username front-running

Mitigation:

```text
commit-reveal
random claim secret
deterministic finalization
signed transitions
```

## 21.11 Username squatting/Sybil

Mitigation chosen explicitly from:

```text
private-beta invitations
rate limits
claim delay
ROC fee/bond
reputation/age
reviewed governance
```

No claim that cryptography alone solves scarcity.

## 21.12 Namespace split brain

Mitigation:

```text
single transactional writer in private beta
or
quorum/checkpoint finality in decentralized profile
user-node replay/challenge
index treated as projection
DHT not used for uniqueness
```

## 21.13 Phishing for recovery phrase

Mitigation:

```text
native-only recovery UI
backend/web content never requests phrase
exact application/environment display
no browser form
no phrase QR/deep link
```

## 21.14 Shoulder surfing/screenshot

Mitigation:

```text
native secret screens
screen-capture protection where available
short reveal duration
TV delegated enrollment default
warnings
```

## 21.15 Rollback and interrupted writes

Mitigation:

```text
authenticated generation
atomic write/fsync/rename
platform marker where possible
rollback detection/recovery tests
```

## 21.16 Supply chain

Mitigation:

```text
reviewed locked Rust crates
cargo audit
cargo deny
cargo vet where available
SBOM/provenance
no homegrown crypto
```

---

# 22. Configuration, Metrics, Audit, and Error Taxonomy

## 22.1 Configuration ownership

Suggested reviewed config families:

```text
configs/passport-crypto.toml
configs/passport-policy.toml
configs/passport-namespace.toml
```

Exact paths follow repository conventions.

Config includes:

```text
network/environment IDs
challenge/capability TTL
clock skew
KDF floors/maxima
auto-lock/root timeout
platform classes
scope ceilings
reserved username policy
release cooldown
claim rate limits
anti-Sybil mode
site namespace policy
feature gates
```

Cryptographic domain strings and released transcript bytes are source/version constants, not mutable runtime config.

## 22.2 Metrics

Allowed bounded labels:

```text
operation=create|restore|unlock|authorize_device|prove|claim_username
result=ok|denied|error
reason_class=expired|replay|policy|invalid_signature|storage|platform_binding
platform_class=desktop|android|ios|tv|test
capability_binding=device|bearer_private_beta
namespace_profile=private_beta_writer|quorum_finalized
```

Forbidden metric labels:

```text
Passport ID
Device ID
@username
site name
public key
nonce
capability ID/token
IP address
PIN
```

## 22.3 Audit

Audit events use stable event IDs and redacted hashes only where necessary for correlation.

Possible events:

```text
passport_registered
device_authorized
device_revoked
proof_denied
capability_issued
capability_refreshed
capability_revoked
username_claimed
username_transferred
username_released
site_pointer_updated
policy_denied
```

No raw secrets or raw capabilities.

## 22.4 Error taxonomy

Native public errors:

```text
PassportAbsent
PassportLocked
RootAuthorityLocked
InvalidRecoveryPhrase
InvalidPassportFormat
InvalidDeviceAuthorization
UnlockFailed
DeviceBindingUnavailable
VaultCorrupt
VaultRollbackDetected
PlatformProtectionUnavailable
ChallengeRejected
ProofDenied
CapabilityDenied
UsernameUnavailable
UsernamePolicyDenied
NamespaceConflict
OperationCancelled
```

Public unlock should avoid distinguishing wrong PIN from modified ciphertext where practical.

Server problem codes are typed, stable, and redacted.

---

# 23. Dependency and Supply-Chain Posture

## 23.1 Candidate native dependencies

Subject to exact-version audit:

```text
bip39
ed25519-dalek
argon2
hkdf
sha2
chacha20poly1305
rand_core/getrandom
zeroize
secrecy
blake3
base64
serde
thiserror
subtle
ron-proto
ron-auth
```

## 23.2 Dependency rules

```text
No homegrown crypto.
No deprecated cipher mode.
No plaintext fallback.
No fixed nonce.
No fixed global salt.
No test RNG in production.
No floating git dependency.
No Tauri/Kotlin/Swift framework in portable core.
No server HTTP runtime in native-client tree.
No cyclic svc-passport↔ron-auth dependency.
```

## 23.3 Platform dependencies

Platform shells provide:

```text
Android Keystore
Apple Keychain/Secure Enclave
macOS Keychain
Windows DPAPI/CNG/Credential Manager reviewed adapter
Linux Secret Service/keyring reviewed adapter
native secure dialogs/views
```

## 23.4 Stronghold posture

Tauri Stronghold remains a candidate adapter only.

It must prove:

```text
no WebView secret requirement
all target platform support needed
platform key binding
migration/export semantics
rollback/corruption behavior
reinstall behavior
```

## 23.5 Supply-chain gates

```text
cargo audit
cargo deny
cargo vet where available
cargo tree feature review
license review
MSRV matrix
target compile matrix
SBOM/provenance
crypto dependency pin review
```

---

# 24. Migration and Compatibility

## 24.1 Legacy identity labels

Legacy/dev labels remain visible only as explicit development state.

They do not automatically migrate to a cryptographic Passport.

Migration UI may say:

```text
Development identity
Create or restore a real CrabLink Passport
```

## 24.2 Legacy profile claims

Existing username/profile prototypes must be classified:

```text
dev data discarded
or
explicitly imported through a signed ownership claim
```

No old arbitrary subject string becomes owner merely by matching text.

## 24.3 Legacy capabilities

Legacy service/trusted capabilities may remain under versioned routes and verification code while needed.

Rules:

```text
no silent reinterpretation as Native Passport V1
no public arbitrary subject issuance
clear sunset/migration notes
```

## 24.4 No private-key import in V1

Canonical recovery is mnemonic restore.

Raw private-key import/export is excluded to avoid ambiguous unversioned key material.

## 24.5 Vault migrations

Every vault format change requires:

```text
explicit version
bounded parser
migration test vectors
atomic migration
rollback/failure behavior
no destructive overwrite before successful verification
```

---

# 25. Acceptance Demonstrations

## 25.1 Root-capable two-device demonstration

```text
1. Start local RustyOnions identity stack.
2. Create Passport on device A through native/test secure surface.
3. Record recovery phrase through approved harness.
4. Choose PIN A.
5. Generate/authorize device A key.
6. Register Passport and device A.
7. Restart device A and unlock operational compartment.
8. Prove routine device challenge and receive device-bound read capability.
9. Call protected read endpoint with request proof.
10. Unlock root compartment separately.
11. Restore phrase on device B.
12. Choose PIN B different from PIN A.
13. Generate/authorize device B key.
14. Prove same Passport ID, different Device IDs and vault ciphertext.
15. Revoke device A from device B/root authority.
16. Prove device A refresh/request is denied while device B remains valid.
```

## 25.2 Delegated TV demonstration

```text
1. TV generates local device key/pairing request.
2. TV displays public short code.
3. Root-admin device reviews `tv_read_only` authorization.
4. Root signs DeviceAuthorizationV1.
5. TV receives authorization without phrase/root key.
6. TV chooses local PIN and stores operational vault.
7. TV proves identity and reads catalog/receipts/entitlements.
8. TV attempts username/publish/spend/operator scope and is denied.
9. Root-admin revokes TV.
10. TV refresh/protected request is denied.
```

## 25.3 Username demonstration

```text
1. Device checks @username availability hint.
2. Device requests purpose/body-bound claim challenge.
3. Authorized proof submits normalized username.
4. Private-beta writer finalizes claim transactionally.
5. svc-index exact lookup resolves to Passport/profile.
6. Concurrent second Passport claim loses.
7. Display-name change does not affect ownership.
8. Transfer/release without current owner root/admin proof fails.
9. Release enters cooldown.
```

## 25.4 Final labels

```text
NATIVE_PASSPORT_ROOT_CREATE=GREEN
NATIVE_PASSPORT_OPERATIONAL_PIN_UNLOCK=GREEN
NATIVE_PASSPORT_ROOT_COMPARTMENT_ISOLATION=GREEN
NATIVE_PASSPORT_RESTART_UNLOCK=GREEN
NATIVE_PASSPORT_RECOVERY_RESTORE=GREEN
NATIVE_PASSPORT_SAME_ID_DIFFERENT_DEVICE_KEYS=GREEN
NATIVE_PASSPORT_DEVICE_AUTHORIZATION=GREEN
NATIVE_PASSPORT_DEVICE_REVOCATION=GREEN
NATIVE_PASSPORT_TV_DELEGATION=GREEN
NATIVE_PASSPORT_PROOF=GREEN
NATIVE_PASSPORT_REPLAY_REJECTION=GREEN
NATIVE_PASSPORT_REQUEST_PROOF=GREEN
NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY=GREEN
NATIVE_PASSPORT_USERNAME_CLAIM=GREEN
NATIVE_PASSPORT_USERNAME_CONFLICT_REJECTION=GREEN
NATIVE_PASSPORT_SITE_POINTER_AUTHORIZATION=GREEN
NATIVE_PASSPORT_SECRET_BOUNDARY=GREEN
NATIVE_PASSPORT_CLOUD_CUSTODY=ABSENT
ACTUAL_HUMAN_NAME_IDENTIFIER_AUTHORITY=ABSENT
SVC_PASSPORT_CANONICAL_OWNER=YES
NATIVE_PASSPORT_NEW_CRATE=ABSENT
NATIVE_CLIENT_SERVER_RUNTIME=ABSENT
NATIVE_PASSPORT_BUILDPLAN=COMPLETE
```

---

# 26. Forbidden Scope

This build plan does not authorize:

```text
cloud login
cloud password account
cloud recovery phrase backup
server-held client root/device private keys
server PIN recovery
email/SMS reset that recreates Passport
actual/legal human name as Passport identifier
actual/legal human name as username ownership proof
username-derived keys
PIN-derived permanent identity
same root private key used routinely on every device
TV root custody by default
raw private-key export
recovery phrase QR
secret deep link
secret browser storage
unrestricted or long-lived bearer capability
arbitrary public caller-supplied Passport subject
wallet spend scope for TV
wallet/ledger mutation by svc-passport
starter ROC issuance
ROX/Solana/bridge/staking/liquidity/exchange runtime
node/operator authority through Passport alone
fake entitlement/receipt/balance/finality
```

Wallet and Passport may later be linked only through explicit scoped authorization and existing wallet/ledger authority boundaries.

---

# 27. Documentation Authority Order

When documents conflict:

```text
1. Current CRABLINK_TV_BUILDPLAN Passport product model
2. This NATIVE_PASSPORT_BUILDPLAN v0.3.0
3. Current repository source and latest green terminal output
4. Current focused crate/app notes
5. Existing Passport/Vault IDBs where consistent
6. Older Native Passport v0.2.0 and legacy bootstrap/pairing plans
7. Older ALLNOTES/combined documents
```

Cryptographic or namespace contract changes require:

```text
explicit version bump
migration impact
updated golden vectors
security review
matching acceptance-gate changes
```

---

# 28. Immediate Next Action

Do not begin with UI.

Do not create a new crate.

Do not add mnemonic/vault code before Phase 0 reconciliation.

First action:

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo check --workspace
```

Then:

```bash
cargo check -p svc-passport
cargo test -p svc-passport
cargo clippy -p svc-passport --all-targets --no-deps -- -D warnings
```

Phase 0A:

```text
inspect first compiler/test failure
patch exact surface
rerun focused check
inventory svc-passport route/token/KMS/profile state
install v0.3.0 plan
add no new runtime authority
```

Phase 0B:

```text
freeze Passport/Device IDs
freeze root/device/namespace transcripts
freeze challenge purpose/body binding
freeze private-beta username/site persistence owners
add vector fixtures and inventory tests
```

Phase 1 begins only after Phase 0 is green:

```text
split server/native-client features
inject service KMS
remove dev-kms from production defaults
prove native-client excludes server runtime
```

---

# 29. Reference Basis

Project references used for this revision:

```text
NATIVE_PASSPORT_BUILDPLAN v0.2.0
current svc-passport code and notes
current ron-auth/ron-kms/ron-proto/ron-naming/ron-policy source
current CrabLink codebundle and notes
current CRABLINK_TV_BUILDPLAN
ALLNOTES and ALL_DOCS
12_PILLARS
SIX_CONCERNS
APP_INTEGRATION_BLUEPRINT
INTEROP_BLUEPRINT
OMNIGATE_BLUEPRINT
HARDENING_BLUEPRINT
BUILD_PLAN_Z two-node/quorum posture
```

External implementation standards to freeze and validate during Phase 0:

```text
BIP-0039 profile
RFC 9106 Argon2id
RFC 8032 Ed25519
HKDF-SHA-256
XChaCha20-Poly1305
BLAKE3-256
Android Keystore
Apple Keychain/Secure Enclave
desktop secure-storage APIs
```

The core v0.3.0 doctrine is:

```text
self-custodied recovery root
separate revocable device keys
local PIN, never network password
device-bound capabilities
purpose-bound replay-resistant proof
@username, not actual human name, as optional public network handle
signed durable namespace transitions
immutable public content with signed mutable pointers
separate identity, namespace, content, and economic truth
one canonical svc-passport owner
Rust-first cross-platform implementation
```
