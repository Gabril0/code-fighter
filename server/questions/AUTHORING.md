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
```

The first two `tests/` cases are **public** (shown and downloadable). The
rest are private. If you set `generated_cases`, that many extra **private**
cases are produced per team, seeded from the team id, so no two teams get
the same anti-cheat cases. The generator and solver that produce them are
just commands you name in the manifest — they can live anywhere (a compiled
binary, a script in the folder, ...).

## How the shipped questions do it (Rust)

The three built-in questions share one small Rust binary, `qtool`
(`server/src/bin/qtool.rs`), which `cargo build --release` produces
alongside the server at `server/target/release/qtool`. It dispatches on its
arguments:

```
qtool <id> gen <seed>   # prints one input to stdout
qtool <id> solve        # reads an input on stdin, prints the expected output
```

so each question's manifest simply points at it with a path relative to the
question folder (the server canonicalizes the folder, so it resolves from any
working directory). To add a Rust question, add `q<id>_generate` /
`q<id>_solve` functions and two `match` arms in `qtool.rs`, rebuild, and add
the manifest entry below.

## Manifest entry

```json
{
  "id": "q6",
  "dir": "Q6-my-question",
  "title": "My Question",
  "difficulty": "intermediate",
  "points": 100,
  "mode": "numeric",
  "generated_cases": 100,
  "generator": ["../../target/release/qtool", "q6", "gen"],
  "solver":    ["../../target/release/qtool", "q6", "solve"]
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

`generator`/`solver` are just command lines, so anything works — the shipped
questions use the Rust `qtool` binary above. Compile your own and point at
it:

```json
"generator": ["./gen"],
"solver":    ["./solve"]
```

Bare names (`python3`, `node`) resolve on `PATH`; anything starting with `.`
or containing `/` is resolved against the question folder. A Python question,
for example, would seed with `random.seed(int(sys.argv[1]))` and read its
input on stdin.

## Quick scaffold

```
python3 tools/new_question.py q4 "Two Sum" --mode numeric
```

By default this scaffolds the **Rust** path used by the shipped questions. It:

- creates `server/questions/Q4-two-sum/` with a `statement.md` stub and a
  `tests/` sample,
- appends the manifest entry pointing at the `qtool` binary, and
- inserts stub `q4_generate` / `q4_solve` functions **and** the two `match`
  arms into `server/src/bin/qtool.rs`.

Then fill in the two Rust functions, rebuild, and restart:

```
cd server && cargo build --release
```

Validate the solver by checking it reproduces every sample in `tests/`:

```
for f in server/questions/Q4-*/tests/*.in; do
  diff <(server/target/release/qtool q4 solve < "$f") "${f%.in}.out" && echo "$f ok"
done
```

Prefer another language? Pass `--lang python` to drop
`generator.py`/`solver.py` stubs in the question folder instead (the engine
runs any executable named in the manifest).
