# Code Fighter

A gamified programming-contest scoreboard. Teams solve coding problems; every correct solution
throws a punch and knocks out one of the *other* team's fighters in a live boxing ring on the
projector.

Two pieces:

- **`server/`** — Rust. The judge, the store, and the API. It holds the answers, the people, the
  teams and the score. Nothing secret ever reaches the browser.
- **`leaderboard/`** — React + Three.js + TypeScript. The boxing ring for the projector, plus a
  login, a profile page and an admin console.

## Run it

```bash
cd server && cargo run                          # http://localhost:8080
cd leaderboard && npm install && npm run dev    # http://localhost:5173
```

Point the frontend at a different backend with `VITE_API_URL=http://host:port npm run dev`.

On first start the server prints an **admin login hash** and stores it in the database. It is
reused on every later start, so grab it once:

```
  ┌──────────────────────────────────────────────┐
  │  admin login hash                            │
  │  admin-mrnhync7hv                            │
  └──────────────────────────────────────────────┘
```

Lost it? `sqlite3 server/data/code-fighter.db "select token from users where role='admin'"`.
Delete the database to start a fresh event.

## The screens

| Route | Who | What |
|---|---|---|
| `#/` | anyone, no login | the ring — this is the projector view |
| `#/login` | anyone | paste a login hash |
| `#/me` | any signed-in person | set your own name and picture |
| `#/admin` | organiser only | orchestrate everything |

**Login is just a hash.** You create one per person in the admin console, hand it over, they
paste it. No passwords, no email. First time someone signs in they land on their profile page;
after that they go straight to the ring.

**Profile** is deliberately tiny: a name and a picture. The organiser can set or replace anyone's
picture from the admin console too, so nobody is blocked waiting on a teammate. Pictures are
cropped square and downscaled to 256×256 JPEG *in the browser* before upload — a phone photo lands
as a few KB — so the state stays small. The server rejects anything over ~300 KB.

## The admin console

Everything needed to run the event on one page, refreshing every 5s:

- **Championship name** — set the title shown across the app.
- **Teams** — create up to two, rename inline, set the colour with a picker, upload an icon,
  delete. The colour drives that team's fighters, ropes and HUD.
- **People** — create a login and get their hash immediately (click to copy). Rename, assign or
  move between teams from a dropdown, upload or replace their picture, remove.
- **Profile column** — shows `waiting` until that person has signed in and set themselves up, so
  you can see at a glance who still needs chasing.
- **Lock submissions** — freeze the match without stopping the server.
- **Reset match** — clears every solve and attempt, keeps people and teams.

Anyone can submit for their own team once assigned; the organiser can submit for either team.

## The questions

Questions live in `server/questions/`, are listed in `manifest.json`, and are loaded at boot
(restart to pick up changes). The three shipped problems are classic computer-science warm-ups:

| id | title | mode | idea |
|---|---|---|---|
| q1 | Maximum Subarray Sum | `numeric` | Kadane's algorithm — largest contiguous sum |
| q2 | FizzBuzz | `exact` | print 1..N with Fizz / Buzz / FizzBuzz |
| q3 | Word Frequency Count | `json` | count each word, return a JSON object |

A question is a folder plus a manifest entry — **you never edit the judge to add one.** Each
folder holds a `statement.md` and `tests/NN.in` + `NN.out` public cases:

```
server/questions/Q1-maximum-subarray/
  statement.md          # the problem shown to teams
  tests/
    01.in  01.out       # the first two cases double as the statement's examples
    02.in  02.out
    ...
```

The manifest entry ties it together:

```json
{ "id": "q1", "dir": "Q1-maximum-subarray", "title": "Maximum Subarray Sum",
  "difficulty": "warm-up", "points": 100, "mode": "numeric",
  "generated_cases": 100,
  "generator": ["../../target/release/qtool", "q1", "gen"],
  "solver":    ["../../target/release/qtool", "q1", "solve"] }
```

**Only inputs ever leave the server.** A team downloads a ZIP with the statement, every input and
a `run.sh` helper, runs their program once per input, and uploads the `.out` files. `extras` in
the manifest is an explicit allowlist of extra files to ship — anything not listed stays
server-side, so dropping a solution into a question folder cannot leak it into a competitor's pack.

## Creating a new question

The engine is generic: it knows how to run a program, seed it, judge the output and cache the
result — nothing about any specific problem. Each question supplies its own **generator** (builds
a random input from a seed) and **solver** (computes the expected output for an input). The
shipped questions implement both in one small Rust binary, `qtool` (`server/src/bin/qtool.rs`).

