# Bootstrap Trellis workflow for toxic order monitor

## Goal

为当前 Rust + React 监控仓库建立可落地的 Trellis 工作流，让后续任务可以按 `trellis-start -> spec index -> task PRD -> 实现/验证` 的路径推进，而不是继续依赖零散计划文档和口头上下文。

## What I already know

* 仓库之前没有 `.trellis/` 工作流骨架，`trellis-start` 依赖的 `workflow.md`、`scripts/get_context.py`、`task.py` 都不存在。
* 项目是只读监控台，核心由 Rust 后端和 `toxic-order-monitor` React 前端组成。
* 仓库已存在 `PRODUCT.md`、`docs/server-deployment-runbook.md`、`docs/runbook/contract-whale-monitor.md` 等项目级规范文档，适合挂入 Trellis spec index。
* 仓库本身已有 `.workflow/` 和 `.runtime/` 历史目录，因此 Trellis 需要与现有运行资产共存，而不是覆盖它们。

## Assumptions (temporary)

* 本次只建立工作流与项目规范入口，不改业务功能，不做服务端同步。
* Trellis 仍需服从 repo 根 `AGENTS.md` 的安全边界：默认只读、默认不触发真实交易或破坏性动作。

## Open Questions

* 后续是否要把 `.trellis/` 和 `.codex/` 一并纳入版本管理，等你确认后再决定是否提交。

## Requirements (evolving)

* 提供可运行的 `.trellis/scripts/get_context.py`、`task.py`、`init_developer.py`、`add_session.py`。
* 提供 `workflow.md`，包含 phase index、skill routing、workflow-state 提示。
* 提供适配本项目的 spec index，至少覆盖 `guides`、`backend`、`frontend`、`ops`。
* 创建一个 bootstrap task，后续可以直接作为 `trellis-start` 的当前任务入口。

## Acceptance Criteria (evolving)

* [x] 仓库存在 `.trellis/workflow.md`
* [x] `python ./.trellis/scripts/get_context.py` 可运行并输出当前上下文
* [x] `python ./.trellis/scripts/get_context.py --mode phase` 可输出 phase index
* [x] `python ./.trellis/scripts/get_context.py --mode packages` 可输出 spec index 列表
* [x] `python ./.trellis/scripts/task.py create ...` 可以创建任务目录和 PRD
* [ ] 确认是否需要提交/同步这套 Trellis 工作流到远端

## Definition of Done (team quality bar)

* 脚本 smoke checks 已记录
* Trellis 与现有 repo 安全边界不冲突
* 后续新任务可以按 Trellis 起步，而不是重新手写计划

## Out of Scope (explicit)

* 业务代码修改
* 线上服务器同步
* 现有 `.workflow/` 历史任务数据迁移

## Technical Notes

* 采用 `npx trellis init --ides codex --framework none --footprint standard --no-index --no-interactive` 先生成基础 TrellisVCS 元数据，再手动补齐 `trellis-start` 所需 workflow/scripts/spec 层。
* 当前 bootstrap task 路径：`.trellis/tasks/06-28-bootstrap-trellis-workflow-byhdo`
