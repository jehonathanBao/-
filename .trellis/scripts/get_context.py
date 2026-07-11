import argparse
import json
import re
from pathlib import Path

from common.developer import get_developer, initialize_developer, workspace_dir
from common.git_context import branch_name, git_status_short, recent_commits
from common.paths import SPEC_DIR, TASKS_DIR, WORKFLOW_FILE
from common.task_utils import get_current_task, list_task_dirs, load_task_json


def parse_step(step: str) -> str:
    text = WORKFLOW_FILE.read_text(encoding="utf-8")
    pattern = re.compile(
        rf"^#### Step {re.escape(step)} .*?(?=^#### Step |\Z|^### Phase )",
        re.MULTILINE | re.DOTALL,
    )
    match = pattern.search(text)
    return match.group(0).strip() if match else f"Step {step} not found in workflow.md"


def phase_index() -> str:
    text = WORKFLOW_FILE.read_text(encoding="utf-8")
    lines = []
    for line in text.splitlines():
        if line.startswith("### Phase ") or line.startswith("#### Step "):
            lines.append(line)
    return "\n".join(lines)


def package_index() -> dict:
    ensure = []
    packages = []
    if SPEC_DIR.exists():
        for entry in sorted(SPEC_DIR.iterdir()):
            if entry.is_dir() and (entry / "index.md").exists():
                packages.append({"name": entry.name, "index": str(entry / "index.md")})
    return {"packages": packages, "guides": str(SPEC_DIR / "guides" / "index.md")}


def build_context() -> dict:
    developer = get_developer()
    initialize_developer(developer)
    current = get_current_task()
    tasks = []
    for task_dir in list_task_dirs():
        data = load_task_json(task_dir)
        tasks.append(
            {
                "path": str(task_dir),
                "title": data.get("title"),
                "status": data.get("status"),
                "active": bool(current and current == task_dir),
                "prd_exists": (task_dir / "prd.md").exists(),
            }
        )
    current_payload = None
    if current:
        data = load_task_json(current)
        current_payload = {
            "path": str(current),
            "title": data.get("title"),
            "status": data.get("status"),
            "prd_exists": (current / "prd.md").exists(),
        }
    return {
        "developer": developer,
        "branch": branch_name(),
        "git_status": git_status_short(),
        "recent_commits": recent_commits(),
        "current_task": current_payload,
        "active_tasks": tasks,
        "journal": str(workspace_dir(developer) / "journal-1.md"),
    }


def print_default(as_json: bool) -> None:
    payload = build_context()
    if as_json:
        print(json.dumps(payload, indent=2))
        return
    print(f"Developer: {payload['developer']}")
    print(f"Branch: {payload['branch']}")
    print("Git status:")
    print(payload["git_status"])
    if payload["current_task"]:
        current = payload["current_task"]
        print(f"Current task: {current['title']} ({current['status']})")
        print(f"Task path: {current['path']}")
        print(f"PRD exists: {'yes' if current['prd_exists'] else 'no'}")
    else:
        print("Current task: none")
    print("Active tasks:")
    if payload["active_tasks"]:
        for task in payload["active_tasks"]:
            active = " [active]" if task["active"] else ""
            print(f"- {task['title']} :: {task['status']}{active}")
    else:
        print("- none")
    print(f"Journal: {payload['journal']}")
    if payload["recent_commits"]:
        print("Recent commits:")
        for commit in payload["recent_commits"]:
            print(f"- {commit}")


def print_phase(step: str | None, as_json: bool) -> None:
    payload = {"step": step, "content": parse_step(step) if step else phase_index()}
    if as_json:
        print(json.dumps(payload, indent=2))
    else:
        print(payload["content"])


def print_packages(as_json: bool) -> None:
    payload = package_index()
    if as_json:
        print(json.dumps(payload, indent=2))
        return
    print("Package indexes:")
    for package in payload["packages"]:
        print(f"- {package['name']}: {package['index']}")
    print(f"Guides: {payload['guides']}")


def print_record(as_json: bool) -> None:
    payload = build_context()
    record = {
        "developer": payload["developer"],
        "current_task": payload["current_task"],
        "branch": payload["branch"],
        "journal": payload["journal"],
    }
    if as_json:
        print(json.dumps(record, indent=2))
    else:
        print(json.dumps(record, indent=2, ensure_ascii=False))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["default", "phase", "packages", "record"], default="default")
    parser.add_argument("--step")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--platform", default="codex")
    args = parser.parse_args()

    if args.mode == "phase":
        print_phase(args.step, args.json)
    elif args.mode == "packages":
        print_packages(args.json)
    elif args.mode == "record":
        print_record(args.json)
    else:
        print_default(args.json)


if __name__ == "__main__":
    main()
