# Contract Impact Grade Tightening Design

## Goal

Make `A` a rare, auditable major-impact grade instead of the fallback bucket for ordinary high-multiple contract-flow events. The same canonical grade must drive the operator page, Discord eligibility, and retention semantics.

## Current Problem

The detector currently passes `dynamic_multiple` as both impact score and z-score. The raw percentile/multiple classifier therefore marks many events as `S`; the existing hard-evidence sanitizer then downgrades those events to `A`. In the live seven-day BTC sample, the newest eight canonical `A` events all had degraded evidence, no multi-exchange confirmation, and zero live liquidation. The page-cohort calculation independently rated those events `C`, proving that the canonical grade was overstating the business significance.

## Decision

Keep the existing `C/B/A/S` vocabulary and canonical detector fields for compatibility, but apply a conservative post-classification gate before persistence:

- `S` remains available only when the existing replayable hard-evidence test passes.
- `A` requires data quality at least 80, percentile at least 99.5, impact multiple and z-score at least 4, absolute price move at least 0.5%, an absolute scale floor of 2,500 BTC or 150M USD, and independent confirmation from multiple exchanges, confirmed behavior, or live liquidation evidence.
- A raw `A` that fails the major-event gate is downgraded to `B` when it still meets the material-event evidence floor (data quality at least 70, percentile at least 99, impact multiple and z-score at least 2.5, absolute price move at least 0.15%, and an absolute scale floor of 800 BTC or 50M USD); otherwise it becomes `C`.
- The downgrade updates `impactLevel`, `signalLevel`, and `signalLabel` together, so Discord and retention see the same grade as the page. API projection re-applies the sanitizer to persisted signals before building `FinalEvent`, then copies the canonical triplet to the flattened event fields; historical rows reflect the tightened policy without rewriting database records.
- No trading, signing, account, or deletion path is added. Existing read-only and dry-run gates remain unchanged.

## Data Flow

`market_impact_normalization` produces the legacy raw candidate. `sanitize_contract_whale_impact` applies the hard-evidence S gate and the new A/B/C gate on the fully populated `ContractWhaleSignal`. The detector persists and emits the sanitized fields; final-event projection re-applies the sanitizer to persisted rows before constructing flattened events, and API normalization, UI rendering, Discord gates, and retention consume those canonical fields.

## Testing

Add boundary tests for a raw A that is downgraded to C, a material event that becomes B, a fully confirmed major event that stays A, and an existing hard-evidence S that remains S. Keep tests for raw S without hard evidence downgrading safely. Run the focused Rust suite, frontend contract-whale tests, build, and read-only live health/grade checks after synchronization.

## Rollout and Rollback

The change is backward-compatible at the JSON field level and is reversible by reverting the single application commit. Deployment is limited to the backend/frontend services after local verification. No database rewrite or retention deletion is performed during this rollout.
