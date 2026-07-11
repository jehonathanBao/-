# Ops Spec Index

Use this index before touching Docker, server sync, local launchers, or runtime audit paths.

- [`/docker-compose.yml`](/docker-compose.yml): container topology and published ports.
- [`/docs/server-deployment-runbook.md`](/docs/server-deployment-runbook.md): canonical deploy flow and nginx/public port behavior.
- [`/docs/live-data-deployment-checklist.md`](/docs/live-data-deployment-checklist.md): post-sync health expectations.
- [`/docs/windows-rust-build-stability-runbook.md`](/docs/windows-rust-build-stability-runbook.md): Windows build quirks and recovery path.

Ops rules for this repo:

1. Treat server verification as part of done only when the user asks for sync/restart.
2. Prefer exact health endpoints and live response checks over assumptions.
3. Keep secrets out of committed config and visible diagnostics.
