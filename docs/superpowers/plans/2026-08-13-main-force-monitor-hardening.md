# BTC 主力合约监控完整化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 BTC 合约监控从“成交量/冲击等级监控 + 行为 MVP”升级为统一证据门槛、可撤销事件状态、通知不绕过、S 级语义清晰的只读主力行为监控系统。

**Architecture:** 保留现有合约流检测和市场冲击评分，但把 `BehaviorAssessment` 作为主力行为唯一权威出口；市场冲击单独作为 impact lane。行为先按信号形成 episode，再由 OI、价格、现货、跨交易所和清算证据推进或撤销状态。Discord 直接通知与 Outbox 共用同一 gate 和稳定 episode key。

**Tech Stack:** Rust/Axum/rusqlite、React/Vite/Vitest、现有 Docker Compose 部署。

## Global Constraints

- 保持 `READ_ONLY=true` 与 `executionEnabled=false`，不加入下单、撤单、资金、签名或账户操作。
- 未确认行为只能显示为候选、普通成交流或市场冲击，不得标记为“主力确认”。
- OI/价格/跨市场证据缺失或降级时 fail-closed。
- 市场冲击等级与主力行为可信度必须分开。
- 现有未跟踪文件 `.trellis/scripts/common/__pycache__/` 与旧计划文件不得删除或覆盖。

---

### Task 1: 统一行为门槛并修复清算误判

**Files:**
- Modify: `src/contract_whale_monitor/behavior.rs`
- Modify: `src/contract_whale_monitor/detector.rs`
- Test: `tests/contract_whale_behavior_tests.rs`
- Test: `tests/contract_whale_monitor_tests.rs`

**Interfaces:**
- `assess_contract_whale_behavior` 继续返回 `BehaviorAssessment`，但清算分支只接受 detector 已确认的 `liquidation_suspected`。
- 任何非零伴随清算不得单独升级为 `LiquidationSweep`。

- [ ] **Step 1: Write failing tests**

增加测试：1 BTC 伴随清算仍按 OI/价格证据分类；`liquidation_suspected=true` 才进入 `LiquidationSweep`；OI 不可用始终 `Insufficient`。

- [ ] **Step 2: Run focused tests and verify RED**

运行 `cargo test -j 1 --test contract_whale_behavior_tests`，预期新增“微小清算仍可分类”断言失败。

- [ ] **Step 3: Implement minimal fix**

删除 `liquidation_total_btc > 0.0` 的行为层直接门槛，使行为层只依赖 detector 的布尔清算结论；保留清算量用于证据文案和 impact lane。

- [ ] **Step 4: Run GREEN**

运行 `cargo test -j 1 --test contract_whale_behavior_tests --test contract_whale_monitor_tests`，预期全部通过。

### Task 2: 统一 Discord 行为/冲击双通道

**Files:**
- Modify: `src/contract_whale_monitor/discord.rs`
- Modify: `src/contract_whale_monitor/discord_notifier.rs`
- Modify: `src/app.rs`
- Test: `tests/contract_whale_discord_notifier_tests.rs`
- Test: `tests/contract_whale_monitor_tests.rs`

**Interfaces:**
- 新增内部 gate：行为通知必须 `Confirmed + main_force_confirmed + confidence >= 80`；清算/冲击通知只能走 impact lane。
- `build_contract_whale_discord_payload` 必须根据 lane 输出“主力行为候选/市场冲击/清算驱动”对应文案。

- [ ] **Step 1: Write failing tests**

覆盖 direct notifier：`behavior_state=insufficient` 的 A/B/S impact 不得以“主力合约异动”推送；`liquidation_sweep` 可以以冲击通道推送但不得含“主力确认”；confirmed 行为必须可推送。

- [ ] **Step 2: Run RED**

运行 `cargo test -j 1 --test contract_whale_discord_notifier_tests --test contract_whale_monitor_tests`，预期旧 direct gate 测试失败。

- [ ] **Step 3: Implement**

在 `should_push_contract_whale_discord` 和 `evaluate_contract_whale_discord_gate` 的共同路径加入 authoritative behavior gate；将 impact override 限制为 impact lane；payload 标题和 footer 使用 lane 标签。

- [ ] **Step 4: Run GREEN**

运行同一组测试并确认全部通过。

### Task 3: 增加稳定 episode key 与通知去重

**Files:**
- Modify: `src/contract_whale_monitor/discord_notifier.rs`
- Modify: `src/contract_whale_monitor/emission.rs`
- Modify: `src/storage/contract_whale_repo.rs`
- Test: `tests/contract_whale_discord_notifier_tests.rs`
- Test: `tests/contract_whale_persistence_tests.rs`

**Interfaces:**
- 新增确定性 `episode_key(symbol, direction, behavior_type, lifecycle_start)`，同一 episode 跨 5/15/60 秒和重启保持一致。
- Outbox 去重优先使用 episode key，保留 signal id 作为审计字段。

- [ ] **Step 1: Write failing tests**

同一 episode 的三个窗口和一次重放只能生成一个可发送记录；方向反转或行为类型变化必须生成新 key。

- [ ] **Step 2: Run RED**

运行 `cargo test -j 1 --test contract_whale_persistence_tests --test contract_whale_discord_notifier_tests`，预期重复记录断言失败。

