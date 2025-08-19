---
title: RustyOnions — Token Design Specification (ROC/ROX)
subtitle: Version 0.4 • Draft for Testnet
author: RustyOnions Project
date: August 18, 2025
---

# RustyOnions — Token Design Specification (ROC/ROX)

**Version 0.4 • Draft for Testnet • © 2025 RustyOnions Project**

## 1. Purpose & Philosophy

*Unchanged from Version 0.3*: RustyOnions introduces a two-token model (ROC for internal settlement, ROX for external liquidity) to align incentives around useful work (bandwidth, storage, compute). Tokens meter and settle services with privacy, fairness, and low cost, rejecting speculation-first economics. Core principles include no premine, Proof-of-Useful-Service (PoUS), closed-loop mint/burn, privacy-by-design, and minimal governance (M-of-N witnesses, no treasury).

## 2. Token Overview

*Unchanged from Version 0.3*: RustyOnions uses ROC (RustyOnions Credits) for internal settlement (non-transferable, used for TLD services like .music, .video) and ROX (RustyOnions Exchange) for external liquidity (tradable Solana SPL token). ROC is minted via PoUS and burned on consumption or ROC→ROX conversion. ROX is minted/burned via conversions.

| Unit | Where it Lives        | Purpose             | Transferability | How Minted           | How Burned                     |
|------|-----------------------|---------------------|-----------------|----------------------|-------------------------------|
| ROC  | RustyOnions overlay   | Pay/earn for services | Internal only  | PoUS receipts        | On consumption + ROC→ROX wrap |
| ROX  | Solana SPL            | External liquidity  | Tradable        | ROC→ROX conversion   | ROX tx burn + ROX→ROC unwrap  |

## 3. Utility-Anchored Pricing

*Updated*: ROC is priced in service units (e.g., 0.1 ROC/MB for .music streaming) via a versioned Service Price Table (SPT) stored on-chain under the `.alg` policy PDA, updated by 3-of-5 witness nodes. Arbitrage ties ROX to ROC’s utility value, stabilizing prices. Transparency is enhanced through a public dashboard showing SPT prices, ROC mint/burn, and ROX staking stats.

**Example SPT Entries**:

.map.bandwidth_mb      = 0.05  # ROC per MB transferred
.image.storage_gb_mo   = 2.00  # ROC per GB·month stored
.ai.compute_sec        = 0.20  # ROC per compute-second
.music.stream_mb       = 0.10  # ROC per MB streamed (media tier = 30d escrow)


**Updates**:
- SPT updates require 3-of-5 witness signatures for security.
- Dashboard integration (Milestone 3) displays real-time pricing and network metrics to deter speculation.

## 4. Conversion Bridge (ROX ↔ ROC)

*Updated*: Conversions are proof-based, ensuring trustless ROC↔ROX exchange. ROX is burned on Solana, verified by watchers, and ROC is minted in the overlay. ROC→ROX burns ROC, produces a proof, and mints ROX. The updated fee structure increases burns to enhance ROX scarcity and fund staking rewards.

### 4.1 ROX→ROC Flow (Proof-of-Burn)
1. **User Action (Client)**:
   - Prepare ROC recipient: `roc_pubkey`.
   - Sign message: `sig = Sign(roc_privkey, "bind:" + roc_pubkey)`.
   - Submit Solana transaction: `burn(amount: u64, memo: roc_pubkey || sig)`.
2. **On-Chain (Conversion Program)**:
   - Verify burn, emit event: `{wallet, amount, roc_pubkey, sig}`.
3. **Off-Chain (Watchers)**:
   - Verify `sig` binds `roc_pubkey`, reach overlay consensus, credit `amount` ROC to `Account(roc_pubkey)`.

### 4.2 ROC→ROX Flow (Proof-of-Burn)
1. **User Action (Client)**:
   - Request `withdraw(amount)`, overlay burns ROC, produces proof `P`.
2. **On-Chain (Conversion Program)**:
   - User submits `P`, verified against transparency roots.
   - Mint `amount` ROX to `user_wallet`.

### 4.3 Fees, Burn & Splits
**On ROC→ROX Conversion**:
- Burn: 2.0% (0.5% pure burn, 1.5% to staking/LP rewards; deflationary).
- Protocol Stake: 1.0% (yield for ROX stakers).
- LP Incentives: 0.5% (deferred until Milestone 4, pools mature).
- User Receives: 96.5% ROX.

**On ROX→ROC Conversion**:
- Fee: 0.2% (optional, discourages churn, adjustable by witnesses).

**Updates**:
- Increased burn to 2% to enhance ROX scarcity and fund staking.
- 3-of-5 witnesses can adjust burn rate (1.5–2.5%) based on network health (Milestone 3).

