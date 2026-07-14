// crates/svc-admin/ui/src/types/admin-api.ts
//
// WHAT: Shared TypeScript DTO definitions for the svc-admin SPA.
// WHY:  Keep the UI strictly aligned with the Rust-side DTOs in
//       `crates/svc-admin/src/dto/*` and the documented JSON contracts.
//
// NOTE:
//   - UiConfigDto mirrors `dto::ui::UiConfigDto` (camelCase via serde).
//   - MeResponse mirrors `dto::me::MeResponse` (camelCase via serde).
//   - Node / metrics types mirror `dto::node` and `dto::metrics`.
//
// COMPAT POLICY (important):
//   Rust DTOs sometimes use snake_case (no rename_all) and sometimes camelCase (rename_all="camelCase").
//   To prevent UI breakage while endpoints are rolling out, we allow harmless alias fields as OPTIONAL
//   (e.g. restart_count vs restartCount vs restarts). Prefer the canonical field when present.

export type UiDevConfig = {
  enableAppPlayground: boolean
}

export type UiConfigDto = {
  defaultTheme: string
  availableThemes: string[]
  defaultLanguage: string
  availableLanguages: string[]
  readOnly: boolean
  dev: UiDevConfig
}

export type MeResponse = {
  subject: string
  displayName: string
  roles: string[]
  authMode: string
  // Optional and may be null when no interactive login is available.
  loginUrl?: string | null
}

// ---- Node listing / status ----------------------------------------------
//
// Mirrors `dto::node::NodeSummary` on the Rust side.
// NOTE: JSON field names are snake_case here (no rename_all on the Rust struct).

export type NodeSummary = {
  id: string
  display_name: string
  profile: string | null

  // Optional compat aliases (safe to ignore if absent).
  displayName?: string
}

// Mirrors `dto::node::PlaneStatus`.
export type PlaneStatus = {
  name: string
  health: 'healthy' | 'degraded' | 'down'
  ready: boolean

  // Canonical (snake_case) used by current Rust DTO.
  restart_count: number

  // Compat aliases (optional) — older/other emitters may use these.
  restarts?: number
  restartCount?: number

  // Optional timing hints some emitters may include (safe for UI).
  age_s?: number
  ageSecs?: number
  age_ms?: number
  ageMs?: number
  sample_age_secs?: number
  last_sample_age_secs?: number
}

export type OapStatusView = {
  protocol: string
  version: number
  runtime_state: string
  max_frame_bytes: number
  stream_chunk_bytes: number
  object_fetch_active: boolean
  full_digest_verification_active: boolean
}

export type ProviderStatusView = {
  state: string
  dht_worker_status: string
  advertisement_active: boolean
  provider_records_published: number
  public_node_uri_format: string
  residential_ip_publication: boolean
}

export type ModerationEntryCountsView = {
  total: number
  global_deny: number
  local_block: number
  local_allow: number
  owner_tombstone: number
  quarantine: number
}

export type PolicyStatusView = {
  state: string
  serve_policy_enforced: boolean
  oap_serve_policy_enforced: boolean
  operator_moderation_active: boolean
  global_moderation_active: boolean
  moderation_configured: boolean
  moderation_state: string
  moderation_source: string
  moderation_load_failed: boolean
  signed_policy_verified: boolean
  signed_policy_epoch: number | null
  signed_policy_expires_at_unix_s: number | null
  rollback_guard_persisted: boolean
  moderation_activation: string
  moderation_hot_reload: boolean
  moderation_entries: ModerationEntryCountsView
  unvetted_persistence_posture: string
  serve_gate_phase: string
  moderation_phase: string
}

export type PersistenceReviewStatusView = {
  state: string
  candidates_total: number
  awaiting_decision: number
  pending_review: number
  persistence_approvals: number
  blocked_candidates: number
  quarantined_candidates: number
  completed_local_prunes: number
  durable_bytes_written: boolean
  reward_finality: boolean
  wallet_mutation: boolean
  ledger_mutation: boolean
}


export type EconomicPipelineStatusView = {
  stage: string
  accounting_snapshot: EconomicAccountingSnapshotStatusView
  reward_plan?: EconomicRewardPlanStatusView | null
  epoch_transition?: EconomicEpochTransitionStatusView | null
  epoch_payout_receipts?: EconomicEpochPayoutReceiptStatusView | null
  wallet_execution_reported: boolean
  ledger_receipt_reported: boolean
  confirmed_roc_reported: boolean
  finality_reported: boolean
  operator_projection_authorizes_economic_mutation: boolean
}

export type EconomicEpochPayoutReceiptStatusView = {
  receipt_count: number
  recipient_count: number
  total_issued_minor: string
  first_ledger_seq: number
  last_ledger_seq: number
  ledger_root: string
  first_receipt_hash: string
  last_receipt_hash: string
  accepted_at_ms: number
  wallet_source: string
  ledger_source: string
  settlement_status: string
  finality_status: string
}

