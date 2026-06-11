# Event Classification and Rewards

RO:WHAT — QC-0A event-class and reward-safety rules.
RO:WHY — Prevent fakeable engagement metrics from becoming protocol ROC payout truth.
RO:INTERACTS — ron-accounting, svc-rewarder, svc-wallet, ron-policy, future QuickChain accounting/reward roots.
RO:INVARIANTS — accounting is not balance truth; rewarder does not mutate ledger; raw engagement does not directly mint or allocate protocol ROC.
RO:METRICS — future raw engagement reward reject counters.
RO:CONFIG — future reward feature gates; raw engagement reward remains disabled.
RO:SECURITY — reward formulas must identify funding source and reject ambiguous event classes.
RO:TEST — future raw_engagement_reward_formula_rejected.

## Event classes

```text
economic_receipt:
  payer-authorized wallet event or ledger mutation.
  Can affect balances only through svc-wallet/ron-ledger.

metering:
  usage measurement.
  Cannot mutate balances.

proof_eligible:
  storage/provider/carrier/archive proof candidate.
  Requires verification/challenge before reward planning.

ad_budgeted:
  advertiser/sponsor-funded event.
  Paid from explicit budget, not raw protocol mint.

analytics_only:
  display/reporting signal.
  Never enters protocol ROC reward allocation.
```

## Forbidden reward bases

```text
total_site_visits share
raw_watch_seconds share
raw_ad_impressions share
raw_click_count share
unverified active user share
```

## Safer reward bases

```text
payer-authorized purchases
advertiser-funded campaign budget with fraud scoring
storage proofs with challenge verification
provider egress proofs with bounded accounting
governance-approved grants with hard caps
creator-defined payout splits from explicit revenue
```

## Required reward manifest rules

```text
funding source required
policy hash required
input accounting root required
deterministic payout plan required
wallet commits payout
rewarder does not mutate balances
same epoch cannot double issue
```

## No-go

Do not implement reward roots until event-class enforcement and forbidden reward formula tests exist.
