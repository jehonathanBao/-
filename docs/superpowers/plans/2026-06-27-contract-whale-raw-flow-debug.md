# Contract Whale Raw Flow Upstream Debug Plan

## Context
- 线上 `/api/contract-whale/pipeline-debug?symbol=BTC&range=24h` 已确认 `rawFlow.flow1sRows = 0`
- `/api/contract-events/debug-counts` 已确认 BTC 24h history = 0、hidden = 0、final-events raw = 0
- latest 仍保留旧 BTC 快照，导致页面把“旧 latest”误解为“当前没有展示历史”

## Goal
把诊断链路向上扩展到：
1. connector requested symbol / venue symbol mapping
2. raw trade ingest health
3. flow-state / rolling window aggregation
4. `contract_flow_1s` 持久化事实
5. stale latest 告警的页面解释

## Non-goals
- 不调 detector 阈值
- 不强行制造 BTC 历史事件
- 不改变 `total_volume_btc = buy + sell` 算法

## TDD Steps
1. 后端新增 `GET /api/contract-whale/raw-flow-debug?symbol=BTC&range=24h` 的失败测试
2. 前端新增 raw-flow-debug 获取与 stale + zero-raw-flow 诊断提示的失败测试
3. 修复 `scripts/check_contract_event_counts.sh`，去掉 `curl | head` + `pipefail` 风险
4. 实现 raw-flow-debug 路由与诊断结构
5. 实现前端提示与诊断展示
6. 跑 targeted tests + build，自查无回归

## Expected Evidence
- 能明确看到 BTC 卡在 raw ingest / mapping / flow-state / `contract_flow_1s` 哪一层
- 页面不再把 stale latest 误导成“当前实时信号”
- 线上同步后可直接用 `raw-flow-debug` 判断是否为 symbol mismatch / no ingest / no persistence