- [ ] **Step 3: Implement**

使用已有生命周期起始时间和规范化行为类型构造稳定 key，在 SQLite outbox 唯一检查和内存 cooldown 两处复用。

- [ ] **Step 4: Run GREEN**

运行同一组测试并检查重复、重启、重放场景全部通过。

### Task 4: 事件状态机与失效路径

**Files:**
- Modify: `src/contract_whale_monitor/behavior.rs`
- Modify: `src/storage/migrations.rs`
- Modify: `src/storage/contract_whale_repo.rs`
- Modify: `src/api/contract_event_routes.rs`
- Test: `tests/contract_whale_behavior_tests.rs`
- Test: `tests/contract_whale_persistence_tests.rs`

**Interfaces:**
- `BehaviorState` 支持 `Insufficient -> Provisional -> Confirmed -> Invalidated/Closed`。
- 保存状态迁移原因、证据版本、最新反证、评估时间。

- [ ] **Step 1: Write failing tests**

增加确认后 OI 反转、价格不跟随、数据源降级三类失效测试；数据库重开后状态和反证保持一致。

- [ ] **Step 2: Run RED**

运行 `cargo test -j 1 --test contract_whale_behavior_tests --test contract_whale_persistence_tests`，预期失效状态测试失败。

- [ ] **Step 3: Implement**

增加状态迁移函数和持久化字段，避免使用 `existing || observation` 永久锁存确认状态；API 返回最新状态和失效原因。

- [ ] **Step 4: Run GREEN**

运行同一组测试并验证旧记录兼容读取。

### Task 5: S 级与行为等级解耦

**Files:**
- Modify: `src/contract_whale_monitor/detector.rs`
- Modify: `src/contract_whale_monitor/config.rs`
- Modify: `src/api/contract_whale_routes.rs`
- Test: `tests/contract_whale_monitor_tests.rs`
- Test: `tests/cwm_risk_fusion_tests.rs`

**Interfaces:**
- 保留 `severity` 兼容字段，新增 `market_impact_grade` 和 `behavior_confidence_tier`。
- S 级只能在 episode 级极端条件满足时产生；短窗口高流量只能是 High/Critical impact，不能自动代表主力行为。

- [ ] **Step 1: Write failing tests**

增加普通 2 亿美元、0.25% 价格变化、P99.9 流量不触发 episode S 的测试；极端清算 + OI 去杠杆 + 多源确认的 fixture 触发 S。

- [ ] **Step 2: Run RED**

运行 `cargo test -j 1 --test contract_whale_monitor_tests --test cwm_risk_fusion_tests`，预期旧 S 语义测试失败。

- [ ] **Step 3: Implement**

把 S 判定移动到 episode 汇总条件；impact 与 behavior 字段同时保留，通知使用明确的 impact/behavior lane。

- [ ] **Step 4: Run GREEN**

运行同一组测试并确认旧 API 兼容字段仍可读。

### Task 6: 前端文案与证据展示

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`

**Interfaces:**
- 未确认 `aggressive_buy/sell` 显示为“主动买压/主动卖压”。
- 只在 `behaviorState=confirmed` 显示“主力建多/建空”；清算和冲击显示独立标签。
- 详情卡显示支持证据、反证、数据新鲜度、状态迁移原因。

- [ ] **Step 1: Write failing tests**

测试普通成交不显示“主力拉盘/主力砸盘”，确认行为显示“主力建多/建空”，清算只显示“清算驱动”。

- [ ] **Step 2: Run RED**

运行 `npm test -- --run src/tests/ContractWhaleMonitor.test.jsx src/tests/ContractWhaleApi.test.js`。

- [ ] **Step 3: Implement**

集中修改 label map 和详情卡，不改变数据请求协议之外的执行行为。

- [ ] **Step 4: Run GREEN**

运行同一组测试并执行 `npm run build`。

### Task 7: 真实回放与发布门禁

**Files:**
- Create: `tests/fixtures/contract_whale_behavior/README.md`
- Modify: `README.md`
- Modify: `docs/live-data-deployment-checklist.md`
- Test: `tests/contract_whale_behavior_tests.rs`

**Interfaces:**
- 回放必须保留只读模式，输出行为混淆矩阵和事件级结果。
- 没有人工标签时不得自动提升阈值或开启主力通知。

- [ ] **Step 1: Add fixture contract and tests**

定义正负样本字段、标签规则和最小验收指标，加入 synthetic fallback 测试以保证无真实数据时安全失败。

- [ ] **Step 2: Run verification**

运行后端关键测试、前端全量测试、前端构建、`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`。

- [ ] **Step 3: Commit and deploy**

检查 diff 与 secrets，提交到当前 `codex/` 分支，推送 origin；服务器执行拉取、构建、重启 backend/frontend，验证 `/healthz`、`/readyz`、只读环境变量和行为/通知接口。

---

## Self-review

- 已覆盖行为真相源、Discord gate、清算误判、episode 去重、失效状态、S 级语义、UI 文案、回放门禁。
- 所有生产代码任务均先写失败测试，再实现，再运行绿色测试。
- 没有引入交易执行、账户权限或密钥变更。
