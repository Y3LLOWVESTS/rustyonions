
---

````markdown
---
title: Configuration — ron-naming
crate: ron-naming
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
template_version: 1.1
audience: contributors, ops, auditors
concerns: [SEC, GOV]
---

# Configuration — ron-naming

This document defines **all configuration** for `ron-naming`, including sources,
precedence, schema (types/defaults), validation, feature flags, (non-)reload
behavior, and security implications. It complements `README.md`, `IDB.md`,
and `SECURITY.md`.

> **Tiering**
>
> - **Library (default role):** Pure functions accept parameters; they **do not** read global config.  
> - **CLI (`tldctl`, feature `cli`):** Reads config from flags/env/file to drive **offline** authoring, linting, packing, signing, verifying of naming artifacts.  
> - **Service settings (ports, /healthz):** **N/A** for this crate.

---

## 1) Sources & Precedence (Authoritative)

Configuration can be supplied by the user when invoking the **CLI**. The **library
does not load config**; callers pass values directly via function parameters.

**Precedence (highest wins) for CLI:**

1. **Process flags** (CLI args to `tldctl`)  
2. **Environment variables**  
3. **Config file** (TOML/JSON via `--config`)  
4. **Built-in defaults** (documented below)

**Env prefixes:**

- Common: `RON_NAMING_…`
- CLI-only (takes precedence over common): `TLDCTL_…`  
  Example: `TLDCTL_FORMAT=cbor` overrides `RON_NAMING_FORMAT=json`.

**File formats:** TOML (preferred), JSON (optional).

**Path resolution for `--config` (if relative):** `./` then `$CWD`.

**Reload:** Not applicable (one-shot CLI). Library has no reload concept.

---

## 2) Quickstart Examples

### 2.1 Lint a TLD set (stdin → stdout, JSON)
```bash
echo '{ "version":1, "entries":[{"tld":"example","owner_key_id":"ops@k1","rules_version":1}] }' \
| TLDCTL_FORMAT=json cargo run -p ron-naming --features cli --bin tldctl -- lint -
````

### 2.2 Pack deterministically to CBOR

```bash
TLDCTL_FORMAT=cbor cargo run -p ron-naming --features cli --bin tldctl -- \
  pack ./tlds.json --out ./tldmap.cbor
```

### 2.3 Sign with an external KMS profile (detached multisig envelope)

```bash
TLDCTL_SIGN=true \
TLDCTL_SIGN_PROFILE=profile://ops-m-of-n \
TLDCTL_SIGN_THRESHOLD_M=2 \
TLDCTL_SIGN_N=3 \
cargo run -p ron-naming --features "cli,verify" --bin tldctl -- \
  sign ./tldmap.cbor --out ./signed-tldmap.cbor
```

### 2.4 Verify a signed map with PQ backend enabled

```bash
TLDCTL_PQ_MODE=hybrid \
cargo run -p ron-naming --features "cli,verify,pq" --bin tldctl -- \
  verify ./signed-tldmap.cbor
```

### 2.5 Use a config file + env overrides

```bash
cargo run -p ron-naming --features cli --bin tldctl -- \
  --config ./Config.toml pack - --out ./tldmap.cbor
```

---

## 2.a Full sample `Config.toml` (ready to commit)

```toml
# CLI behavior
format = "cbor"        # json|cbor
pretty = false         # pretty JSON (no effect for CBOR)
in_path = "-"          # "-" means stdin
out_path = "-"         # "-" means stdout

# Normalization policy
[normalize]
strict = true
allow_mixed_scripts = false
allowed_scripts = ["Latin"]   # optional whitelist
policy_bundle = "builtin"     # builtin|minimal|extended|custom
custom_tables_dir = ""        # used when bundle="custom"

# Signing / verification
[sign]
enabled = false
profile = "profile://ops-m-of-n"
threshold_m = 2
n = 3
key_ids = ["ops-k1","ops-k2","ops-k3"]

[pq]
mode = "off"                  # off|hybrid

