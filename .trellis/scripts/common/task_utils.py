import json
import re
from datetime import datetime
from pathlib import Path

from .developer import default_developer, get_developer
from .paths import RUNTIME_DIR, SESSIONS_DIR, TASKS_DIR, ensure_dir


def session_id() -> str:
    return "codex-default"


def session_file() -> Path:
    ensure_dir(SESSIONS_DIR)
    return SESSIONS_DIR / f"{session_id()}.json"


def load_session_state() -> dict:
    path = session_file()
    if not path.exists():
        return {"current_task": None, "platform": "codex"}
    return json.loads(path.read_text(encoding="utf-8"))


def save_session_state(state: dict) -> None:
    ensure_dir(RUNTIME_DIR)
    ensure_dir(SESSIONS_DIR)
    session_file().write_text(json.dumps(state, indent=2), encoding="utf-8")


def get_current_task() -> Path | None:
    current = load_session_state().get("current_task")
    if not current:
        return None
    path = Path(current)
    return path if path.exists() else None


def set_current_task(path: Path | None) -> None:
    state = load_session_state()
    state["current_task"] = str(path) if path else None
    state["platform"] = "codex"
    state["last_seen_at"] = datetime.utcnow().isoformat(timespec="seconds") + "Z"
    save_session_state(state)


def slugify(value: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", value.strip().lower())
    slug = slug.strip("-")
    return slug or "task"


def task_dir_name(title: str, slug: str | None = None, developer: str | None = None) -> str:
    dev = developer or get_developer() or default_developer()
    base = slugify(slug or title)
    prefix = datetime.now().strftime("%m-%d")
    return f"{prefix}-{base}-{dev}"


def list_task_dirs() -> list[Path]:
    ensure_dir(TASKS_DIR)
    return sorted([path for path in TASKS_DIR.iterdir() if path.is_dir() and path.name != "archive"])


def load_task_json(path: Path) -> dict:
    return json.loads((path / "task.json").read_text(encoding="utf-8"))


def save_task_json(path: Path, payload: dict) -> None:
    (path / "task.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