## 5. ROX Staking & Liquidity

*Updated*: ROX staking ensures liquidity by incentivizing holding without speculative hype, funded by real network activity (ROC→ROX conversions). Two tracks are offered: Protocol Staking (safer, early) and LP Staking (riskier, deferred). Yields are capped at 5% APR long-term, with a temporary boost to attract early adopters.

### 5.1 Protocol Staking (Milestone 2)
- Stake ROX for 14, 30, or 60 days (tiered yields: 3%, 5%, 7% APR for first 90 days; 5% cap thereafter).
- Rewards: 1.0% of ROC→ROX conversions, distributed weekly via StakingPool PDA.
- 7-day unbond period for flexibility.
- Parameters adjustable by 3-of-5 witnesses.

### 5.2 LP Staking (Milestone 4)
- Provide ROX-SOL/USDC liquidity on a DEX (e.g., Orca).
- Rewards: 0.5% of conversion flows + 0.5% of protocol fees from high-volume TLDs (.music, .video).
- Risk notice: Impermanent loss; M3 dashboard includes risk calculator.

### 5.3 Liquidity Bootstrapping
- Seed 10,000 ROX in M4 testnet pools (ROX-SOL, Orca) using ROC from incentivized services (2 ROC/GB .music streaming).
- Use bonding curve tied to SPT prices (e.g., 1 ROX ≈ 0.1 ROC/MB average).
- Time-locked releases (25% monthly) to prevent whale manipulation.

**Updates**:
- Flexible staking tiers (14/30/60 days) attract diverse holders.
- Temporary 7% APR boost for 90 days ensures early adoption.
- Capped pools and bonding curve stabilize ROX trading.

## 6. Escrow & Holdbacks

*Unchanged from Version 0.3*: Media TLDs (.music, .video, .image, .musicvideo) use 30-day escrow holdbacks; low-risk TLDs (.map, compute) use 7 days. Payouts split per TOML manifests, disputes resolved via Merkle proofs.

**Escrow Account (PDA)**:
- `creator: Pubkey`
- `amount: u64` (ROC)
- `created_at: i64`
- `manifest_hash: [u8;32]`
- `receipts_root: [u8;32]`
- `tier: "media" | "low-risk"`
- `released: bool`
- `witnesses: [Pubkey; 3]` (optional co-signs)

## 7. Receipts, Commitments & Transparency

*Updated*: Monetizable events generate signed receipts, batched into Merkle trees with hourly/daily roots published to a public transparency feed (overlay + optional on-chain). Hybrid storage (IPFS + Arweave) retains data ≥90 days post-settlement. The M3 dashboard displays real-time ROC mint/burn, ROX staking, and conversion stats to deter speculation.

**Receipt**:
- `content_hash: [u8;32]`
- `creator_id: [u8;32]`
- `units: u64`
- `ts: u64`
- `session_id: [u8;32]`
- `nonce: [u8;32]`
- `sig: [u8;64]`

**Updates**:
- Dashboard integration enhances transparency, showing network health metrics (e.g., “2% burn rate, 5% APR”).

## 8. Governance: Witnesses, Not Treasuries

*Updated*: A rotating set of 5 community witnesses (.mod/.alg operators) co-signs transparency checkpoints, updates SPT, and adjusts parameters (burn: 1.5–2.5%, staking: 1–2%) with 3-of-5 consensus. No protocol treasury or premine exists. Future community funds (e.g., for hackathons) are DAO-controlled, off the hot path.

**Updates**:
- Specified 3-of-5 consensus for parameter adjustments.
- Added hackathon support via DAO to drive adoption.

## 9. Security Model & Threats

*Unchanged from Version 0.3*: Security measures prevent double-minting (ROC issuance tied to ROX burns), replay attacks (burn events include ROC pubkey + signature), relayer abuse (stake + slashing), data loss (IPFS + Arweave), and privacy breaches (off-chain receipts, pseudonymous sessions).

## 10. Economics & Costing

*Updated*: Solana fees (~$0.001/signature at SOL ≈ $200) and batching keep per-event costs near-zero. Relayers recover ROC fees, earning staking yields. ROX’s 2% burn (0.5% pure) creates scarcity; staking yields (5–7% APR) are sustainable, funded by conversion fees.

