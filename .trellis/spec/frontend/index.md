# Frontend Spec Index

Use this index before editing React pages, API adapters, or operator-facing dashboards.

- [`/toxic-order-monitor/src`](/toxic-order-monitor/src): Vite/React UI surface.
- [`/PRODUCT.md`](/PRODUCT.md): design language, state semantics, and anti-patterns.
- [`/docs/toxic-signal-operator-runbook.md`](/docs/toxic-signal-operator-runbook.md): operator expectations for read-only review flows.
- [`/docs/react-risk-console-scaffold.md`](/docs/react-risk-console-scaffold.md): dashboard composition and console-style UI expectations.

Frontend rules for this repo:

1. Explain risk and state before exposing low-level metrics.
2. Distinguish `online`, `stale`, `disabled`, `spot_only`, and `dry-run` with text, not color alone.
3. Do not imply automated execution from ranking or intelligence panels.