**One command scaffolds everything:**

```bash
python3 tools/new_question.py q4 "Two Sum" --mode numeric
```

That will:

1. create `server/questions/Q4-two-sum/` with a `statement.md` stub and a sample `tests/01.in` +
   `01.out`,
2. append the manifest entry pointing at `qtool`, and
3. insert stub `q4_generate` / `q4_solve` functions **and** the two `match` arms into
   `server/src/bin/qtool.rs`.

Then finish the question:

1. **Write the statement** in `server/questions/Q4-two-sum/statement.md`.
2. **Fill in the two Rust functions** in `qtool.rs`. `q4_generate(rng)` returns one input string;
   `q4_solve(input)` returns the expected output. Both must be deterministic for a given seed —
   use the provided `Rng` (SplitMix64) for all randomness.
3. **Rebuild:** `cd server && cargo build --release` (this builds both the server and `qtool`).
4. **Add public cases:** replace the sample `tests/*.in` / `*.out` with real ones (the first two
   are shown in the statement). Verify your solver reproduces every one:
   ```bash
   for f in server/questions/Q4-*/tests/*.in; do
     diff <(server/target/release/qtool q4 solve < "$f") "${f%.in}.out" && echo "$f ok"
   done
   ```
5. **Restart the server.** The new question appears automatically.

Prefer another language? Pass `--lang python` and the scaffolder drops `generator.py` /
`solver.py` stubs in the question folder instead — the engine runs any executable named in the
manifest, so a compiled binary, a script, anything works. The contract is tiny: the generator is
invoked as `<cmd> <seed>` (the seed is also in the `CASE_SEED` env var) and prints one **input**
to stdout; the solver reads an input on **stdin** and prints the expected **output**. Both run
with the question folder as their working directory. See `server/questions/AUTHORING.md` for the
full reference.

### Cases are generated per team

Each question serves its curated `tests/` cases first (the first two are the statement's
examples, so everyone gets them), then **`generated_cases` inputs built from a seed derived from
the team id and the question id**. The engine runs the generator to build each input and the
solver to compute its expected output, so no answer key is ever shipped.

The consequence that matters: the two teams get different inputs. Feeding one team's outputs to
the other scores only the shared curated prefix. Generation is deterministic, so re-downloading a
pack yields the same cases and judging always reproduces them. Because 100 cases mean 200
subprocess calls, each team's set is built once and **cached** — the first pack takes a few
seconds, later ones are instant — and the cache is cleared on **Reset match**.

### Judge modes

`mode` picks how the answer is compared:

| mode | rule |
|---|---|
| `numeric` | line by line, token by token; numbers match within **0.01 absolute** |
| `exact` | text must match exactly (trailing whitespace and final newline are ignored) |
| `json` | parsed and compared semantically; numbers within **max(0.01, 1e-6 × expected)** |
| `api` | no files: the team submits their API's URL and the server runs live checks |

Trailing whitespace, a missing final newline, a UTF-8 BOM and `-0.00` vs `0.00` never decide a
verdict. Numbers are compared as scaled integers, not floats, so a value exactly one cent out
lands inside the window instead of on a rounding coin-flip.

**Questions unlock in order.** A team sees the ones it has solved plus the next one; the rest
answer `403` on the statement, the pack and the submission alike, so hiding them in the panel is
presentation, not the control. The organiser bypasses it — a team stuck on q2 cannot skip to q3.

**A wrong submission costs nothing.** It bumps the attempt counter, ragdolls that team's fighter
on every screen watching, and leaves the score untouched. Hidden cases never reveal the expected
value: a failing public case shows the diff, a hidden one only says which line went wrong.

## The ring

Roster size follows each team's member count. With two people a side, **two knockouts wipes a
team out** and the survivor breaks into a victory dance.

What a solve triggers, in order:

1. The scoring team's fighter throws a **Punch**.
2. The opposing fighter plays **Death** and drops.
3. They fade out and reappear behind the ring, **Sitting**, ghosted to 40%.
4. The next fighter **Walks** in from the corner and settles into **Idle**.

Position carries the state: bright and in the ring is fighting, standing at the side is waiting,
ghosted behind the ropes is knocked out. The HUD mirrors it — a downed member's photo goes
greyscale with an "out" tag. `focus mode` hides the panel for the projector.

### The fighters

Mii-style characters built from Three.js primitives at runtime — round head, hair cap, rounded
body, detached floating hands and feet. No model files, nothing to license. Detached limbs mean
no skeleton, so every pose is direct transforms computed per frame in `pose()`.

