# Pro trading terminal v2 for contract whale

## Goal

把当前 `/contract-whale` 从“只读半机构级分析终端”升级为“职业级交易终端”，但保持明确语义分层：主视图继续承担市场结构分析，交易语义单独隔离到独立的 Trade Ideas / Setup 层，不自动交易、不污染现有事件流与分析链路。

## What I already know

* 当前主链路已经存在只读分析终端：`InstitutionalAnalysisTerminalPanel`，内容包括 `Market Regime / Liquidity Behavior / Signal Strength Ranking / Opportunity Map`。
* 旧版交易导向 UI 还残留在 `ContractWhaleMonitor.jsx` 中，但当前不在主视图展示：`TradingDecisionLayerPanel` 与 `TradeOpportunitiesPanel` 仍然存在。
* 后端当前已有 intelligence 层和只读接口：`/api/contract-whale/intelligence-terminal`。
* 现有产品边界是监控/分析台，不应把整个页面直接重写成“自动可交易信号源”。
* 用户已经明确选择方案 3：主视图保持只读分析终端，交易语义通过独立 panel / tab 隔离展示。

## Assumptions (temporary)

* 本次允许输出 `Trade Setups / LONG / SHORT / entry zone / invalidation` 等交易员辅助语义，但仍然是**只读建议**，不触发自动执行。
* 旧的交易决策代码可以复用，但需要重新收敛到新的语义分层，而不是直接恢复旧页面。

## Open Questions

* 交易层最终以什么 UI 形式挂到 `/contract-whale`：顶部 tabs、同页独立 panel，还是右侧辅助栏。

## Requirements (evolving)

* 保留并强化现有只读分析层：
  * Market Regime
  * Liquidity Map / Behavior
  * Signal Ranking
  * Fake Breakout Detection
  * Risk Context
* 新增独立的交易员辅助层：
  * Top trade setups
  * LONG / SHORT bias
  * entry zone
  * invalidation zone
  * confidence / rationale
* 交易层不得污染主事件流，不得让用户误以为当前系统会自动下单。
* Trade Setups 必须是独立语义平面，不与主分析层混排成一锅。

## Acceptance Criteria (evolving)

* [ ] `/contract-whale` 主视图仍以分析层为默认入口
* [ ] 交易层以独立区域展示，不替代主分析层
* [ ] 至少展示 Top 3 setups
* [ ] 继续保留 read-only / non-execution 文案边界
* [ ] 现有 intelligence API 和事件流不被破坏
* [ ] 有明确的前后端测试覆盖新的分层展示

## Definition of Done (team quality bar)

* 设计方案先明确语义分层与 UI 结构
* 再进入实现计划与 TDD
* 完成后能清楚解释“分析层”和“交易层”分别负责什么

## Out of Scope (explicit)

* 自动交易执行
* 订单路由
* 风险资金管理
* 无边界的全页面交易化重构

## Technical Notes

* 相关现状文件：
  * `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
  * `src/contract_whale_monitor/intelligence/mod.rs`
  * `src/api/contract_whale_routes.rs`
  * `toxic-order-monitor/src/api/contractWhale.js`
* 相关历史计划：
  * `docs/superpowers/plans/2026-06-28-contract-whale-institutional-terminal.md`
* 已确认的产品决策：
  * 方案 3：分析终端 + 独立 Trade Ideas panel/tab
