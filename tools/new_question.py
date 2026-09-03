#!/usr/bin/env python3
"""Scaffold a new question folder + manifest entry.

    python3 tools/new_question.py q6 "My Question" --mode numeric

Creates server/questions/Q6-my-question/ with a statement stub, a sample
test case, Python generator/solver stubs, and appends the manifest entry.
Fill in the logic, then validate the solver against the sample cases.
"""
import argparse
import json
import re
import sys
from pathlib import Path

MODES = ["numeric", "exact", "json", "api"]

STATEMENT = """# {title}

Describe the problem here.

## Input

Describe the input format.

## Output

Describe the expected output.

## Example

Input:
```
{sample_in}```

Output:
```
{sample_out}```
"""

GENERATOR = '''#!/usr/bin/env python3
"""Prints one random test input to stdout, deterministic for a given seed."""
import os
import random
import sys


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else int(os.environ.get("CASE_SEED", "0"))
    rng = random.Random(seed)

    # TODO: build a random case using rng, print exactly one input.
    print(rng.randint(1, 100))


if __name__ == "__main__":
    main()
'''

SOLVER = '''#!/usr/bin/env python3
"""Reads a test input on stdin, prints the expected output to stdout."""
import sys


def main():
    data = sys.stdin.read().strip()

    # TODO: compute and print the expected answer for `data`.
    print(data)


if __name__ == "__main__":
    main()
'''


def slug(text):
    text = text.lower()
    text = (text.replace("á", "a").replace("ã", "a").replace("â", "a")
                .replace("é", "e").replace("ê", "e").replace("í", "i")
                .replace("ó", "o").replace("ô", "o").replace("õ", "o")
                .replace("ú", "u").replace("ç", "c"))
    text = re.sub(r"[^a-z0-9]+", "-", text).strip("-")
    return text or "question"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("id", help="question id, e.g. q6")
    ap.add_argument("title", help="human title, e.g. \"My Question\"")
    ap.add_argument("--mode", choices=MODES, default="numeric")
    ap.add_argument("--difficulty", default="intermediária")
    ap.add_argument("--points", type=int, default=100)
    ap.add_argument("--generated-cases", type=int, default=100,
                    help="private per-team cases (0 for static-only)")
    args = ap.parse_args()

    root = Path(__file__).resolve().parent.parent / "server" / "questions"
    manifest_path = root / "manifest.json"
    manifest = json.loads(manifest_path.read_text())

    if any(e["id"] == args.id for e in manifest):
        sys.exit(f"question id {args.id!r} already exists")

    folder_name = f"{args.id.upper()}-{slug(args.title)}"
    folder = root / folder_name
    if folder.exists():
        sys.exit(f"folder {folder} already exists")

    tests = folder / "tests"
    tests.mkdir(parents=True)

    sample_in = "1\n"
    sample_out = "1\n"
    (tests / "01.in").write_text(sample_in)
    (tests / "01.out").write_text(sample_out)
    (folder / "statement.md").write_text(
        STATEMENT.format(title=args.title, sample_in=sample_in, sample_out=sample_out))

    entry = {
        "id": args.id,
        "dir": folder_name,
        "title": args.title,
        "difficulty": args.difficulty,
        "points": args.points,
        "mode": args.mode,
    }

    if args.mode != "api" and args.generated_cases > 0:
        (folder / "generator.py").write_text(GENERATOR)
        (folder / "solver.py").write_text(SOLVER)
        entry["generated_cases"] = args.generated_cases
        entry["generator"] = ["python3", "generator.py"]
        entry["solver"] = ["python3", "solver.py"]

    manifest.append(entry)
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")

    print(f"created {folder}")
    print(f"appended {args.id} to manifest.json")
    print("next: edit statement.md, tests/, generator.py and solver.py")


if __name__ == "__main__":
    main()
