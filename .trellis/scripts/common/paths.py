from pathlib import Path

TRELLIS_DIR = Path(__file__).resolve().parents[2]
REPO_ROOT = TRELLIS_DIR.parent
RUNTIME_DIR = TRELLIS_DIR / ".runtime"
SESSIONS_DIR = RUNTIME_DIR / "sessions"
TASKS_DIR = TRELLIS_DIR / "tasks"
SPEC_DIR = TRELLIS_DIR / "spec"
WORKSPACE_DIR = TRELLIS_DIR / "workspace"
DEVELOPER_FILE = TRELLIS_DIR / ".developer"
WORKFLOW_FILE = TRELLIS_DIR / "workflow.md"


def ensure_dir(path: Path) -> Path:
    path.mkdir(parents=True, exist_ok=True)
    return path
