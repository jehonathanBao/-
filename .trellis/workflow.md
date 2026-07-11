# Trellis Workflow

This repository uses a lightweight Trellis workflow to keep multi-step work explicit, reversible, and easy to resume.

## Quick Start

1. Load session context:
   `python ./.trellis/scripts/get_context.py`
2. Review workflow phases:
   `python ./.trellis/scripts/get_context.py --mode phase`
3. Discover spec layers:
   `python ./.trellis/scripts/get_context.py --mode packages`
4. Create or resume a task:
   `python ./.trellis/scripts/task.py create "<title>" --slug <slug>`

## Skill Routing

| Situation | Skill |
| --- | --- |
| New feature / fuzzy scope | `trellis-brainstorm` |
| About to edit code | `trellis-before-dev` |
| Finished implementation | `trellis-check` |
| Repeated bug loop | `trellis-break-loop` |
| New repo convention learned | `trellis-update-spec` |

## Phase Index

### Phase 1 - Planning

#### Step 1.1 - Create or resume task
- Create a task for any multi-file or multi-step request.
- Seed `prd.md` immediately with goal, requirements, acceptance, and out-of-scope notes.
- If the request is ambiguous, route through `trellis-brainstorm`.

#### Step 1.2 - Gather repo and product context
- Read `PRODUCT.md`, repo-level `AGENTS.md`, and relevant `.trellis/spec/*/index.md` files.
- Inspect affected code paths before asking questions that can be answered locally.
- Capture constraints in the task PRD.

#### Step 1.3 - Lock MVP scope
- Confirm what is in scope for this task.
- State safety boundaries explicitly for trading, deployment, deletion, or any irreversible path.
- Leave stretch ideas in `Out of Scope`.

### Phase 2 - Prepare for Implementation

#### Step 2.1 - Load implementation guidance
- Run `python ./.trellis/scripts/get_context.py --mode packages`.
- Read the relevant spec indexes for backend, frontend, and ops before editing.
- Decide the smallest validation set that proves the change.

#### Step 2.2 - Set test strategy
- Start with the failing behavior or missing capability.
- Prefer narrow tests first, then broaden only if the blast radius justifies it.
- Keep notes about expected commands in the PRD.

### Phase 3 - Execute

#### Step 3.1 - TDD red to green
- Reproduce the failure or capture the missing capability.
- Implement the smallest change that turns the test green.
- Keep edits scoped to the active task.

#### Step 3.2 - Verify locally
- Run the project-native validation commands that match the touched surfaces.
- Record failures before retrying.
- Confirm user-visible behavior, not just code-level assumptions.

#### Step 3.3 - Capture learnings
- If the task introduced a reusable pattern, update `.trellis/spec/`.
- If the workflow itself needs tuning, update this file and note why.

### Phase 4 - Wrap Up

#### Step 4.1 - Self-check
- Re-read the acceptance criteria.
- Confirm that safety boundaries still hold.
- Summarize what changed, how it was verified, and any remaining risks.

#### Step 4.2 - Handoff
- If the user asked for sync/deploy, do the real sync and verify health.
- If not, leave the repo in a clean, explainable state and point to the active task artifacts.

[workflow-state:no_task]
No active Trellis task. For multi-step work, create a task first or route through `trellis-brainstorm`.
[/workflow-state:no_task]

[workflow-state:planning]
Task is in planning. Update the PRD, read spec indexes, and lock the MVP before editing code.
[/workflow-state:planning]

[workflow-state:in_progress]
Task is in progress. Stay scoped to the current PRD, keep TDD tight, and verify touched surfaces before claiming done.
[/workflow-state:in_progress]

[workflow-state:completed]
Task is completed. Re-check acceptance criteria, archive if appropriate, and carry forward any reusable conventions into `.trellis/spec/`.
[/workflow-state:completed]