**Member photos map onto the faces.** Each head carries its own canvas texture; the face is drawn
into the front-facing region of an equirectangular layout, and an uploaded photo is clipped to a
circle and composited over it. Fighters stand at a slight angle rather than in profile so the
face stays visible to the room.

## API

| | | |
|---|---|---|
| `GET` | `/health` | counts, no secrets |
| `GET` | `/v1/questions` | `[{id, title, points, difficulty, mode, cases}]` |
| `GET` | `/v1/questions/{id}` | statement + public examples — hidden cases stay `null` |
| `GET` | `/v1/questions/{id}/pack` | ZIP of the statement and every input, generated for the caller's team |
| `GET` | `/v1/board` | teams, members, photos, scores — **no answers, no hashes** |
| `POST` | `/v1/auth` | `{token}` → who that hash belongs to |
| `GET` `PATCH` | `/v1/me` | read / update your own name and picture |
| `POST` | `/v1/submissions` | `{question_id, outputs{} \| api_url, team_id?}` → `{correct, passed, total, cases[]}` |
| `GET` | `/v1/admin/state` | everything, including every login hash |
| `POST` `PATCH` `DELETE` | `/v1/admin/users`, `/v1/admin/teams` | orchestration |
| `POST` | `/v1/admin/lock`, `/v1/admin/reset` | match control |

Admin routes return `403` for a participant token.

## Deploying

The backend is a single Rust binary that serves the API; the frontend is a static bundle
(`cd leaderboard && npm run build` → `dist/`). Any host that can run a binary and serve static
files works. The server binds `0.0.0.0:$PORT` when `PORT` is set (so it drops onto most PaaS
hosts as-is); `BIND` still wins locally.

Environment variables:

| variable | on | what |
|---|---|---|
| `ADMIN_TOKEN` | backend | pins the organiser hash so a restart cannot lock you out |
| `SEED_JSON` (or `SEED_FILE`) | backend | a roster imported when the database comes up with no teams |
| `ALLOWED_ORIGINS` | backend | the frontend's URL, for CORS |
| `DB_FILE` | backend | database path (default `data/code-fighter.db`) |
| `VITE_API_URL` | frontend | the backend's URL (set at build time) |

### Surviving an ephemeral disk

On a host with no persistent disk, every deploy hands the process an empty filesystem — and with
an empty database the server would mint a **new admin hash** and every participant's login would
stop existing, mid-event. Two env vars remove that: `ADMIN_TOKEN` pins the organiser hash, and
`SEED_JSON` carries a roster that is imported whenever the database comes up with no teams. Prefer
`SEED_JSON` (it contains everyone's login hash, so it belongs in the host's environment, not the
repo). Produce the seed once the match is set up:

```bash
curl -H "Authorization: ******" https://your-backend.example.com/v1/admin/export
```

The export is the full state — teams, people, solves and attempts — so re-seeding from a mid-event
snapshot restores the scoreboard too, not just the logins. Import only runs when there are no
teams, so it never overwrites a live match. Spectators and chat are not covered; they rebuild as
people rejoin.

### Keeping a free instance awake

Free instances often sleep after ~15 minutes without traffic, and the first request after that is
slow. Point any external pinger (cron-job.org, UptimeRobot) at `/health` every ~10 minutes. A
ping keeps it warm but cannot prevent a redeploy or platform restart — that is what the seed is
for.

## Things worth knowing

**State lives on the server**, in a SQLite database at `server/data/code-fighter.db`. That is
what makes logins work: someone sets their picture on their own laptop and it appears on the
projector.

Every write is a transaction, with WAL journalling and `synchronous=FULL`, so a crash or a pulled
power cable cannot leave a half-written record — verified by `kill -9` mid-run with no loss. Back
it up by copying the `.db`, `.db-wal` and `.db-shm` files together, or with
`sqlite3 code-fighter.db ".backup out.db"`.

**A solve is recorded once.** Re-submitting a solved question returns `{"correct": true}` without
changing the score, so a double-click cannot double-count.

**Points are frozen at solve time.** Editing a question's points later does not retro-change a
score already awarded.

**The board polls every 2.5s** rather than using a live stream. Knockouts therefore fire up to
2.5s after the submission — fine on a projector, and it means a laptop submitting from anywhere
shows up on the big screen with no extra plumbing.

**Screenshots need the canvas frozen first.** The scene renders every frame, so tools that wait
for a stable frame never settle. The renderer runs with `preserveDrawingBuffer`, so
`canvas.toDataURL()` works — swap in that image, then capture.
