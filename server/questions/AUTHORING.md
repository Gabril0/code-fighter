# Authoring questions

A question is just a folder plus an entry in `manifest.json`. The server
reads them at boot — you never edit Rust to add a question. The engine
(mechanism) knows how to run things; each question supplies its own
generator and solver (policy).

## Folder layout

```
Q6-my-question/
  statement.md        # the problem statement shown to teams (enunciado.md also works)
  tests/              # sample/public cases: 01.in + 01.out, 02.in + 02.out, ...  (testes/ also works)
  generator.py        # OPTIONAL: prints one random test input to stdout
  solver.py           # OPTIONAL: reads an input on stdin, prints the expected output
```

The first two `tests/` cases are **public** (shown and downloadable). The
rest are private. If you set `generated_cases`, that many extra **private**
cases are produced per team, seeded from the team id, so no two teams get
the same anti-cheat cases.

## Manifest entry

```json
{
  "id": "q6",
  "dir": "Q6-my-question",
  "title": "My Question",
  "difficulty": "intermediária",
  "points": 100,
  "mode": "numeric",
  "generated_cases": 100,
  "generator": ["python3", "generator.py"],
  "solver":    ["python3", "solver.py"]
}
```

- `mode`: how answers are judged.
  - `numeric` — token-by-token, floats compared with a 0.01 tolerance.
  - `exact`   — text must match exactly (whitespace at line ends is ignored).
  - `json`    — parsed as JSON; numbers compared with tolerance, key order ignored.
  - `api`     — checked live against the team's running API (ships no cases, cannot generate).
- `generated_cases`, `generator`, `solver` are optional. Omit them for a
  purely static question (only the files in `tests/`). If you set
  `generated_cases`, you must provide both `generator` and `solver`.

## The generator/solver contract

The engine runs both programs **with the working directory set to the
question folder**, so relative paths (`./data.txt`) and `./binary` work.

- **generator**: invoked as `<cmd...> <seed>`; the same seed is also in the
  `CASE_SEED` environment variable. It must print exactly **one** test input
  to stdout, and be **deterministic** for a given seed (regenerated
  identically after a restart).
- **solver**: receives a test input on **stdin** and prints the **expected
  output** to stdout. It is the source of truth — it must match your
  statement's worked examples.

In Python, seed with `random.seed(int(sys.argv[1]))`.

## Any language

`generator`/`solver` are just command lines, so anything works. Compile a
binary and point at it:

```json
"generator": ["./gen"],
"solver":    ["./solve"]
```

Bare names (`python3`, `node`) resolve on `PATH`; anything starting with `.`
or containing `/` is resolved against the question folder.

## Quick scaffold

```
python3 tools/new_question.py q6 "My Question" --mode numeric
```

This creates the folder, a `statement.md` stub, a `tests/` sample, Python
`generator.py`/`solver.py` stubs, and appends the manifest entry. Fill in
the logic, then validate:

```
# a solver must reproduce every sample in tests/
for f in server/questions/Q6-*/tests/*.in; do
  diff <(python3 server/questions/Q6-*/solver.py < "$f") "${f%.in}.out" && echo "$f ok"
done
```
