import os
from pathlib import Path

from .paths import DEVELOPER_FILE, WORKSPACE_DIR, ensure_dir


def default_developer() -> str:
    return (
        os.environ.get("TRELLIS_DEVELOPER")
        or os.environ.get("USERNAME")
        or os.environ.get("USER")
        or "unknown"
    ).strip()


def get_developer() -> str:
    if DEVELOPER_FILE.exists():
        value = DEVELOPER_FILE.read_text(encoding="utf-8").strip()
        if value:
            return value
    return default_developer()


def workspace_dir(developer: str | None = None) -> Path:
    name = developer or get_developer()
    return WORKSPACE_DIR / name


def initialize_developer(developer: str) -> Path:
    ensure_dir(WORKSPACE_DIR)
    DEVELOPER_FILE.write_text(f"{developer}\n", encoding="utf-8")
    root = ensure_dir(workspace_dir(developer))
    index_path = root / "index.md"
    journal_path = root / "journal-1.md"
    if not index_path.exists():
        index_path.write_text(
            "# Workspace Index\n\n- journal-1.md: initial journal\n",
            encoding="utf-8",
        )
    if not journal_path.exists():
        journal_path.write_text("# Journal 1\n\n", encoding="utf-8")
    ensure_dir(root / ".agents")
    return root
