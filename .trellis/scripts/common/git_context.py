import subprocess
from pathlib import Path

from .paths import REPO_ROOT


def _run_git(args: list[str]) -> str:
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
    except FileNotFoundError:
        return "git unavailable"
    output = (result.stdout or result.stderr or "").strip()
    return output


def branch_name() -> str:
    value = _run_git(["branch", "--show-current"])
    return value or "unknown"


def git_status_short() -> str:
    value = _run_git(["status", "--short", "--branch"])
    return value or "clean"


def recent_commits(limit: int = 5) -> list[str]:
    output = _run_git(["log", f"-{limit}", "--pretty=format:%h %s"])
    if not output or output == "git unavailable":
        return []
    return [line for line in output.splitlines() if line.strip()]
