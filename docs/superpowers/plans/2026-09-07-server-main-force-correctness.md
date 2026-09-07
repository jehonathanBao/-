# Server-authoritative contract main-force monitoring implementation plan

> For agentic workers: use subagent-driven-development for isolated correctness tasks and keep an evidence ledger. The user approved implementation and Git/local/server synchronization; no additional execution-choice prompt is needed.

**Goal:** Import the current server source as the authoritative baseline, repair the audited signal/calibration/grade faults, improve sustained-flow behavior evidence, and synchronize verified source and running artifacts.

**Architecture:** Preserve the Rust/SQLite collector, event and outbox architecture. Produce event-time evidence before persistence; use one canonical event importance grade across API/filter/UI/notification. Keep historical price validation distinct from participant-behavior hypotheses, with no trading execution.

**Tech stack:** Rust 2021, Tokio, rusqlite, React/Vite/Vitest, Docker Compose.

## Global constraints

- Server source is the newest baseline; do not restore older local behavior over it.
- Preserve local uncommitted user files, production data, runtime secrets and existing notification targets.
- Monitoring remains read-only with respect to accounts: no orders, cancellations, transfers, signing or private exchange credentials.
- No credentials, real runtime datasets, logs or `.env` files in Git. No force-push or destructive checkout/reset.
- One canonical S/A/B/C event importance grade; data insufficiency is a status, not an extra letter grade. Behavior, evidence confidence and prediction maturity are separate.
- Missing is not zero; event evidence must not use future observations. Hypotheses never establish real trader identity.
- External notifications retain high/critical eligibility, score >=80 and quality >=70, cooldown/idempotency, and no cached-on-boot historical burst. Medium/low remain display-only. Historical prediction maturity must not silently become a second event grade.
- Real side effects are limited to the explicitly requested version sync/deployment and existing monitoring behavior. Do not introduce new admin/trading buttons.
- Add focused regression tests first, run release checks before deployment, preserve rollback artifacts, and verify container/source revision and health.

## Task 1: authoritative baseline and provenance

Files: source/test/config-default/frontend/build assets from `/opt/toxic-order-monitor-rs`; exclude runtime/secret paths. Retain local-only tooling/docs separately rather than silently deleting them.

- [x] Inspect current Git state, remote and server source/runtime identity.
- [x] Preserve pre-sync commit in `codex/pre-server-sync-20260907` and create isolated worktree.
- [ ] Import server source archive and compare core source paths, dependency locks and build files.
- [ ] Scan staged files for secrets; record archive hash and baseline limitations.
- [ ] Commit imported baseline separately from fixes; run baseline contract-module tests.

## Task 2: calibration correctness

Owned files: `src/contract_whale_monitor/impact_forecast.rs`, `impact_v4_2.rs`, `impact_v4_2_gate.rs`, focused associated regression tests. Do not modify app.rs; report integration changes for Task 3/4.

Interfaces: retain existing forecast/outcome structs and public functions where possible. If API additions are necessary, make them explicit in the report and backward-compatible via serde defaults. The live caller passes persisted outcomes and references.

- [ ] Add failing regression tests for profitable bearish signed markout, symmetric bullish/bearish model priors, source precedence, future candle exclusion, duplicate outcomes, matured-only cohorts and non-tautological coverage.
- [ ] Run focused module tests to confirm the expected failures.
- [ ] Make aligned signed return positive for successful long and short signals, with raw price direction converted only once. Canonical success predicate: `markout.is_finite() && markout > 0.0` for known directional hypotheses.
- [ ] Select references using actual event timestamps, prefer valid index/mark before fallback, and forbid future entry data. Count unique time slots for coverage, not raw path length.
- [ ] Deduplicate prior outcomes by independent episode/horizon/version (with deterministic version choice), require closed horizons and valid quality/coverage; exclude the current episode.
- [ ] Count eligible mature candidates including missing results in gate coverage, do not invent 100% for any non-empty sample. Use consistent effective sample weighting and no future mature outcomes at past cutoff.
- [ ] Invalidate old forecast/outcome calibration versions when corrected semantics change. Preserve old rows for audit and ensure new training does not consume invalid prior-version data.
- [ ] Run focused regression tests and existing impact/forecast/gate module tests, commit owned files and report RED/GREEN evidence.

## Task 3: truthful evidence and behavior

Files: `src/app.rs`, `src/api/contract_whale_routes.rs`, `src/contract_whale_monitor/{aggregator,collector_binance,detector,behavior_assessment,trajectory,types,persistence}.rs`, relevant focused tests.

