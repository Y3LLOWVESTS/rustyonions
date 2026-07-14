// crates/svc-admin/ui/src/routes/node-detail/OperatorTruthPanels.tsx
//
// RO:WHAT — Read-only service-node truth panels for Phase 20.
// RO:WHY — Surface backend-reported protocol, provider, moderation,
//          reward-binding, and evidence posture without fake success.
// RO:INVARIANTS — Missing data remains unavailable. Evidence is not ROC.
//                 Binding state is not registry finality or a receipt.

import type { ReactNode } from 'react'

import type { AdminStatusView } from '../../types/admin-api'

type CardProps = {
  title: string
  subtitle: string
  children: ReactNode
}

function TruthCard({ title, subtitle, children }: CardProps) {
  return (
    <article className="svc-admin-operator-truth-card">
      <header className="svc-admin-operator-truth-card-header">
        <h3>{title}</h3>
        <p>{subtitle}</p>
      </header>

      {children}
    </article>
  )
}

function TruthRow({
  label,
  value,
  mono = false,
}: {
  label: string
  value: ReactNode
  mono?: boolean
}) {
  return (
    <div className="svc-admin-operator-truth-row">
      <span className="svc-admin-operator-truth-label">{label}</span>
      <span
        className={
          mono
            ? 'svc-admin-operator-truth-value mono'
            : 'svc-admin-operator-truth-value'
        }
      >
        {value}
      </span>
    </div>
  )
}

function TruthFlag({
  label,
  value,
}: {
  label: string
  value: boolean | null | undefined
}) {
  const state =
    typeof value === 'boolean' ? (value ? 'true' : 'false') : 'unknown'

  let text = 'Not reported'
  if (value === true) text = 'Yes'
  if (value === false) text = 'No'

  return (
    <span className="svc-admin-operator-truth-flag" data-state={state}>
      <span>{label}</span>
      <strong>{text}</strong>
    </span>
  )
}

function Unavailable({ label }: { label: string }) {
  return (
    <div className="svc-admin-operator-truth-unavailable">
      {label} was not reported by this node.
    </div>
  )
}

function textValue(value: string | null | undefined): string {
  const clean = value?.trim()
  return clean ? clean : 'Not reported'
}

function numberValue(value: number | null | undefined): string {
  return typeof value === 'number' && Number.isFinite(value)
    ? value.toLocaleString()
    : 'Not reported'
}

function bytesValue(value: number | null | undefined): string {
  if (
    typeof value !== 'number' ||
    !Number.isFinite(value) ||
    value < 0
  ) {
    return 'Not reported'
  }

  if (value >= 1024 * 1024) {
    return `${(value / (1024 * 1024)).toFixed(2)} MiB`
  }

  if (value >= 1024) {
    return `${(value / 1024).toFixed(2)} KiB`
  }

  return `${value} B`
}

function unixMsValue(
  value: number | null | undefined,
): string {
  if (
    typeof value !== 'number' ||
    !Number.isFinite(value) ||
    value < 0
  ) {
    return 'Not reported'
  }

  const date = new Date(value)
  return Number.isNaN(date.getTime())
    ? 'Not reported'
    : date.toISOString()
}

function unixValue(value: number | null | undefined): string {
  if (
    typeof value !== 'number' ||
    !Number.isFinite(value) ||
    value < 0
  ) {
    return 'Not reported'
  }

  const date = new Date(value * 1000)
  return Number.isNaN(date.getTime())
    ? 'Not reported'
    : date.toISOString()
}