| Parameter                | Default        | Notes                                  |
|--------------------------|----------------|----------------------------------------|
| ROX burn (ROC→ROX)       | 2.0%           | 0.5% pure, 1.5% to staking/LP         |
| Protocol staking reward   | 1.0%           | APR 5–7% (90 days), then 5% cap       |
| LP staking reward        | 0.5% (M4)      | Enabled when pools mature             |
| ROC→ROX fee (user)       | 96.5%          | After burn + rewards                  |
| ROX→ROC fee              | 0.2% (opt.)    | Discourages churn, witness-adjustable |
| Escrow holdback (media)  | 30 days        | DRM/dispute buffer                    |
| Escrow holdback (low-risk) | 7 days       | Faster liquidity                      |

**Updates**:
- Increased burn to 2% for stronger deflation.
- Added temporary 7% APR boost for staking.

## 11. Developer Interfaces (Anchor Sketches)

*Updated*: Split programs for modularity. Enhanced staking and conversion logic.

### 11.1 Conversion Program

```rust
#[program]
mod conversion {
    pub fn burn_rox_and_bind_roc(ctx, amount: u64, roc_pubkey: [u8;32], roc_sig: [u8;64]) -> Result<()> {
        // Verify ROX burn, roc_sig binds roc_pubkey
        // Emit event {wallet, amount, roc_pubkey, sig, slot}
        Ok(())
    }

    pub fn mint_rox_from_proof(ctx, amount: u64, proof: Proof) -> Result<()> {
        // Verify ROC burn proof against transparency root
        // Burn 2% ROC (0.5% pure, 1.5% to staking/LP)
        // Mint 96.5% ROX to user wallet
        Ok(())
    }
}



#[account]
pub struct Escrow {
    creator: Pubkey,
    amount: u64,
    created_at: i64,
    manifest_hash: [u8;32],
    receipts_root: [u8;32],
    tier: u8, // 0=low-risk, 1=media
    released: bool,
}

#[program]
mod settlement {
    pub fn init_escrow(ctx, creator: Pubkey, amount: u64, manifest_hash: [u8;32], root: [u8;32], tier: u8) -> Result<()> { Ok(()) }
    pub fn flag_dispute(ctx, proof: MerkleProof) -> Result<()> { Ok(()) }
    pub fn release(ctx) -> Result<()> { Ok(()) }
}
```

### 11.2 Escrow & Settlement Program

```rust

#[account]
pub struct Escrow {
    creator: Pubkey,
    amount: u64,
    created_at: i64,
    manifest_hash: [u8;32],
    receipts_root: [u8;32],
    tier: u8, // 0=low-risk, 1=media
    released: bool,
}

#[program]
mod settlement {
    pub fn init_escrow(ctx, creator: Pubkey, amount: u64, manifest_hash: [u8;32], root: [u8;32], tier: u8) -> Result<()> { Ok(()) }
    pub fn flag_dispute(ctx, proof: MerkleProof) -> Result<()> { Ok(()) }
    pub fn release(ctx) -> Result<()> { Ok(()) }
}

```

### 11.3 Staking & Registry

```rust

#[account]
pub struct StakingPool {
    total_staked: u64,
    reward_accum: u128,
    last_updated: i64,
    lock_period: u16, // 14, 30, 60 days
}

#[program]
mod staking {
    pub fn stake_rox(ctx, amount: u64, lock_days: u16) -> Result<()> {
        // Lock ROX, set lock period (14/30/60 days)
        Ok(())
    }
    pub fn unstake_rox(ctx, amount: u64) -> Result<()> {
        // Enforce 7-day unbond
        Ok(())
    }
    pub fn distribute_rewards(ctx) -> Result<()> {
        // Distribute reward_accum weekly
        Ok(())
    }
}

```

## 12. Manifests (TOML) — Attribution & Pricing
>Updated: Streamlined schema for flexibility across TLDs.

```rust

[manifest]
version = "1.0"
tld = ".music"
content_hash = "bafybeih..."  # IPFS CID
manifest_hash = "0xabc123..." # Hash of this manifest

[pricing]
service_type = "streaming"
rate = 0.10  # ROC per MB
currency = "ROC"

[attribution]
creator = { id = "crea_9f...", share = 0.60 }
owner = { id = "ownr_11...", share = 0.20 }
moderator = { id = "mod_33...", share = 0.10 }
service = { id = "node_8a...", share = 0.10 }

[escrow]
tier = "media"
holdback_days = 30
receipts_root = "0xdeadbeef..."

```

**Updates:**

- Added currency for future-proofing.
- Supports both public keys and PDAs for IDs.

## 13. UX & Ops

**Updated:**

- Client bundles ROC pubkey signature in Solana burn transactions.
- M3 dashboard shows ROC mint/burn, ROX staking (APR, lock periods), burn %, and liquidity depth.
- Witness rotation documented; 3-of-5 consensus, monthly cadence.
- M4 hackathon offers 100 ROC per valid .music/.video app to drive adoption.

