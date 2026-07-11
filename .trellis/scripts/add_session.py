import argparse
from datetime import datetime

from common.developer import get_developer, initialize_developer, workspace_dir


parser = argparse.ArgumentParser()
parser.add_argument("summary")
args = parser.parse_args()

developer = get_developer()
initialize_developer(developer)
journal = workspace_dir(developer) / "journal-1.md"
with journal.open("a", encoding="utf-8") as handle:
    handle.write(f"- {datetime.now().isoformat(timespec='seconds')}: {args.summary}\n")

print(str(journal))