[log]
format = "json"               # json|text
level  = "info"               # trace|debug|info|warn|error
```

### 2.b Env export snippet (ops convenience)

```bash
export TLDCTL_FORMAT=cbor
export TLDCTL_NORM_STRICT=true
export TLDCTL_SIGN=false
export TLDCTL_LOG_FORMAT=json
export TLDCTL_LOG_LEVEL=info
```

---

## 3) Schema (Typed, With Defaults)

> **Durations/Sizes:** Not applicable here (no network/server).
> **OAP tie-in:** Normalized names and governance artifacts are **small** by design; CLI/library must keep serialized outputs **well below** OAP/1 `max_frame = 1 MiB`.
> **Env names:** Prefer `TLDCTL_…` for CLI; `RON_NAMING_…` also accepted as a lower-precedence alias.

| Key / Env Var                                     | Type                                          | Default      | Applies to | Description                                                                             | Security Notes                                |
| ------------------------------------------------- | --------------------------------------------- | ------------ | ---------- | --------------------------------------------------------------------------------------- | --------------------------------------------- |
| `format` / `TLDCTL_FORMAT`                        | enum(`json`,`cbor`)                           | `cbor`       | CLI        | Output encoding for `pack`/`sign`/`show`. Input auto-detected or forced via `--format`. | JSON may leak whitespace; CBOR is compact.    |
| `pretty` / `TLDCTL_PRETTY`                        | bool                                          | `false`      | CLI        | Pretty-print JSON output (no effect for CBOR).                                          | Disable in scripts to keep outputs canonical. |
| `in_path` / `TLDCTL_IN`                           | path (`-` = stdin)                            | `-`          | CLI        | Default input path for commands accepting a single input.                               | Validate path; avoid shell glob surprises.    |
| `out_path` / `TLDCTL_OUT`                         | path (`-` = stdout)                           | `-`          | CLI        | Default output path; `-` keeps output on stdout.                                        | Overwrites only with explicit `--out`.        |
| `normalize.strict` / `TLDCTL_NORM_STRICT`         | bool                                          | `true`       | CLI/Lib    | Reject confusables/mixed-script by policy (default-deny).                               | Reduces spoof risks.                          |
| `normalize.allow_mixed_scripts` / `…_ALLOW_MIX`   | bool                                          | `false`      | CLI/Lib    | Permit mixed scripts when `strict=false`.                                               | Increases spoof risks; use sparingly.         |
| `normalize.allowed_scripts` / `…_ALLOWED_SCRIPTS` | list<string>                                  | `[]`         | CLI/Lib    | Optional allow-list (e.g., `["Latin"]`).                                                | Narrow scope to reduce risk.                  |
| `normalize.policy_bundle` / `…_POLICY_BUNDLE`     | enum(`builtin`,`minimal`,`extended`,`custom`) | `builtin`    | CLI/Lib    | Which built-in table set to use.                                                        | Custom bundles must be reviewed.              |
| `normalize.custom_tables_dir` / `…_TABLES_DIR`    | path                                          | `""`         | CLI/Lib    | Directory containing custom policy tables (only if `policy_bundle="custom"`).           | Treat as untrusted input.                     |
| `unicode.version_pin` / `…_UNICODE_VERSION`       | string                                        | pinned build | CLI/Lib    | Build-time Unicode/IDNA version pin (read-only at runtime; surfaced for diagnostics).   | Pin updates require vectors + review.         |
| `sign.enabled` / `TLDCTL_SIGN`                    | bool                                          | `false`      | CLI        | Enable signing flow (`sign` command).                                                   | Keys never stored by CLI.                     |
| `sign.profile` / `TLDCTL_SIGN_PROFILE`            | string URI                                    | `""`         | CLI        | KMS/HSM profile, e.g., `profile://ops-m-of-n`.                                          | No secrets in URI; use profile indirection.   |
| `sign.threshold_m` / `TLDCTL_SIGN_THRESHOLD_M`    | u8                                            | `2`          | CLI        | M in M-of-N multisig.                                                                   | Validate `1 ≤ M ≤ N`.                         |
| `sign.n` / `TLDCTL_SIGN_N`                        | u8                                            | `3`          | CLI        | N in M-of-N multisig.                                                                   |                                               |
| `sign.key_ids` / `TLDCTL_SIGN_KEY_IDS`            | list<string>                                  | `[]`         | CLI        | Logical key IDs included in the envelope metadata.                                      | Do not log.                                   |
| `verify.enabled` / `TLDCTL_VERIFY`                | bool                                          | `true`       | CLI        | Enable verification checks in `verify` command.                                         |                                               |
| `pq.mode` / `TLDCTL_PQ_MODE`                      | enum(`off`,`hybrid`)                          | `off`        | CLI/Lib    | Select verifier/signing hybrid mode (e.g., Ed25519 + Dilithium).                        | Interop gating required.                      |
| `log.format` / `TLDCTL_LOG_FORMAT`                | enum(`json`,`text`)                           | `json`       | CLI        | CLI log output format.                                                                  | JSON preferred for CI.                        |
| `log.level` / `TLDCTL_LOG_LEVEL`                  | enum                                          | `info`       | CLI        | `trace`..`error`.                                                                       | Avoid `trace` in CI; may leak paths.          |

> The library surfaces policy/Unicode versions via functions like `version_info()`.
> Consumers should treat them as **diagnostic values**, not mutable config.

---

## 4) Validation Rules (Fail-Closed)

Applied by the CLI at startup (and by library constructors where relevant):

