// crates/svc-admin/ui/src/lib/nodeCapabilities.ts
//
// RO:WHAT — Capability helpers for two-node svc-admin UI gating.
// RO:WHY  — Keep user_node/private-verifier views from showing service-node-only
//           operator controls while keeping old/partial nodes rollout-safe.
// RO:INVARIANTS —
//   - user_node never gets service/operator actions by default.
//   - service_node keeps content/storage/operator panels.
//   - unknown role remains permissive enough for rollout compatibility.

import type { AdminStatusView, NodeSummary } from '../types/admin-api'

export type TwoNodeRole = 'user_node' | 'service_node' | 'devnet_all_in_one' | 'unknown'

function norm(v: unknown): string {
  return String(v ?? '').trim().toLowerCase()
}

function capsOf(status: AdminStatusView | null | undefined): string[] {
  const caps = status?.capabilities
  return Array.isArray(caps) ? caps.map((c) => String(c)) : []
}

export function nodeRoleOf(
  status: AdminStatusView | null | undefined,
  node?: NodeSummary | null,
): TwoNodeRole {
  const explicit = norm(status?.node_role)
  if (explicit === 'user_node') return 'user_node'
  if (explicit === 'service_node') return 'service_node'
  if (explicit === 'devnet_all_in_one') return 'devnet_all_in_one'

  const profile = norm(status?.node_profile ?? status?.profile ?? node?.profile)
  if (profile === 'micronode' || profile.includes('user')) return 'user_node'
  if (profile === 'macronode' || profile.includes('service')) return 'service_node'

  return 'unknown'
}

export type NodeCapabilityView = {
  role: TwoNodeRole
  roleLabel: string
  roleDescription: string
  nodeProfile: string | null
  isUserNode: boolean
  isServiceNode: boolean
  isUnknown: boolean
  canShowServicePanels: boolean
  canOpenStorage: boolean
  canShowOperatorActions: boolean
  canShowDebugActions: boolean
  privacyMode: boolean | null
  amnesiaMode: boolean | null
  publicInboundEnabled: boolean | null
  verificationEnabled: boolean | null
  economicReplayEnabled: boolean | null
  ledgerReplayEnabled: boolean | null
  contentServingEnabled: boolean | null
  serviceQuorumEnabled: boolean | null
  walletExecutionParticipant: boolean | null
  userIpPublication: string | null
  capabilities: string[]
}

export function buildNodeCapabilityView(
  status: AdminStatusView | null | undefined,
  node?: NodeSummary | null,
): NodeCapabilityView {
  const role = nodeRoleOf(status, node)
  const capabilities = capsOf(status)
  const hasCap = (cap: string) => capabilities.includes(cap)

  const isUserNode = role === 'user_node'
  const isServiceNode = role === 'service_node'
  const isUnknown = role === 'unknown'
  const isDevnet = role === 'devnet_all_in_one'

  const contentServingEnabled =
    typeof status?.content_serving_enabled === 'boolean'
      ? status.content_serving_enabled
      : null

  const serviceQuorumEnabled =
    typeof status?.service_quorum_enabled === 'boolean'
      ? status.service_quorum_enabled
      : null

  const walletExecutionParticipant =
    typeof status?.wallet_execution_participant === 'boolean'
      ? status.wallet_execution_participant
      : null

  const canOpenStorage =
    isServiceNode ||
    isDevnet ||
    contentServingEnabled === true ||
    hasCap('storage.readonly.v1') ||
    hasCap('storage_summary_v1') ||
    hasCap('content_service_shell') ||
    // rollout compatibility for old nodes that do not yet report role/caps
    isUnknown

  const canShowServicePanels = !isUserNode
  const canShowOperatorActions = !isUserNode
  const canShowDebugActions = !isUserNode

  const roleLabel =
    role === 'user_node'
      ? 'User Node'
      : role === 'service_node'
        ? 'Service Node'
        : role === 'devnet_all_in_one'
          ? 'Devnet All-in-One'
          : 'Unknown Node'

  const roleDescription =
    role === 'user_node'
      ? 'Private passive verifier. No public serving, no wallet authority, no service-node operator actions.'
      : role === 'service_node'
        ? 'Public service/operator node. Serves content and may participate in future quorum surfaces.'
        : role === 'devnet_all_in_one'
          ? 'Local/dev co-location profile. Useful for testing, not a production third node type.'
          : 'Role not reported yet. svc-admin keeps older rollout surfaces visible but labels missing data.'

  const privacyMode =
    typeof status?.privacy_mode === 'boolean'
      ? status.privacy_mode
      : isUserNode
        ? true
        : null

  const amnesiaMode =
    typeof status?.amnesia_mode === 'boolean'
      ? status.amnesia_mode
      : null

  const publicInboundEnabled =
    typeof status?.public_inbound_enabled === 'boolean'
      ? status.public_inbound_enabled
      : isUserNode
        ? false
        : null

  const verificationEnabled =
    typeof status?.verification_enabled === 'boolean'
      ? status.verification_enabled
      : null

  const economicReplayEnabled =
    typeof status?.economic_replay_enabled === 'boolean'
      ? status.economic_replay_enabled
      : null

  const ledgerReplayEnabled =
    typeof status?.ledger_replay_enabled === 'boolean'
      ? status.ledger_replay_enabled
      : null

  const userIpPublication =
    typeof status?.user_ip_publication === 'string'
      ? status.user_ip_publication
      : isUserNode
        ? 'forbidden'
        : isServiceNode
          ? 'not_applicable_service_node'
          : null

  return {
    role,
    roleLabel,
    roleDescription,
    nodeProfile: status?.node_profile ?? status?.profile ?? node?.profile ?? null,
    isUserNode,
    isServiceNode,
    isUnknown,
    canShowServicePanels,
    canOpenStorage,
    canShowOperatorActions,
    canShowDebugActions,
    privacyMode,
    amnesiaMode,
    publicInboundEnabled,
    verificationEnabled,
    economicReplayEnabled,
    ledgerReplayEnabled,
    contentServingEnabled,
    serviceQuorumEnabled,
    walletExecutionParticipant,
    userIpPublication,
    capabilities,
  }
}

export function postureValue(v: boolean | null | undefined, yes = 'Enabled', no = 'Disabled'): string {
  if (typeof v !== 'boolean') return 'Not reported'
  return v ? yes : no
}
