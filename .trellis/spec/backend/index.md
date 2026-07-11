# Backend Spec Index

Use this index before editing Rust monitor logic, APIs, persistence, or live diagnostics.

- [`/Cargo.toml`](/Cargo.toml): workspace dependencies and crate shape.
- [`/src`](/src): primary Rust implementation tree.
- [`/docs/security/contract-whale-readonly-boundary.md`](/docs/security/contract-whale-readonly-boundary.md): read-only and disabled-state expectations for contract whale flows.
- [`/docs/project-runtime-acceptance-matrix.md`](/docs/project-runtime-acceptance-matrix.md): runtime checks, data integrity expectations, and diagnostic patterns.
- [`/docs/live-data-deployment-checklist.md`](/docs/live-data-deployment-checklist.md): health and rollout verification for live data services.

Backend rules for this repo:

1. Preserve read-only monitoring semantics unless the user explicitly widens scope.
2. Prefer diagnostics and explicit reasons over silent filtering.
3. Verify real runtime paths when the task touches server behavior or persistence.