* `format ∈ {json,cbor}`; `pretty` valid only when `format=json`.
* If `normalize.policy_bundle="custom"` then `normalize.custom_tables_dir` **must exist** and contain required table files.
* If `normalize.strict=true`, deny mixed scripts regardless of `allow_mixed_scripts`.
* If `sign.enabled=true` then:

  * `sign.profile ≠ ""`,
  * `1 ≤ threshold_m ≤ n`,
  * if `key_ids` provided, **length ≥ n** and unique.
* `pq.mode="hybrid"` requires the selected `Verifier/Signer` backend to advertise support; otherwise **fail** with a clear message.
* `in_path` and `out_path` are validated; `out_path="-"` is permitted; writing to an existing file requires explicit `--out` (no silent overwrite).
* Unknown keys in config files are **rejected** (serde `deny_unknown_fields`).

**On violation**: CLI prints a structured error to stderr and exits with code `64` (usage) or `2/3` for validation/signature failures. Library APIs return typed errors; **no panics** on user input.

---

## 5) Dynamic Reload

**Not supported.** `tldctl` is a one-shot CLI; library is stateless/pure.

---

## 6) CLI Flags (Canonical)

> Flags override env and file settings.

```
# Global
--config <path>                 # Load TOML/JSON config (lowest precedence)
--format <json|cbor>
--pretty                        # Pretty JSON (no effect for CBOR)
--in <path|->                   # Default stdin/out shorthands
--out <path|->

# Normalization
--strict                        # Enforce strict policy (default true)
--no-strict                     # Disable strict policy
--allow-mixed-scripts           # Only if not strict
--allowed-scripts <CSV>
--policy-bundle <builtin|minimal|extended|custom>
--custom-tables-dir <path>

# Signing / Verification
--sign                          # Enable signing flow
--sign-profile <uri>            # e.g., profile://ops-m-of-n
--m <num>                       # threshold M
--n <num>                       # total N
--key-id <id> ...               # repeatable
--verify                        # Enable verify flow (default true)
--pq-mode <off|hybrid>

# Logging
--log-format <json|text>
--log-level <trace|debug|info|warn|error>
```

> Subcommands: `lint <in>`, `pack <in> [--out <out>]`, `sign <in> [--out <out>]`,
> `verify <in>`, `show <in>`.

---

## 7) Feature Flags (Cargo)

| Feature        | Default | Effect                                                                          |
| -------------- | ------: | ------------------------------------------------------------------------------- |
| `cli`          |     off | Builds the `tldctl` binary and pulls arg-parsing/logging deps for the CLI only. |
| `verify`       |     off | Enables signature verification helpers and KMS integration points.              |
| `pq`           |     off | Wires PQ verifier/signers (e.g., Dilithium) via `ron-kms` backend.              |
| `large-tables` |     off | Bundles extended Unicode/confusables tables (larger binary).                    |

> Enabling `cli` **must not** raise MSRV or violate workspace dependency pins.

---

## 8) Security Implications

* **Keys & secrets**: The CLI never stores keys; it references an external **profile** (KMS/HSM). No secret material should appear in logs or on disk via this tool.
* **Normalization strictness**: Relaxing policy (allowing mixed scripts) increases spoof risks; defaults are deny-by-policy.
* **Custom policy bundles**: Treat directories as untrusted input; pin and review changes; include provenance in PRs.
* **Canonical outputs**: Prefer CBOR in pipelines; JSON pretty is for humans.
* **PQ hybrid**: Interop must be planned (publish mode in release notes) to avoid network partitions later when services enforce maps.
* **Amnesia-compatibility**: Lib is stateless; CLI is one-shot with explicit I/O only. In “amnesia mode,” there is **no implicit state** to purge beyond user-provided files.

---

## 9) Compatibility & Migration

* **Additive keys** are introduced with safe defaults (`strict=true`, `format=cbor`).
* **Renames** ship with env aliases for ≥1 minor; CLI emits a deprecation warning.
* **Breaking changes** (e.g., policy semantics) require a major version bump and documented migration steps (including regeneration of golden vectors).

**Deprecation table**

| Old Key              | New Key                       | Removal Target | Notes               |
| -------------------- | ----------------------------- | -------------: | ------------------- |
| `policy_bundle_path` | `normalize.custom_tables_dir` |         v0.2.0 | Name clarified      |
| `sign.threshold`     | `sign.threshold_m`            |         v0.2.0 | Disambiguate from N |

---

## 10) Reference Implementation (Rust)

