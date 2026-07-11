import argparse
import json
from datetime import datetime
from pathlib import Path

from common.developer import get_developer, initialize_developer
from common.paths import TASKS_DIR, ensure_dir
from common.task_utils import (
    get_current_task,
    list_task_dirs,
    load_task_json,
    save_task_json,
    set_current_task,
    slugify,
    task_dir_name,
)


def prd_template(title: str, goal: str) -> str:
    return f"""# {title}

## Goal

{goal}

## What I already know

* Task was created through Trellis bootstrap.

## Assumptions (temporary)

* Scope stays read-only unless the user explicitly widens it.

## Open Questions

* None yet.

## Requirements (evolving)

* Keep edits scoped to the task goal.

## Acceptance Criteria (evolving)

* [ ] Workflow-ready plan exists
* [ ] Validation strategy is captured

## Definition of Done (team quality bar)

* Tests or smoke checks are recorded
* Validation commands are noted
* Behavior changes are documented if needed

## Out of Scope (explicit)

* Unrelated repo cleanup

## Technical Notes

* Created by `.trellis/scripts/task.py`
"""


def create_task(args: argparse.Namespace) -> None:
    developer = get_developer()
    initialize_developer(developer)
    ensure_dir(TASKS_DIR)
    slug = slugify(args.slug or args.title)
    task_dir = TASKS_DIR / task_dir_name(args.title, slug=slug, developer=developer)
    task_dir.mkdir(parents=True, exist_ok=False)
    payload = {
        "title": args.title,
        "slug": slug,
        "status": "planning",
        "assignee": developer,
        "type": args.type,
        "created_at": datetime.utcnow().isoformat(timespec="seconds") + "Z",
        "branch": None,
        "parent": args.parent,
        "subtasks": [],
    }
    save_task_json(task_dir, payload)
    (task_dir / "prd.md").write_text(
        prd_template(args.title, args.title),
        encoding="utf-8",
    )
    set_current_task(task_dir)
    print(str(task_dir))


def list_tasks(_: argparse.Namespace) -> None:
    current = get_current_task()
    for path in list_task_dirs():
        data = load_task_json(path)
        marker = " (active)" if current and current == path else ""
        print(f"{path.name}: {data.get('status', 'unknown')}{marker}")


def start_task(args: argparse.Namespace) -> None:
    task = Path(args.task_dir)
    if not task.exists():
        raise SystemExit(f"Task not found: {task}")
    set_current_task(task)
    data = load_task_json(task)
    data["status"] = "in_progress"
    save_task_json(task, data)
    print(str(task))


def finish_task(_: argparse.Namespace) -> None:
    task = get_current_task()
    if task is None:
        print("No active task")
        return
    data = load_task_json(task)
    data["status"] = "completed"
    save_task_json(task, data)
    set_current_task(None)
    print(str(task))


def current_task(_: argparse.Namespace) -> None:
    task = get_current_task()
    print(str(task) if task else "")


def set_branch(args: argparse.Namespace) -> None:
    task = Path(args.task_dir)
    data = load_task_json(task)
    data["branch"] = args.branch
    save_task_json(task, data)
    print(str(task))


def add_subtask(args: argparse.Namespace) -> None:
    task = Path(args.task_dir)
    data = load_task_json(task)
    subtasks = data.setdefault("subtasks", [])
    subtasks.append(args.subtask_dir)
    save_task_json(task, data)
    print(str(task))


def init_context(args: argparse.Namespace) -> None:
    task = Path(args.task_dir)
    context = task / "context"
    context.mkdir(parents=True, exist_ok=True)
    for name in ("notes.jsonl", "research.jsonl", "verification.jsonl"):
        file = context / name
        if not file.exists():
            file.write_text("", encoding="utf-8")
    print(str(context))


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    create = subparsers.add_parser("create")
    create.add_argument("title")
    create.add_argument("--slug")
    create.add_argument("--assignee", default=None)
    create.add_argument("--type", default="fullstack")
    create.add_argument("--parent", default=None)
    create.set_defaults(func=create_task)

    listing = subparsers.add_parser("list")
    listing.set_defaults(func=list_tasks)

    start = subparsers.add_parser("start")
    start.add_argument("task_dir")
    start.set_defaults(func=start_task)

    finish = subparsers.add_parser("finish")
    finish.set_defaults(func=finish_task)

    current = subparsers.add_parser("current")
    current.set_defaults(func=current_task)

    init_context_parser = subparsers.add_parser("init-context")
    init_context_parser.add_argument("task_dir")
    init_context_parser.add_argument("dev_type")
    init_context_parser.set_defaults(func=init_context)

    branch = subparsers.add_parser("set-branch")
    branch.add_argument("task_dir")
    branch.add_argument("branch")
    branch.set_defaults(func=set_branch)

    add_subtask_parser = subparsers.add_parser("add-subtask")
    add_subtask_parser.add_argument("task_dir")
    add_subtask_parser.add_argument("subtask_dir")
    add_subtask_parser.set_defaults(func=add_subtask)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