export type EconomicAccountingSnapshotStatusView = {
  chain_id: string
  snapshot_id: string
  snapshot_root: string
  window_started_at_ms: number
  window_ended_at_ms: number
  sealed_at_ms: number
  source_event_count: number
  economic_receipt_count: number
  metering_count: number
  proof_eligible_count: number
  ad_budgeted_count: number
  analytics_only_count: number
}

export type EconomicRewardPlanStatusView = {
  plan_id: string
  plan_root: string
  snapshot_id: string
  snapshot_root: string
  source_event_class: string
  planned_total_minor: string
  payout_candidate_count: number
  capped_by_policy: boolean
  verification_ref?: string | null
  funding_budget_ref?: string | null
  produced_at_ms: number
}

export type EconomicEpochTransitionStatusView = {
  chain_id: string
  epoch_id: string
  transition_hash: string
  accounting_snapshot_hash: string
  reward_plan_hash: string
  policy_hash: string
  economics_config_hash: string
  registry_root: string
  reward_binding_root: string
  evidence_root: string
  reward_cap_minor_units: string
  reward_total_minor_units: string
  allocation_count: number
  eligible_service_node_count: number
  required_signature_references: number
  supplied_signature_references: number
  quorum_reference_threshold_met: boolean
  cryptographic_signatures_verified: boolean
  recipient_accounts_resolved: boolean
  produced_at_ms: number
}

export type ServiceNodeLifecycleStatusView = {
  lifecycle_state: string
  registered_at_epoch: number
  state_effective_epoch: number
  quorum_status: string
  counts_toward_quorum: boolean
  probation_reward_cap_required: boolean
  enforcement?: ServiceNodeContainmentStatusView | null
  operator_projection_authorizes_state_change: boolean
  operator_projection_authorizes_economic_mutation: boolean
}

export type ServiceNodeContainmentStatusView = {
  status_id: string
  state: string
  reason: string
  evidence_root: string
  effective_epoch: number
  counts_toward_quorum: boolean
  permits_reward_planning: boolean
  authorizes_economic_mutation: boolean
  appeal: ServiceNodeAppealStatusView
}

export type ServiceNodeAppealStatusView = {
  state: string
  appeal_id?: string | null
  submitted_epoch?: number | null
  resolved_epoch?: number | null
  resolution_evidence_root?: string | null
  pending: boolean
  authorizes_state_change: boolean
}

export type RewardBindingStatusView = {
  state: string
  reward_recipient_display_address: string | null
  pending_rotation_display_address: string | null
  updated_at_unix_s: number | null
  registry_finality: boolean
  wallet_mutation: boolean
  ledger_mutation: boolean
  confirmed_roc_minor_units: number | null
}

export type ServiceEvidenceStatusView = {
  state: string
  queued_records: number
  delivery_records: number
  reward_evidence_records: number
  signature_required: boolean
  replay_scope: string
  durable: boolean
  accounting_accepted: boolean
  reward_eligible: boolean
  reward_truth: boolean
  payout_authority: boolean
  wallet_mutation: boolean
  ledger_mutation: boolean
}

// Mirrors `dto::node::AdminStatusView`.
export type AdminStatusView = {
  id: string
  display_name: string
  profile: string | null
  version: string | null

  node_role?: 'user_node' | 'service_node' | 'devnet_all_in_one' | string | null
  node_profile?: string | null
  uptime_seconds?: number | null

  planes: PlaneStatus[]
  capabilities?: string[] | null

  ready?: boolean | null
  oap?: OapStatusView | null
  provider?: ProviderStatusView | null
  policy?: PolicyStatusView | null
  economic_pipeline?: EconomicPipelineStatusView | null
  service_node_lifecycle?: ServiceNodeLifecycleStatusView | null
  persistence_review?: PersistenceReviewStatusView | null
  reward_binding?: RewardBindingStatusView | null
  service_evidence?: ServiceEvidenceStatusView | null

  amnesia_mode?: boolean | null
  privacy_mode?: boolean | null
  public_inbound_enabled?: boolean | null
  headless_mode?: boolean | null
  admin_ui_enabled?: boolean | null
  admin_ui_bind?: string | null
  operator_ui_profile?: string | null
  admin_ui_runtime_required?: boolean | null
  verification_enabled?: boolean | null
  content_serving_enabled?: boolean | null
  economic_replay_enabled?: boolean | null
  service_quorum_enabled?: boolean | null
  wallet_execution_participant?: boolean | null
  ledger_replay_enabled?: boolean | null
  user_ip_publication?: string | null
  peer_ip_display?: string | null
  admin_bind_publication?: boolean | null
  service_socket_publication?: string | null
  transport_routes_public?: boolean | null
  raw_socket_publication?: boolean | null

  displayName?: string
  uptimeSeconds?: number | null
  nodeRole?: string | null
  nodeProfile?: string | null
}

