from .task_utils import get_current_task, load_task_json


def current_status() -> str:
    task = get_current_task()
    if task is None:
        return "no_task"
    data = load_task_json(task)
    return data.get("status", "planning")


def workflow_state() -> str:
    status = current_status()
    if status in {"planning", "in_progress", "completed"}:
        return status
    return "planning"