> Minimal structures used by the **CLI**. The **library** takes params directly and does not read env/file.

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct NormalizeCfg {
    #[serde(default = "default_true")]
    pub strict: bool,
    #[serde(default)]
    pub allow_mixed_scripts: bool,
    #[serde(default)]
    pub allowed_scripts: Vec<String>,
    #[serde(default = "default_policy_bundle")]
    pub policy_bundle: PolicyBundle,
    #[serde(default)]
    pub custom_tables_dir: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyBundle { Builtin, Minimal, Extended, Custom }
fn default_policy_bundle() -> PolicyBundle { PolicyBundle::Builtin }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default = "default_m")]
    pub threshold_m: u8,
    #[serde(default = "default_n")]
    pub n: u8,
    #[serde(default)]
    pub key_ids: Vec<String>,
}
fn default_m() -> u8 { 2 }
fn default_n() -> u8 { 3 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PqCfg {
    #[serde(default = "default_pq_mode")]
    pub mode: PqMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PqMode { Off, Hybrid }
fn default_pq_mode() -> PqMode { PqMode::Off }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogCfg {
    #[serde(default = "default_log_fmt")]
    pub format: LogFormat,
    #[serde(default = "default_log_level")]
    pub level: LogLevel,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat { Json, Text }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel { Trace, Debug, Info, Warn, Error }
fn default_log_fmt() -> LogFormat { LogFormat::Json }
fn default_log_level() -> LogLevel { LogLevel::Info }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliCfg {
    #[serde(default = "default_format")]
    pub format: OutputFormat,      // json|cbor
    #[serde(default)]
    pub pretty: bool,              // json only
    #[serde(default = "default_in")]
    pub in_path: String,           // "-" = stdin
    #[serde(default = "default_out")]
    pub out_path: String,          // "-" = stdout
    #[serde(default)]
    pub normalize: NormalizeCfg,
    #[serde(default)]
    pub sign: SignCfg,
    #[serde(default)]
    pub pq: PqCfg,
    #[serde(default)]
    pub log: LogCfg,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat { Json, Cbor }
fn default_format() -> OutputFormat { OutputFormat::Cbor }
fn default_in() -> String { "-".to_string() }
fn default_out() -> String { "-".to_string() }

impl CliCfg {
    pub fn validate(&self) -> anyhow::Result<()> {
        if matches!(self.format, OutputFormat::Json) == false && self.pretty {
            // pretty ignored for CBOR; not an error
        }
        let norm = &self.normalize;
        if matches!(norm.policy_bundle, PolicyBundle::Custom) && norm.custom_tables_dir.is_none() {
            anyhow::bail!("custom policy bundle requires custom_tables_dir");
        }
        let s = &self.sign;
        if s.enabled {
            if s.profile.as_deref().unwrap_or("").is_empty() {
                anyhow::bail!("sign.enabled=true but sign.profile missing");
            }
            if !(1..=s.n).contains(&s.threshold_m) {
                anyhow::bail!("sign.threshold_m must satisfy 1 ≤ M ≤ N");
            }
        }
        Ok(())
    }
}
```

---

## 11) Test Matrix

| Scenario                                    | Expected Outcome                         |
| ------------------------------------------- | ---------------------------------------- |
| Unknown key in `Config.toml`                | Fail with “unknown field”                |
| `policy_bundle=custom` without tables dir   | Fail with clear message                  |
| `sign.enabled=true` but no `sign.profile`   | Fail with clear message                  |
| `threshold_m > n`                           | Fail with clear message                  |
| `--format json --pretty`                    | Pretty JSON output                       |
| `verify` on tampered signature              | Exit code `3` (signature failed)         |
| Mixed-script input when `strict=true`       | `lint` fails; exit code `2`              |
| PQ hybrid with backend lacking PQ           | Fail with interop capability error       |
| `out_path` is existing file without `--out` | No overwrite unless explicitly specified |

---

## 12) Mermaid — Config Resolution Flow (CLI)

```mermaid
flowchart TB
  F[Flags] --> M[Merge]
  E[Env (TLDCTL_ > RON_NAMING_)] --> M
  C[Config file (TOML/JSON)] --> M
  D[Defaults] --> M
  M --> V{Validate}
  V -- ok --> RUN[Execute subcommand]
  V -- fail --> ERR[Exit non-zero]
  style RUN fill:#0ea5e9,stroke:#075985,color:#fff
```

---

## 13) Operational Notes

* Keep policy bundles and signed artifacts in **version control**; treat them as code (review, provenance).
* Prefer **CBOR** for machine pipelines and **JSON** only for human inspection or diffs.
* For PQ rollout, publish the planned `pq.mode` in release notes so consuming services can align.
* The CLI is **offline by design**; any request to add network behavior should be rejected or moved to a service crate (e.g., `svc-registry`, `svc-index`).
* **Amnesia:** Since the lib is stateless and the CLI is one-shot with explicit I/O only, amnesia mode is trivially satisfied (no implicit state).

```
```