export function OperatorTruthPanels({
  status,
}: {
  status: AdminStatusView
}) {
  const oap = status.oap
  const provider = status.provider
  const policy = status.policy
  const economic = status.economic_pipeline
  const lifecycle = status.service_node_lifecycle
  const persistence = status.persistence_review
  const binding = status.reward_binding
  const evidence = status.service_evidence

  const hasTruthBlock = Boolean(
    oap ||
      provider ||
      policy ||
      economic ||
      lifecycle ||
      persistence ||
      binding ||
      evidence,
  )

  const signedPolicyExpired =
    typeof policy?.signed_policy_expires_at_unix_s === 'number' &&
    policy.signed_policy_expires_at_unix_s * 1000 <= Date.now()

  const bindingReportsMutation = Boolean(
    binding?.wallet_mutation || binding?.ledger_mutation,
  )

  const evidenceReportsAuthority = Boolean(
    evidence?.reward_truth ||
      evidence?.payout_authority ||
      evidence?.wallet_mutation ||
      evidence?.ledger_mutation,
  )

  const persistenceReportsAuthority = Boolean(
    persistence?.durable_bytes_written ||
      persistence?.reward_finality ||
      persistence?.wallet_mutation ||
      persistence?.ledger_mutation,
  )

  const runtimePostureReported = [
    status.amnesia_mode,
    status.headless_mode,
    status.admin_ui_enabled,
    status.admin_ui_runtime_required,
    status.content_serving_enabled,
    status.economic_replay_enabled,
    status.service_quorum_enabled,
    status.wallet_execution_participant,
    status.ledger_replay_enabled,
    status.privacy_mode,
    status.public_inbound_enabled,
    status.user_ip_publication,
    status.peer_ip_display,
    status.admin_bind_publication,
    status.service_socket_publication,
    status.transport_routes_public,
    status.raw_socket_publication,
  ].some((value) => value !== null && value !== undefined)

  const persistencePosture =
    status.amnesia_mode === true
      ? 'amnesia-first'
      : status.amnesia_mode === false
        ? 'amnesia disabled'
        : 'Not reported'

  const uiRuntimeViolation =
    status.admin_ui_runtime_required === true

  const directPublicationRisk = Boolean(
    status.admin_bind_publication === true ||
      status.transport_routes_public === true ||
      status.raw_socket_publication === true,
  )

  let readinessState = 'unknown'
  let readinessText = 'Readiness not reported'

  if (status.ready === true) {
    readinessState = 'ready'
    readinessText = 'Backend ready'
  } else if (status.ready === false) {
    readinessState = 'not-ready'
    readinessText = 'Backend not ready'
  }

  return (
    <section className="svc-admin-section svc-admin-section-operator-truth">
      <div className="svc-admin-operator-truth-heading">
        <div>
          <h2>Service-node truth</h2>
          <p>
            Read-only backend status. These panels do not create rewards,
            receipts, finality, or wallet and ledger authority.
          </p>
        </div>

        <span
          className="svc-admin-operator-truth-readiness"
          data-state={readinessState}
        >
          {readinessText}
        </span>
      </div>

      {!hasTruthBlock && (
        <div className="svc-admin-operator-truth-empty">
          This node has not published the Phase 20 operator-status blocks.
          svc-admin is showing unavailable state rather than placeholder
          success.
        </div>
      )}

      <div className="svc-admin-operator-truth-grid">
        <TruthCard
          title="Runtime, quorum and privacy"
          subtitle="Headless independence, persistence posture, participation switches, and publication boundaries."
        >
          {runtimePostureReported ? (
            <>
              <div className="svc-admin-operator-truth-subsection">
                <h4>Runtime independence</h4>

                <TruthRow
                  label="Persistence posture"
                  value={persistencePosture}
                  mono
                />
                <TruthRow
                  label="Operator UI profile"
                  value={textValue(status.operator_ui_profile)}
                  mono
                />

                <div className="svc-admin-operator-truth-flags">
                  <TruthFlag
                    label="Headless mode"
                    value={status.headless_mode}
                  />
                  <TruthFlag
                    label="Admin UI enabled"
                    value={status.admin_ui_enabled}
                  />
                  <TruthFlag
                    label="UI runtime required"
                    value={status.admin_ui_runtime_required}
                  />
                  <TruthFlag
                    label="Content serving"
                    value={status.content_serving_enabled}
                  />
                </div>

                {uiRuntimeViolation && (
                  <div
                    className="svc-admin-operator-truth-notice"
                    data-tone="danger"
                  >
                    The backend reports that node runtime requires the admin
                    UI. A Service Node must remain CLI and headless operable.
                  </div>
                )}
              </div>

              <div className="svc-admin-operator-truth-subsection">
                <h4>Quorum and replay posture</h4>

                <div className="svc-admin-operator-truth-flags">
                  <TruthFlag
                    label="Service quorum enabled"
                    value={status.service_quorum_enabled}
                  />
                  <TruthFlag
                    label="Wallet execution participant"
                    value={status.wallet_execution_participant}
                  />
                  <TruthFlag
                    label="Economic replay"
                    value={status.economic_replay_enabled}
                  />
                  <TruthFlag
                    label="Ledger replay"
                    value={status.ledger_replay_enabled}
                  />
                </div>

                <div className="svc-admin-operator-truth-notice">
                  Quorum participation is not protocol-earned eligibility,
                  an accepted epoch, payout authority, or ledger finality.
                </div>
              </div>

              <div className="svc-admin-operator-truth-subsection">
                <h4>Privacy publication posture</h4>

                <TruthRow
                  label="User IP publication"
                  value={textValue(status.user_ip_publication)}
                  mono
                />
                <TruthRow
                  label="Peer IP display"
                  value={textValue(status.peer_ip_display)}
                  mono
                />
                <TruthRow
                  label="Service socket publication"
                  value={textValue(
                    status.service_socket_publication,
                  )}
                  mono
                />

                <div className="svc-admin-operator-truth-flags">
                  <TruthFlag
                    label="Privacy mode"
                    value={status.privacy_mode}
                  />
                  <TruthFlag
                    label="Public inbound"
                    value={status.public_inbound_enabled}
                  />
                  <TruthFlag
                    label="Admin bind publication"
                    value={status.admin_bind_publication}
                  />
                  <TruthFlag
                    label="Transport routes public"
                    value={status.transport_routes_public}
                  />
                  <TruthFlag
                    label="Raw socket publication"
                    value={status.raw_socket_publication}
                  />
                </div>

                {status.privacy_mode === false && (
                  <div
                    className="svc-admin-operator-truth-notice"
                    data-tone="danger"
                  >
                    The backend reports privacy mode as disabled. Review
                    node posture before treating this status as privacy
                    compliant.
                  </div>
                )}

                {directPublicationRisk && (
                  <div
                    className="svc-admin-operator-truth-notice"
                    data-tone="danger"
                  >
                    The backend reports direct administrative, transport,
                    or raw-socket publication. This conflicts with the
                    private operator-console posture and requires review.
                  </div>
                )}
              </div>
            </>
          ) : (
            <Unavailable label="Runtime, quorum, and privacy posture" />
          )}
        </TruthCard>

        <TruthCard
          title="Accounting, reward plan and epoch transition"
          subtitle="Canonical report-only economic references. This operator view does not execute payouts or mutate the wallet or ledger."
        >
          {economic ? (
            <>
              <TruthRow
                label="Reported stage"
                value={textValue(economic.stage)}
                mono
              />

              <div className="svc-admin-operator-truth-subsection">
                <h4>Sealed accounting snapshot</h4>

                <TruthRow
                  label="Chain ID"
                  value={textValue(
                    economic.accounting_snapshot.chain_id,
                  )}
                  mono
                />
                <TruthRow
                  label="Snapshot ID"
                  value={textValue(
                    economic.accounting_snapshot.snapshot_id,
                  )}
                  mono
                />
                <TruthRow
                  label="Snapshot root"
                  value={textValue(
                    economic.accounting_snapshot.snapshot_root,
                  )}
                  mono
                />
                <TruthRow
                  label="Window started"
                  value={unixMsValue(
                    economic.accounting_snapshot
                      .window_started_at_ms,
                  )}
                  mono
                />
                <TruthRow
                  label="Window ended"
                  value={unixMsValue(
                    economic.accounting_snapshot
                      .window_ended_at_ms,
                  )}
                  mono
                />
                <TruthRow
                  label="Sealed at"
                  value={unixMsValue(
                    economic.accounting_snapshot.sealed_at_ms,
                  )}
                  mono
                />

                <div className="svc-admin-operator-truth-flags">
                  <TruthRow
                    label="Source events"
                    value={numberValue(
                      economic.accounting_snapshot
                        .source_event_count,
                    )}
                  />
                  <TruthRow
                    label="Economic receipts observed"
                    value={numberValue(
                      economic.accounting_snapshot
                        .economic_receipt_count,
                    )}
                  />
                  <TruthRow
                    label="Metering records"
                    value={numberValue(
                      economic.accounting_snapshot
                        .metering_count,
                    )}
                  />
                  <TruthRow
                    label="Proof eligible"
                    value={numberValue(
                      economic.accounting_snapshot
                        .proof_eligible_count,
                    )}
                  />
                  <TruthRow
                    label="Ad budgeted"
                    value={numberValue(
                      economic.accounting_snapshot
                        .ad_budgeted_count,
                    )}
                  />
                  <TruthRow
                    label="Analytics only"
                    value={numberValue(
                      economic.accounting_snapshot
                        .analytics_only_count,
                    )}
                  />
                </div>
              </div>

              <div className="svc-admin-operator-truth-subsection">
                <h4>Reward-plan reference</h4>

                {economic.reward_plan ? (
                  <>
                    <TruthRow
                      label="Plan ID"
                      value={textValue(
                        economic.reward_plan.plan_id,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Plan root"
                      value={textValue(
                        economic.reward_plan.plan_root,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Snapshot ID"
                      value={textValue(
                        economic.reward_plan.snapshot_id,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Snapshot root"
                      value={textValue(
                        economic.reward_plan.snapshot_root,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Source event class"
                      value={textValue(
                        economic.reward_plan
                          .source_event_class,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Planned total minor units"
                      value={textValue(
                        economic.reward_plan
                          .planned_total_minor,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Payout candidates"
                      value={numberValue(
                        economic.reward_plan
                          .payout_candidate_count,
                      )}
                    />
                    <TruthRow
                      label="Verification reference"
                      value={textValue(
                        economic.reward_plan
                          .verification_ref,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Funding-budget reference"
                      value={textValue(
                        economic.reward_plan
                          .funding_budget_ref,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Produced at"
                      value={unixMsValue(
                        economic.reward_plan.produced_at_ms,
                      )}
                      mono
                    />

                    <div className="svc-admin-operator-truth-flags">
                      <TruthFlag
                        label="Policy cap applied"
                        value={
                          economic.reward_plan
                            .capped_by_policy
                        }
                      />
                    </div>
                  </>
                ) : (
                  <Unavailable label="Reward-plan reference" />
                )}
              </div>

              <div className="svc-admin-operator-truth-subsection">
                <h4>Epoch-transition reference</h4>

                {economic.epoch_transition ? (
                  <>
                    <TruthRow
                      label="Chain ID"
                      value={textValue(
                        economic.epoch_transition.chain_id,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Epoch ID"
                      value={textValue(
                        economic.epoch_transition.epoch_id,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Transition hash"
                      value={textValue(
                        economic.epoch_transition
                          .transition_hash,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Accounting snapshot hash"
                      value={textValue(
                        economic.epoch_transition
                          .accounting_snapshot_hash,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Reward-plan hash"
                      value={textValue(
                        economic.epoch_transition
                          .reward_plan_hash,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Policy hash"
                      value={textValue(
                        economic.epoch_transition.policy_hash,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Economics-config hash"
                      value={textValue(
                        economic.epoch_transition
                          .economics_config_hash,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Registry root"
                      value={textValue(
                        economic.epoch_transition
                          .registry_root,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Reward-binding root"
                      value={textValue(
                        economic.epoch_transition
                          .reward_binding_root,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Evidence root"
                      value={textValue(
                        economic.epoch_transition
                          .evidence_root,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Reward cap minor units"
                      value={textValue(
                        economic.epoch_transition
                          .reward_cap_minor_units,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Reward total minor units"
                      value={textValue(
                        economic.epoch_transition
                          .reward_total_minor_units,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Allocations"
                      value={numberValue(
                        economic.epoch_transition
                          .allocation_count,
                      )}
                    />
                    <TruthRow
                      label="Eligible Service Nodes"
                      value={numberValue(
                        economic.epoch_transition
                          .eligible_service_node_count,
                      )}
                    />
                    <TruthRow
                      label="Required signature references"
                      value={numberValue(
                        economic.epoch_transition
                          .required_signature_references,
                      )}
                    />
                    <TruthRow
                      label="Supplied signature references"
                      value={numberValue(
                        economic.epoch_transition
                          .supplied_signature_references,
                      )}
                    />
                    <TruthRow
                      label="Produced at"
                      value={unixMsValue(
                        economic.epoch_transition
                          .produced_at_ms,
                      )}
                      mono
                    />

                    <div className="svc-admin-operator-truth-flags">
                      <TruthFlag
                        label="Reference threshold met"
                        value={
                          economic.epoch_transition
                            .quorum_reference_threshold_met
                        }
                      />
                      <TruthFlag
                        label="Cryptographic signatures verified"
                        value={
                          economic.epoch_transition
                            .cryptographic_signatures_verified
                        }
                      />
                      <TruthFlag
                        label="Recipient accounts resolved"
                        value={
                          economic.epoch_transition
                            .recipient_accounts_resolved
                        }
                      />
                    </div>
                  </>
                ) : (
                  <Unavailable label="Epoch-transition reference" />
                )}
              </div>

              <div className="svc-admin-operator-truth-subsection">
                <h4>Execution and finality boundary</h4>

                {economic.epoch_payout_receipts ? (
                  <>
                    <TruthRow
                      label="Wallet source"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .wallet_source,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Ledger source"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .ledger_source,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Settlement status"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .settlement_status,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Finality status"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .finality_status,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Accepted receipts"
                      value={numberValue(
                        economic.epoch_payout_receipts
                          .receipt_count,
                      )}
                    />
                    <TruthRow
                      label="Recipient count"
                      value={numberValue(
                        economic.epoch_payout_receipts
                          .recipient_count,
                      )}
                    />
                    <TruthRow
                      label="Issued ROC minor units"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .total_issued_minor,
                      )}
                      mono
                    />
                    <TruthRow
                      label="First ledger sequence"
                      value={numberValue(
                        economic.epoch_payout_receipts
                          .first_ledger_seq,
                      )}
                    />
                    <TruthRow
                      label="Last ledger sequence"
                      value={numberValue(
                        economic.epoch_payout_receipts
                          .last_ledger_seq,
                      )}
                    />
                    <TruthRow
                      label="Ledger root"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .ledger_root,
                      )}
                      mono
                    />
                    <TruthRow
                      label="First receipt hash"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .first_receipt_hash,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Last receipt hash"
                      value={textValue(
                        economic.epoch_payout_receipts
                          .last_receipt_hash,
                      )}
                      mono
                    />
                    <TruthRow
                      label="Accepted at"
                      value={unixMsValue(
                        economic.epoch_payout_receipts
                          .accepted_at_ms,
                      )}
                      mono
                    />

                    <div className="svc-admin-operator-truth-notice">
                      These are canonical accepted svc-wallet and
                      ron-ledger receipt references. Recipient account
                      IDs and per-recipient balances are intentionally
                      omitted from the operator projection.
                    </div>
                  </>
                ) : (
                  <Unavailable label="Canonical epoch-payout receipts" />
                )}

                <div className="svc-admin-operator-truth-flags">
                  <TruthFlag
                    label="Wallet execution reported"
                    value={economic.wallet_execution_reported}
                  />
                  <TruthFlag
                    label="Ledger receipt reported"
                    value={economic.ledger_receipt_reported}
                  />
                  <TruthFlag
                    label="Confirmed ROC reported"
                    value={economic.confirmed_roc_reported}
                  />
                  <TruthFlag
                    label="Finality reported"
                    value={economic.finality_reported}
                  />
                  <TruthFlag
                    label="UI economic authority"
                    value={
                      economic
                        .operator_projection_authorizes_economic_mutation
                    }
                  />
                </div>

                {economic
                  .operator_projection_authorizes_economic_mutation && (
                  <div
                    className="svc-admin-operator-truth-notice"
                    data-tone="danger"
                  >
                    Invalid operator posture: the UI projection must
                    never authorize wallet or ledger mutation.
                  </div>
                )}

                {economic.epoch_payout_receipts ? (
                  <div className="svc-admin-operator-truth-notice">
                    Confirmed ROC here means backend-accepted ledger
                    issuance evidence. It is not epoch finality,
                    external anchoring, ROX settlement, or Solana
                    finality.
                  </div>
                ) : (
                  <div className="svc-admin-operator-truth-notice">
                    Reward planning is not payout execution. Meeting a
                    quorum-reference threshold is not cryptographic
                    signature verification. Allocation IDs are not
                    resolved wallet recipients. No wallet receipt,
                    durable ledger receipt, confirmed ROC, balance, or
                    finality exists unless its canonical owner reports
                    that later stage explicitly.
                  </div>
                )}
              </div>
            </>
          ) : (
            <Unavailable label="Canonical economic pipeline status" />
          )}
        </TruthCard>

        <TruthCard
          title="Eligibility, containment and appeal"
          subtitle="Canonical lifecycle state, quorum posture, effective epochs, and Phase 19 containment."
        >
          {lifecycle ? (
            <>
              <TruthRow
                label="Lifecycle state"
                value={textValue(lifecycle.lifecycle_state)}
                mono
              />
              <TruthRow
                label="Registered epoch"
                value={numberValue(
                  lifecycle.registered_at_epoch,
                )}
              />
              <TruthRow
                label="State effective epoch"
                value={numberValue(
                  lifecycle.state_effective_epoch,
                )}
              />
              <TruthRow
                label="Quorum status"
                value={textValue(lifecycle.quorum_status)}
                mono
              />

              <div className="svc-admin-operator-truth-flags">
                <TruthFlag
                  label="Counts toward quorum"
                  value={lifecycle.counts_toward_quorum}
                />
                <TruthFlag
                  label="Probation cap required"
                  value={
                    lifecycle.probation_reward_cap_required
                  }
                />
                <TruthFlag
                  label="UI may change state"
                  value={
                    lifecycle
                      .operator_projection_authorizes_state_change
                  }
                />
                <TruthFlag
                  label="UI economic authority"
                  value={
                    lifecycle
                      .operator_projection_authorizes_economic_mutation
                  }
                />
              </div>

              {lifecycle.enforcement ? (
                <div className="svc-admin-operator-truth-subsection">
                  <h4>Containment</h4>

                  <TruthRow
                    label="Status ID"
                    value={textValue(
                      lifecycle.enforcement.status_id,
                    )}
                    mono
                  />
                  <TruthRow
                    label="State"
                    value={textValue(
                      lifecycle.enforcement.state,
                    )}
                    mono
                  />
                  <TruthRow
                    label="Reason"
                    value={textValue(
                      lifecycle.enforcement.reason,
                    )}
                    mono
                  />
                  <TruthRow
                    label="Evidence root"
                    value={textValue(
                      lifecycle.enforcement.evidence_root,
                    )}
                    mono
                  />
                  <TruthRow
                    label="Effective epoch"
                    value={numberValue(
                      lifecycle.enforcement.effective_epoch,
                    )}
                  />

                  <div className="svc-admin-operator-truth-flags">
                    <TruthFlag
                      label="Counts toward quorum"
                      value={
                        lifecycle.enforcement
                          .counts_toward_quorum
                      }
                    />
                    <TruthFlag
                      label="Reward planning permitted"
                      value={
                        lifecycle.enforcement
                          .permits_reward_planning
                      }
                    />
                    <TruthFlag
                      label="Economic mutation"
                      value={
                        lifecycle.enforcement
                          .authorizes_economic_mutation
                      }
                    />
                  </div>

                  <h4>Appeal</h4>

                  <TruthRow
                    label="Appeal state"
                    value={textValue(
                      lifecycle.enforcement.appeal.state,
                    )}
                    mono
                  />
                  <TruthRow
                    label="Appeal ID"
                    value={textValue(
                      lifecycle.enforcement.appeal.appeal_id,
                    )}
                    mono
                  />
                  <TruthRow
                    label="Submitted epoch"
                    value={numberValue(
                      lifecycle.enforcement.appeal
                        .submitted_epoch,
                    )}
                  />
                  <TruthRow
                    label="Resolved epoch"
                    value={numberValue(
                      lifecycle.enforcement.appeal
                        .resolved_epoch,
                    )}
                  />
                  <TruthRow
                    label="Resolution evidence"
                    value={textValue(
                      lifecycle.enforcement.appeal
                        .resolution_evidence_root,
                    )}
                    mono
                  />

                  <div className="svc-admin-operator-truth-flags">
                    <TruthFlag
                      label="Pending"
                      value={
                        lifecycle.enforcement.appeal.pending
                      }
                    />
                    <TruthFlag
                      label="Appeal changes state"
                      value={
                        lifecycle.enforcement.appeal
                          .authorizes_state_change
                      }
                    />
                  </div>
                </div>
              ) : (
                <div className="svc-admin-operator-truth-notice">
                  No canonical containment status is attached to
                  this lifecycle descriptor.
                </div>
              )}

              <div className="svc-admin-operator-truth-notice">
                Quorum eligibility is protocol-earned lifecycle
                posture. It is not an accepted epoch signature,
                reward plan, payout approval, wallet receipt,
                ledger receipt, confirmed ROC, or finality.
              </div>
            </>
          ) : (
            <Unavailable label="Service Node lifecycle status" />
          )}
        </TruthCard>

        <TruthCard
          title="Protocol and provider"
          subtitle="OAP integrity and provider-advertisement posture."
        >
          <div className="svc-admin-operator-truth-subsection">
            <h4>OAP runtime</h4>

            {oap ? (
              <>
                <TruthRow
                  label="Protocol"
                  value={`${oap.protocol} v${oap.version}`}
                  mono
                />
                <TruthRow
                  label="Runtime state"
                  value={textValue(oap.runtime_state)}
                  mono
                />
                <TruthRow
                  label="Maximum frame"
                  value={bytesValue(oap.max_frame_bytes)}
                />
                <TruthRow
                  label="Stream chunk"
                  value={bytesValue(oap.stream_chunk_bytes)}
                />

                <div className="svc-admin-operator-truth-flags">
                  <TruthFlag
                    label="Object fetch active"
                    value={oap.object_fetch_active}
                  />
                  <TruthFlag
                    label="Full digest verification"
                    value={oap.full_digest_verification_active}
                  />
                </div>
              </>
            ) : (
              <Unavailable label="OAP status" />
            )}
          </div>

          <div className="svc-admin-operator-truth-subsection">
            <h4>Provider publication</h4>

            {provider ? (
              <>
                <TruthRow
                  label="State"
                  value={textValue(provider.state)}
                  mono
                />
                <TruthRow
                  label="DHT worker"
                  value={textValue(provider.dht_worker_status)}
                  mono
                />
                <TruthRow
                  label="Published records"
                  value={provider.provider_records_published.toLocaleString()}
                />
                <TruthRow
                  label="Public URI format"
                  value={textValue(provider.public_node_uri_format)}
                  mono
                />

                <div className="svc-admin-operator-truth-flags">
                  <TruthFlag
                    label="Advertisement active"
                    value={provider.advertisement_active}
                  />
                  <TruthFlag
                    label="Residential IP publication"
                    value={provider.residential_ip_publication}
                  />
                </div>

                {provider.residential_ip_publication && (
                  <div
                    className="svc-admin-operator-truth-notice"
                    data-tone="danger"
                  >
                    The backend reports residential IP publication as active.
                    Review the node privacy configuration.
                  </div>
                )}
              </>
            ) : (
              <Unavailable label="Provider status" />
            )}
          </div>
        </TruthCard>

        <TruthCard
          title="Moderation and quarantine"
          subtitle="Serve-policy enforcement, signed-policy freshness, and bounded counts."
        >
          {policy ? (
            <>
              <TruthRow
                label="State"
                value={textValue(policy.state)}
                mono
              />
              <TruthRow
                label="Moderation state"
                value={textValue(policy.moderation_state)}
                mono
              />
              <TruthRow
                label="Source"
                value={textValue(policy.moderation_source)}
                mono
              />
              <TruthRow
                label="Activation"
                value={textValue(policy.moderation_activation)}
                mono
              />
              <TruthRow
                label="Signed epoch"
                value={numberValue(policy.signed_policy_epoch)}
              />
              <TruthRow
                label="Signed policy expires"
                value={unixValue(
                  policy.signed_policy_expires_at_unix_s,
                )}
                mono
              />
              <TruthRow
                label="Persistence posture"
                value={textValue(
                  policy.unvetted_persistence_posture,
                )}
                mono
              />
              <TruthRow
                label="Serve gate phase"
                value={textValue(policy.serve_gate_phase)}
                mono
              />
              <TruthRow
                label="Moderation phase"
                value={textValue(policy.moderation_phase)}
                mono
              />

              <div className="svc-admin-operator-truth-flags">
                <TruthFlag
                  label="Moderation configured"
                  value={policy.moderation_configured}
                />
                <TruthFlag
                  label="Serve policy enforced"
                  value={policy.serve_policy_enforced}
                />
                <TruthFlag
                  label="OAP policy enforced"
                  value={policy.oap_serve_policy_enforced}
                />
                <TruthFlag
                  label="Operator moderation"
                  value={policy.operator_moderation_active}
                />
                <TruthFlag
                  label="Global moderation"
                  value={policy.global_moderation_active}
                />
                <TruthFlag
                  label="Signed policy verified"
                  value={policy.signed_policy_verified}
                />
                <TruthFlag
                  label="Rollback guard persisted"
                  value={policy.rollback_guard_persisted}
                />
                <TruthFlag
                  label="Hot reload"
                  value={policy.moderation_hot_reload}
                />
              </div>

              <div className="svc-admin-operator-truth-counts">
                <div>
                  <span>Total</span>
                  <strong>{policy.moderation_entries.total}</strong>
                </div>
                <div>
                  <span>Global deny</span>
                  <strong>
                    {policy.moderation_entries.global_deny}
                  </strong>
                </div>
                <div>
                  <span>Local block</span>
                  <strong>
                    {policy.moderation_entries.local_block}
                  </strong>
                </div>
                <div>
                  <span>Local allow</span>
                  <strong>
                    {policy.moderation_entries.local_allow}
                  </strong>
                </div>
                <div>
                  <span>Tombstones</span>
                  <strong>
                    {policy.moderation_entries.owner_tombstone}
                  </strong>
                </div>
                <div>
                  <span>Quarantine</span>
                  <strong>
                    {policy.moderation_entries.quarantine}
                  </strong>
                </div>
              </div>

              {policy.moderation_load_failed && (
                <div
                  className="svc-admin-operator-truth-notice"
                  data-tone="danger"
                >
                  Moderation policy loading failed. Treat serving posture as
                  unsafe until the backend reports a successfully loaded
                  policy.
                </div>
              )}

              {policy.moderation_configured &&
                !policy.signed_policy_verified && (
                  <div
                    className="svc-admin-operator-truth-notice"
                    data-tone="warning"
                  >
                    Moderation is configured, but the backend does not report
                    a verified signed policy.
                  </div>
                )}

              {signedPolicyExpired && (
                <div
                  className="svc-admin-operator-truth-notice"
                  data-tone="danger"
                >
                  The reported signed-policy expiry is in the past.
                </div>
              )}
            </>
          ) : (
            <Unavailable label="Moderation policy status" />
          )}
        </TruthCard>

        <TruthCard
          title="Review and persistence queues"
          subtitle="Process-local eligibility metadata and completed local prune operations."
        >
          {persistence ? (
            <>
              <TruthRow
                label="State"
                value={textValue(persistence.state)}
                mono
              />

              <div className="svc-admin-operator-truth-counts">
                <div>
                  <span>Candidates</span>
                  <strong>{persistence.candidates_total}</strong>
                </div>
                <div>
                  <span>Awaiting decision</span>
                  <strong>{persistence.awaiting_decision}</strong>
                </div>
                <div>
                  <span>Pending review</span>
                  <strong>{persistence.pending_review}</strong>
                </div>
                <div>
                  <span>Approvals</span>
                  <strong>{persistence.persistence_approvals}</strong>
                </div>
                <div>
                  <span>Blocked</span>
                  <strong>{persistence.blocked_candidates}</strong>
                </div>
                <div>
                  <span>Quarantined</span>
                  <strong>{persistence.quarantined_candidates}</strong>
                </div>
                <div>
                  <span>Local prunes</span>
                  <strong>{persistence.completed_local_prunes}</strong>
                </div>
              </div>

              <div className="svc-admin-operator-truth-flags">
                <TruthFlag
                  label="Durable bytes written"
                  value={persistence.durable_bytes_written}
                />
                <TruthFlag
                  label="Reward finality"
                  value={persistence.reward_finality}
                />
                <TruthFlag
                  label="Wallet mutation"
                  value={persistence.wallet_mutation}
                />
                <TruthFlag
                  label="Ledger mutation"
                  value={persistence.ledger_mutation}
                />
              </div>

              <div className="svc-admin-operator-truth-notice">
                Persistence approval means policy eligibility only. It does
                not prove durable byte storage, reward approval, payout,
                wallet mutation, ledger mutation, or finality.
              </div>

              {persistenceReportsAuthority && (
                <div
                  className="svc-admin-operator-truth-notice"
                  data-tone="warning"
                >
                  This status reports authority beyond process-local review
                  metadata. Verify it through the canonical storage,
                  accounting, wallet, and ledger surfaces.
                </div>
              )}
            </>
          ) : (
            <Unavailable label="Persistence-review status" />
          )}
        </TruthCard>

        <TruthCard
          title="Reward-recipient binding"
          subtitle="Binding posture only; requested state is not registry finality or payout."
        >
          {binding ? (
            <>
              <TruthRow
                label="State"
                value={textValue(binding.state)}
                mono
              />
              <TruthRow
                label="Recipient"
                value={textValue(
                  binding.reward_recipient_display_address,
                )}
                mono
              />
              <TruthRow
                label="Pending rotation"
                value={textValue(
                  binding.pending_rotation_display_address,
                )}
                mono
              />
              <TruthRow
                label="Updated"
                value={unixValue(binding.updated_at_unix_s)}
                mono
              />
              <TruthRow
                label="Reported confirmed ROC"
                value={numberValue(
                  binding.confirmed_roc_minor_units,
                )}
              />

              <div className="svc-admin-operator-truth-flags">
                <TruthFlag
                  label="Registry finality"
                  value={binding.registry_finality}
                />
                <TruthFlag
                  label="Wallet mutation"
                  value={binding.wallet_mutation}
                />
                <TruthFlag
                  label="Ledger mutation"
                  value={binding.ledger_mutation}
                />
              </div>

              <div className="svc-admin-operator-truth-notice">
                A runtime-local or requested binding is not a confirmed
                reward, durable receipt, or registry-final binding.
              </div>

              {bindingReportsMutation && (
                <div
                  className="svc-admin-operator-truth-notice"
                  data-tone="warning"
                >
                  The status reports wallet or ledger mutation. Verify the
                  canonical wallet receipt and ledger replay before relying
                  on this display.
                </div>
              )}
            </>
          ) : (
            <Unavailable label="Reward-binding status" />
          )}
        </TruthCard>

        <TruthCard
          title="Service evidence"
          subtitle="Outbox posture; queued records are evidence, not ROC."
        >
          {evidence ? (
            <>
              <TruthRow
                label="State"
                value={textValue(evidence.state)}
                mono
              />
              <TruthRow
                label="Queued records"
                value={evidence.queued_records.toLocaleString()}
              />
              <TruthRow
                label="Delivery evidence"
                value={evidence.delivery_records.toLocaleString()}
              />
              <TruthRow
                label="Reward-eligible evidence"
                value={evidence.reward_evidence_records.toLocaleString()}
              />
              <TruthRow
                label="Replay scope"
                value={textValue(evidence.replay_scope)}
                mono
              />

              <div className="svc-admin-operator-truth-flags">
                <TruthFlag
                  label="Signature required"
                  value={evidence.signature_required}
                />
                <TruthFlag
                  label="Durable"
                  value={evidence.durable}
                />
                <TruthFlag
                  label="Accounting accepted"
                  value={evidence.accounting_accepted}
                />
                <TruthFlag
                  label="Reward eligible"
                  value={evidence.reward_eligible}
                />
                <TruthFlag
                  label="Reward truth"
                  value={evidence.reward_truth}
                />
                <TruthFlag
                  label="Payout authority"
                  value={evidence.payout_authority}
                />
                <TruthFlag
                  label="Wallet mutation"
                  value={evidence.wallet_mutation}
                />
                <TruthFlag
                  label="Ledger mutation"
                  value={evidence.ledger_mutation}
                />
              </div>

              <div className="svc-admin-operator-truth-notice">
                Queued evidence is not a reward plan, payout approval, wallet
                receipt, ledger receipt, or confirmed ROC.
              </div>

              {evidenceReportsAuthority && (
                <div
                  className="svc-admin-operator-truth-notice"
                  data-tone="warning"
                >
                  This status reports downstream economic authority. Confirm
                  it through the canonical accounting, policy, wallet, and
                  ledger surfaces.
                </div>
              )}
            </>
          ) : (
            <Unavailable label="Service-evidence status" />
          )}
        </TruthCard>
      </div>
    </section>
  )
}