// ---- Facet metrics DTO ---------------------------------------------------
//
// Mirrors `dto::metrics::FacetMetricsSummary`.
// Historically this has been snake_case on the wire.

export type FacetMetricsSummary = {
  facet: string
  rps: number
  error_rate: number
  p95_latency_ms: number
  p99_latency_ms: number
  last_sample_age_secs: number | null

  // Compat aliases (optional) — safe.
  errorRate?: number
  p95LatencyMs?: number
  p99LatencyMs?: number
  lastSampleAgeSecs?: number | null
}

// ---- Node actions DTO ----------------------------------------------------
//
// Mirrors `dto::node::NodeActionResponse`.

export type NodeActionResponse = {
  node_id: string
  action: string
  accepted: boolean
  message?: string | null

  // Compat alias (optional)
  nodeId?: string
}

// ---- Storage / Databases (read-only) -------------------------------------

export type NodeCapability = 'storage.readonly.v1' | string

export type StorageSummaryDto = {
  fsType: string
  mount: string

  totalBytes: number
  usedBytes: number
  freeBytes: number

  ioReadBps: number | null
  ioWriteBps: number | null
}

export type DatabaseHealth = 'ok' | 'degraded' | 'error'

export type DatabaseEntryDto = {
  name: string
  engine: string
  sizeBytes: number

  mode: string
  owner: string

  health: DatabaseHealth

  notes?: string | null
  worldReadable?: boolean
  worldWritable?: boolean
}

export type DatabaseDetailDto = {
  name: string
  engine: string
  sizeBytes: number

  mode: string
  owner: string
  health: DatabaseHealth

  pathAlias: string

  fileCount: number
  lastCompaction: string | null

  approxKeys: number | null
  warnings: string[]
}

// ---- App Playground (dev-only, read-only MVP) ----------------------------

export type PlaygroundExampleDto = {
  id: string
  title: string
  description: string
  manifestToml: string
}

export type PlaygroundValidateManifestReq = {
  manifestToml: string
}

export type PlaygroundValidateManifestResp = {
  ok: boolean
  errors: string[]
  warnings: string[]
  parsed?: unknown | null
}

// ---- System summary ------------------------------------------------------

export type SystemSummaryDto = {
  updatedAt: string
  cpuPercent?: number | null

  // NEW: optional basic CPU facts (safe for rollout)
  cpuCores?: number | null
  cpuThreads?: number | null

  ramTotalBytes: number
  ramUsedBytes: number

  netRxBps?: number | null
  netTxBps?: number | null
}

// ---- Network accounting (read-only optional endpoint) --------------------

export type NetAccountingSeriesPoint = {
  ts?: number
  t?: number
  time?: number
  at?: number
  epoch?: number
  epochSeconds?: number
  totalBytes?: number
  bytes?: number
  b?: number
  requests?: number
  req?: number
  r?: number
}

export type NetAccountingRollup = {
  totalBytes?: number
  bytes?: number
  requests?: number
  rxBytes?: number
  txBytes?: number
  readBytes?: number
  writeBytes?: number
  requestsByFacet?: Record<string, number>
}

export type NetAccountingDto = {
  minute?: NetAccountingRollup
  hour?: NetAccountingRollup
  day?: NetAccountingRollup
  month?: NetAccountingRollup
  seriesMinute?: NetAccountingSeriesPoint[]
  seriesHour?: NetAccountingSeriesPoint[]
  seriesDay?: NetAccountingSeriesPoint[]
  seriesMonth?: NetAccountingSeriesPoint[]
}

// ---- Benchmarks (node-executed; bounded) ---------------------------------

export type BenchRunReq = {
  suite: string
  durationSecs: number
  concurrency: number
  payloadSize: number
  seed: number
  limits?: {
    maxDurationSecs?: number
    maxConcurrency?: number
    maxPayloadSize?: number
  }
}

export type BenchRunResp = {
  runId: string
}

export type BenchRunState = 'queued' | 'running' | 'done' | 'failed'

export type BenchRunStatusDto = {
  runId: string
  status: BenchRunState
  progress: number
  phase: string
  startedAt?: string | null
  endedAt?: string | null
  error?: string | null
  partial?: unknown | null
}

export type BenchScenarioResultDto = {
  name: string
  ok: boolean
  p50LatencyMs?: number | null
  p95LatencyMs?: number | null
  p99LatencyMs?: number | null
  throughputOpsPerSec?: number | null
  throughputBytesPerSec?: number | null
  errorRate?: number | null
  notes?: string[] | null
}

export type BenchRunResultDto = {
  runId: string
  suite: string
  nodeId?: string | null
  startedAt: string
  endedAt: string
  scenarios: BenchScenarioResultDto[]
  env?: {
    service?: string | null
    version?: string | null
    gitSha?: string | null
    profile?: string | null
    host?: string | null
    os?: string | null
    arch?: string | null
  } | null
}
