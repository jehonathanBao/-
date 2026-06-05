---
name: langflow-risk-sidecar
description: Design or integrate a Langflow/LLM sidecar for risk scoring with strict policy gates, SSRF protection, redaction, and manual-review boundaries.
---

# Langflow Risk Sidecar

Use this skill when creating or reviewing an AI sidecar, Langflow flow, webhook bridge, or Rust client for risk assistance.

## Sidecar Boundary

- The sidecar is advisory only.
- It may return `risk_score`, `risk_level`, `reason`, and `suggested_action`.
- It must not place orders, refund, intercept shipments, ban users, update rules, mutate blacklists, or sign transactions.
- Irreversible actions require manual review regardless of model output.
- Low-risk output must never trigger automatic blacklist or block.

## Input Policy

- Redact PII before sending data to Langflow unless the deployment is explicitly approved for that data class.
- Use stable IDs and hashed summaries instead of raw addresses, ID numbers, phones, and tokens.
- Include rule context and blacklist/whitelist summaries, not raw private records.

## Output Contract

```json
{
  "risk_score": 0.0,
  "risk_level": "low",
  "reason": "short traceable explanation",
  "suggested_action": "allow",
  "policy_flags": [],
  "requires_manual_review": false
}
```

## Integration Rules

- Rust callers must enforce timeout, HTTPS/allowlist, response size limit, and schema validation.
- Sidecar errors must fail closed as `data_insufficient`; never treat missing AI output as approval.
- Treat invalid or unavailable sidecar responses as `data_insufficient`, not as approval.
- Persist only evidence references or redacted summaries unless a durable evidence card explicitly allows more.

## Webhook Prototype Rules

- Keep the first integration behind a local-only or allowlisted webhook URL.
- Send redacted order features, not raw L2/trade captures or raw customer fields.
- Require `requires_manual_review: true` for refund, ban, intercept, blacklist, or rule-change suggestions.
- Log sidecar request IDs and schema failures, not raw prompts or raw responses containing private data.
- Add tests for timeout, invalid schema, low-risk no-action, and manual-review escalation before enabling in runtime.
