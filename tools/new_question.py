#!/usr/bin/env python3
"""Scaffold a new question.

By default this scaffolds the **Rust** path used by the shipped questions:

    python3 tools/new_question.py q4 "Two Sum" --mode numeric

That creates server/questions/Q4-two-sum/ (statement stub + a sample test),
appends the manifest entry pointing at the `qtool` binary, and inserts stub
`q4_generate` / `q4_solve` functions plus the two `match` arms into
server/src/bin/qtool.rs. Fill in the two Rust functions, run
`cd server && cargo build --release`, and restart the server.

Pass `--lang python` to instead drop `generator.py` / `solver.py` stubs in the
question folder (the engine runs any executable), wired into the manifest.
"""
import argparse
import json
import re
import sys
from pathlib import Path

MODES = ["numeric", "exact", "json", "api"]

ROOT = Path(__file__).resolve().parent.parent
QUESTIONS_DIR = ROOT / "server" / "questions"
MANIFEST = QUESTIONS_DIR / "manifest.json"
QTOOL = ROOT / "server" / "src" / "bin" / "qtool.rs"

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

PY_GENERATOR = '''#!/usr/bin/env python3
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

PY_SOLVER = '''#!/usr/bin/env python3
"""Reads a test input on stdin, prints the expected output to stdout."""
import sys


def main():
    data = sys.stdin.read().strip()

    # TODO: compute and print the expected answer for `data`.
    print(data)


if __name__ == "__main__":
    main()
'''

RUST_FUNCTIONS = '''// ---------------------------------------------------------------------------
// {qid} — {title} (mode: {mode})
// ---------------------------------------------------------------------------

fn {qid}_generate(rng: &mut Rng) -> String {{
    // TODO: build a random case using `rng`, return exactly one input.
    let value = rng.between(1, 100);
    format!("{{value}}\\n")
}}

fn {qid}_solve(input: &str) -> String {{
    // TODO: compute and return the expected output for `input`.
    format!("{{}}\\n", input.trim())
}}

'''


def slug(text):
    text = text.lower()
    for accented, plain in {
        "á": "a", "ã": "a", "â": "a", "é": "e", "ê": "e", "í": "i",
        "ó": "o", "ô": "o", "õ": "o", "ú": "u", "ç": "c",
    }.items():
        text = text.replace(accented, plain)
    text = re.sub(r"[^a-z0-9]+", "-", text).strip("-")
    return text or "question"


def insert_before_marker(text, marker, snippet):
    if marker not in text:
        sys.exit(f"could not find marker {marker!r} in qtool.rs — insert manually")
    return text.replace(marker, snippet + marker, 1)


def scaffold_rust(qid, entry, title, mode):
    src = QTOOL.read_text()
    if f"fn {qid}_generate" in src:
        sys.exit(f"qtool.rs already defines {qid}_generate")

    src = insert_before_marker(
        src,
        "// ---------------------------------------------------------------------------\n"
        "// SCAFFOLD:FUNCTIONS",
        RUST_FUNCTIONS.format(qid=qid, title=title, mode=mode),
    )
    src = insert_before_marker(
        src,
        "// SCAFFOLD:GEN_ARMS",
        f'"{qid}" => {qid}_generate(&mut rng),\n                ',
    )
    src = insert_before_marker(
        src,
        "// SCAFFOLD:SOLVE_ARMS",
        f'"{qid}" => {qid}_solve(&input),\n                ',
    )
    QTOOL.write_text(src)

    entry["generator"] = ["../../target/release/qtool", qid, "gen"]
    entry["solver"] = ["../../target/release/qtool", qid, "solve"]
    print(f"inserted {qid}_generate / {qid}_solve + match arms into {QTOOL.relative_to(ROOT)}")


def scaffold_python(qid, entry, folder):
    (folder / "generator.py").write_text(PY_GENERATOR)
    (folder / "solver.py").write_text(PY_SOLVER)
    entry["generator"] = ["python3", "generator.py"]
    entry["solver"] = ["python3", "solver.py"]
    print(f"wrote generator.py / solver.py stubs in {folder.relative_to(ROOT)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("id", help="question id, e.g. q4")
    ap.add_argument("title", help='human title, e.g. "Two Sum"')
    ap.add_argument("--mode", choices=MODES, default="numeric")
    ap.add_argument("--lang", choices=["rust", "python"], default="rust",
                    help="where the generator/solver live (default: rust, via qtool)")
    ap.add_argument("--difficulty", default="intermediate")
    ap.add_argument("--points", type=int, default=100)
    ap.add_argument("--generated-cases", type=int, default=100,
                    help="private per-team cases (0 for static-only)")
    args = ap.parse_args()

    manifest = json.loads(MANIFEST.read_text())
    if any(e["id"] == args.id for e in manifest):
        sys.exit(f"question id {args.id!r} already exists in the manifest")

    folder_name = f"{args.id.upper()}-{slug(args.title)}"
    folder = QUESTIONS_DIR / folder_name
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
        entry["generated_cases"] = args.generated_cases
        if args.lang == "rust":
            scaffold_rust(args.id, entry, args.title, args.mode)
        else:
            scaffold_python(args.id, entry, folder)

    manifest.append(entry)
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")

    print(f"created {folder.relative_to(ROOT)}")
    print(f"appended {args.id} to {MANIFEST.relative_to(ROOT)}")
    if args.mode == "api":
        print("next: this is an api question — no generator/solver needed")
    elif args.lang == "rust":
        print("next: fill in the two Rust functions in qtool.rs, then:")
        print("      cd server && cargo build --release  (and restart the server)")
    else:
        print("next: fill in generator.py and solver.py, then restart the server")


if __name__ == "__main__":
    main()
