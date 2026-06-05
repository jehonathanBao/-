---
name: risk-control-business
description: Implement or review toxic-order and order-risk business rules including dedupe, manual review precedence, blacklist and whitelist priority, model evidence, and audit trails.
---

# Risk Control Business Rules

Use this skill for changes to risk scoring, alert generation, manual review, blacklist/whitelist handling, model evidence, or suggested actions.

## Decision Priority

1. Tenant/shop/user scope must be valid before any lookup or decision.
2. Manual release wins over automated block for the same order.
3. Whitelist wins over blacklist only when scoped to the same tenant/shop/user and explicitly documented.
4. Blacklist wins over general model/rule scores.
5. Hard safety rules win over soft model suggestions.
6. Low-risk model output must never trigger automatic blacklist, refund, block, or ban.
7. Refund, intercept, ban, or irreversible action requires manual review.

## Candidate Decision Contract

Every risk decision should carry:

- `order_id`
- `tenant_id`
- `shop_id`
- `user_id`
- `risk_score`
- `risk_level`
- `rule_hits`
- `model_reason`
- `suggested_action`
- `decision_source`
- `dedupe_key`
- `audit_ref`

For toxic L2/trade candidates, every operator-facing result should also carry:

- final direction bias
- core reason summary
- risk score
- data quality score or bucket
- detector/event type
- read-only or analysis-only marker

## Dedupe Rules

- Use a stable key such as `tenant_id:shop_id:order_id:risk_version`.
- Reprocessing the same order and rule version must not emit a second alert.
- A new rule version may emit a new candidate only when evidence or action changes.

## Alert Priority

- Critical and high risk candidates may be pinned and considered for external alerts.
- Medium candidates stay in the operator inbox and default folded.
- Low candidates stay hidden unless the operator explicitly filters for them.
- Candidate alerts must not be treated as enforcement decisions.

## Tests To Require

- Ordinary order remains low risk.
- Blacklist hit produces high risk with traceable reason.
- Missing buyer, phone, address, or amount is handled safely.
- Duplicate order emits one alert.
- Boundary amounts `0`, one cent, max supported amount, and overflow-like values are safe.
- Shop A cannot read or act on shop B.
- Manual release prevents later automatic block.
- PII never appears in logs or audit messages.
- Medium candidate is retained in reports/inbox but never triggers Discord or Telegram.