Interfaces: keep existing response schema compatible; extend structured availability/heuristic semantics only when necessary. Persist actual event-time signal evidence rather than retrofitting it only during GET.

- [ ] Add production-entry tests proving micro-volatility has samples and spot evidence reaches persistence.
- [ ] Share one evidence enrichment path for producer and API without replacing historical snapshots with current market context.
- [ ] Calculate source quality using enabled-source freshness/completeness, not raw venue count. Cross-venue breadth remains confidence context; a healthy single venue must not be permanently capped below downstream quality requirements.
- [ ] Implement collector heartbeat/connection/event state; distinguish healthy-empty liquidation from unknown/stale. Preserve WebSocket write half to handle ping/pong and bounded reconnects; do not log stream payloads or secret values.
- [ ] Replace residual whale/retail percentages with explicit unknown attribution when inputs do not establish origin. Mark heuristics as scores, not true participant shares.
- [ ] Require multiple independent actions and non-zero sustained duration before trajectory accumulation/distribution. Buy pressure with falling OI is not proof of new-long building.
- [ ] Separate price follow-through validation from behavior confirmation; never let price alone confirm a missing-evidence behavior.
- [ ] Prevent calibration in-memory history duplication at the app loop and avoid frequent reprocessing of unchanged completed outcomes; freeze trigger-time forecasts and update only eligible maturity results.
- [ ] Run focused tests and commit verified changes.

## Task 4: one canonical event grade and sustained-flow detection

Files: `src/contract_whale_monitor/{config,impact_grade,discord_gate,discord_notifier,discord,emission,event_lifecycle,mod}.rs`, `src/storage/{contract_whale_repo,contract_event_grade_repo}.rs`, `src/core_event/final_store/final_event_store.rs`, `src/api/{contract_whale_routes,contract_event_routes,final_event_routes}.rs`, `src/app.rs`, frontend contract API/component/tests. Add a focused sustained-flow module if needed instead of expanding unrelated modules.

- [ ] Test that the same event ID exposes the same grade/version/status in history, filtering, detail and Discord payload; independent forecast grades cannot override event importance.
- [ ] Choose the canonical current event assessment as the sole displayed/routed grade, update its version, and make source requirements compatible with configured Binance-only operation without lowering high-risk notification safety floors.
- [ ] Keep forecasts as optional outcome diagnostics; distinguish insufficient event evidence from C-grade ordinary events.
- [ ] Aggregate independent 1-second flow into sustained 1/5/15/60-minute observation windows, with bounded state and no double-counting overlapping windows. Detect persistent net-flow participation against available baseline; emit explicit candidate evidence, not a trader identity.
- [ ] Keep fast 5/15/60-second sweep detection. Group a sustained episode with stable IDs, lifecycle update/invalidation and restart-safe no-backfill alert boundaries.
- [ ] Label new-position, closing/squeeze, absorption-candidate and unknown behaviors separately. No L2 implies no verified iceberg/passive-owner claim.
- [ ] Update frontend grade labels/filter mapping and hide misleading participant percentages; preserve existing layout and no new execution actions.
- [ ] Run contract API/Discord/persistence/lifecycle tests and targeted Vitest suites; commit changes.

## Task 5: verification, rollout, Git/local sync

Files: build-info/build scripts/Docker labels, release notes, deployment checks. No secrets or runtime data tracked.

- [ ] Run `cargo fmt --check`, `cargo clippy -j 1 --all-targets --all-features -- -D warnings`, `cargo test -j 1 --all-targets`; diagnose baseline versus introduced failures explicitly.
- [ ] Run `node --check web/app.js`, frontend `npm run test:full`, `npm run build`, `npm audit --audit-level=high`.
- [ ] Review change set against audit findings; fix important findings and retest affected paths.
- [ ] Scan staged content; commit final source and push to the existing remote branch without force.
- [ ] Preserve production configuration/database and rollback image before replacing code; build explicit revision-labelled artifacts with monitoring-only boundaries.
- [ ] Deploy only the approved source, keep unrelated modules/settings intact, and prevent old history from generating real alerts on restart.
- [ ] Verify source revision/hash, healthy containers, `/healthz`, `/readyz`, canonical-grade APIs, actual evidence freshness and frontend artifact version.
- [ ] Fast-forward original local branch to the release, preserving user untracked files, and verify Git remote/local/server revision agreement.
- [ ] Report completed fixes, exact validation/deployment results, and remaining empirical calibration limitations. Do not claim improved real-world precision without a valid labelled holdout evaluation.
