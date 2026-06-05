# Langflow Order Risk Sidecar

This is an advisory sidecar design for future order-risk workflows. It is intentionally not wired into the live BTC monitor runtime.

## Flow Contract

Input is redacted order JSON:

```json
{
  "order_id": "ord_123",
  "tenant_id": "tenant_a",
  "shop_id": "shop_1",
  "features": {
    "amount_cents": 2599,
    "buyer_age_days": 12,
    "phone_hash_present": true,
    "address_hash_present": true,
    "rule_hits": ["new_buyer_high_amount"]
  }
}
```

Output:

```json
{
  "risk_score": 72.5,
  "risk_level": "high",
  "reason": "New buyer with high amount and matching rule hit.",
  "suggested_action": "review",
  "policy_flags": [],
  "requires_manual_review": true
}
```

## Curl Example

```bash
curl -X POST "https://risk-sidecar.example.com/webhook/order-risk" \
  -H "Content-Type: application/json" \
  -d @redacted-order.json
```

## Rust Caller Notes

- Enforce HTTPS and allowlist before constructing the client.
- Set a short timeout.
- Limit response size.
- Validate schema.
- Fail closed as `data_insufficient` when unavailable.
- Never send raw phone, address, ID number, full email, token, or raw order payload unless a data approval exists.

See `.agents/templates/rust/order-risk/langflow_client.template.rs`.
