# Live Contract Flow Persistence Plan

## Context
- 线上 `raw-flow-debug` 已确认 BTC / ETH rolling windows 有内存数据，但 `contract_flow_1s` 全空。
- live 自动循环当前只持久化 `contract_whale_signals`，没有把 1s flow bucket flush 到 SQLite。
- 公网 5173 对 `/api/contract-whale/raw-flow-debug` 的访问需要继续保持 `/api` 代理和 operator token 语义。

## Goal
1. 把 live runtime 的 1s flow bucket 真正落到 `contract_flow_1s`。
2. 保持 detector / signal 阈值不变，不伪造历史信号。
3. 保持 raw-flow-debug / latest stale 诊断链路可用。
4. 为后续服务器同步提供可验证的本地测试与脚本结果。

## TDD Steps
1. 新增失败测试：live flush 后 `contract_flow_1s` 有 BTC 行，重复 flush 不重复插入。
2. 新增失败测试：非 loopback 配置下 `raw-flow-debug` 缺 token 返回 JSON 401/403。
3. 在 live auto-push path 接入 flow bucket flush，并用轻量去重避免重复全量重刷。
4. 跑 targeted Rust / frontend tests、`cargo check`、`npm run build`。
5. 只提交本次相关文件，同步服务器并验证 5173 + raw-flow-debug + `contract_flow_1s`。

## Non-goals
- 不调整 detector 阈值
- 不修改 S / Critical / High / Medium 判定
- 不把 latest 强行写入 history
- 不手工造 `contract_whale_signals` / `contract_flow_1s` 假数据
