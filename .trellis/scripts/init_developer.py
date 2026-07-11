import argparse

from common.developer import default_developer, initialize_developer


parser = argparse.ArgumentParser()
parser.add_argument("developer", nargs="?", default=default_developer())
args = parser.parse_args()

path = initialize_developer(args.developer)
print(str(path))
