# Hackaton 2/2026 — Boxing Leaderboard

Two pieces:

- **`server/`** — Rust. Holds the answers, the people, the teams and the score. Nothing secret
  ever reaches the browser.
- **`leaderboard/`** — React + Three.js. A boxing ring on the projector, plus a login, a profile
  page and an admin console.

Solving a question lands a punch and knocks out one of the *other* team's fighters, who is
carried out to rest while a teammate walks in.

## Run it

```bash
cd server && cargo run                          # http://localhost:8080
cd leaderboard && npm install && npm run dev    # http://localhost:5173
```

Point the frontend elsewhere with `VITE_API_URL=http://host:port npm run dev`.

On first start the server prints an **admin login hash** and stores it in the database. It is
reused on every later start, so grab it once:

```
  ┌──────────────────────────────────────────────┐
  │  admin login hash                            │
  │  admin-mrnhync7hv                            │
  └──────────────────────────────────────────────┘
```

Lost it? `sqlite3 server/data/hackaton.db "select token from users where role='admin'"`.
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
picture from the admin console too, so nobody is blocked waiting on a teammate — whoever writes
last wins. Pictures are cropped square and
downscaled to 256×256 JPEG *in the browser* before upload — a phone photo lands as a few KB — so
the state file stays small. The server rejects anything over ~300 KB.

## The admin console

Everything needed to run the event on one page, refreshing every 5s:

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

The five questions of the Code Fighter contest live in `server/questions/`, listed in
`manifest.json` and loaded at boot. Restart to pick up changes.

```json
{ "id": "q1", "dir": "Q1-patrimonio-liquido", "title": "Patrimônio Líquido",
  "difficulty": "aquecimento", "points": 100, "mode": "numeric" }
```

Each folder holds `enunciado.md` and `testes/NN.in` + `NN.out`. **Only the inputs ever leave the
server.** A team downloads a ZIP with the statement, every input and a `rodar.sh` helper, runs
their program once per input, and uploads the `.out` files. `extras` in the manifest is an
explicit allowlist of extra files to ship — anything not listed stays server-side, so dropping a
`solucao.py` into a question folder cannot leak it into a competitor's pack.

### Cases are generated per team

Each question serves **107–111 cases**: the curated ones from `testes/` first (the first two are
the statement's examples, so everyone gets them), then **100 generated from a seed derived from
the team id and the question id**. Q1–Q4 have a native generator and reference solver in
`src/reference/`, so the server produces both the input and the expected output — no Python at
event time.

The consequence that matters: the two teams get different inputs. Feeding one team's outputs to
the other scores only the shared curated prefix — 7/107 on Q1. Generation is deterministic, so a
team re-downloading its pack gets a byte-identical ZIP, and judging always reproduces the same
cases the team was given. Packs run 42 KB–490 KB and take under 60 ms to build.

The Rust solvers are checked against your `solucao.py`: they reproduce all 38 shipped gabaritos
exactly and agree over 12,000 generated cases per sweep. Two details keep them equivalent — the
Q4 ledger sums balances in first-integralisation order, because Python iterates its dict that way
and float addition is not associative, and it uses `powf` rather than repeated multiplication so
`(1 + taxa) ** dias` maps onto the same C `pow`.

The generators deliberately construct the shapes each trap needs: Q3 hands out ids independently
of settlement order and emits the file shuffled (so both the id and the credit-before-debit
tie-breaks are required), sometimes debits exactly the balance (so `>` fails where `>=` passes),
and Q2 duplicates its best day (so a `>=` argmax picks the wrong one). Q4 mixes long runs with
small amounts, where per-step rounding drifts past the absolute tolerance. Every wrong
implementation we tried is caught by at least 25 of the cases.

`mode` picks how the answer is compared:

| mode | used by | rule |
|---|---|---|
| `numeric` | Q1, Q2 | line by line, token by token; numbers match within **0.01 absolute** |
| `exact` | Q3 | text must match exactly — the question is all integer arithmetic |
| `json` | Q4 | parsed and compared semantically; numbers within **max(0.01, 1e-6 × expected)** |
| `api` | Q5 | no files: the team submits their API's URL and the server runs 28 live checks |

For Q5 the team can point the judge at a LAN address or at any `https://` endpoint — a
tunnel (`cloudflared tunnel --url http://localhost:8000`, `ngrok http 8000`) is the path the
pack recommends, since it keeps their own process alive and the question stores accounts in
memory. Serverless hosting breaks that: a correct solution scores 13/28 when state does not
survive between requests, so the pack says so explicitly. The client speaks TLS through
`rustls` and trusts the OS certificate store as well as the Mozilla bundle, which matters on a
corporate laptop behind TLS inspection — Python's `teste_api.py` cannot validate through it,
the judge can.

Trailing whitespace, a missing final newline, a UTF-8 BOM and `-0.00` vs `0.00` never decide a
verdict. Numbers are compared as scaled integers, not floats, so a value exactly one cent out
lands inside the window instead of on a rounding coin-flip.

The `api` mode is a hand-port of `teste_api.py` — same 28 checks, same order, same wording — so a
team gets the same score from the site as from running the script themselves.

**Questions unlock in order.** A team sees the ones it has solved plus the next one; the rest
answer `403` on the statement, the pack and the submission alike, so hiding them in the panel is
presentation, not the control. The organiser bypasses it. This deliberately overrides the
`t = 0` rule in the original notes — a team stuck on Q2 cannot skip ahead to Q3.

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
circle and composited over it. No photo leaves the drawn face. Fighters stand at a slight angle
rather than in profile so the face stays visible to the room. A photo wrapped on a sphere
stretches toward the edges — it reads clearly, but it is not a flat portrait.

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

## Deploying to Render

`render.yaml` describes both services — the Rust checker as a web service and the leaderboard
as a static site. Point Render at the repo as a Blueprint and fill the four secrets it asks for:

| variable | on | what |
|---|---|---|
| `ADMIN_TOKEN` | checker | the organiser hash. Set it, do not let it be random — see below |
| `SEED_JSON` | checker | the roster, pasted from `GET /v1/admin/export` |
| `ALLOWED_ORIGINS` | checker | the static site's URL |
| `VITE_API_URL` | leaderboard | the checker's URL |

The server binds `0.0.0.0:$PORT` when Render sets it; `BIND` still wins locally, so nothing about
running it on your own machine changes.

### Surviving an ephemeral disk

A free instance has no persistent disk. Every deploy and every platform restart hands the process
an empty filesystem, and with an empty database the server would mint a **new admin hash** and
every participant's login would stop existing — mid-event.

Two env vars remove that failure. `ADMIN_TOKEN` pins the organiser hash so a restart cannot lock
you out. `SEED_JSON` (or `SEED_FILE`) carries a roster that is imported whenever the database
comes up with no teams, so logins survive a wipe. Prefer `SEED_JSON`: the roster contains
everyone's login hash, so it belongs in Render's environment rather than in the repository.

Produce the seed once the match is set up:

```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" https://your-checker.onrender.com/v1/admin/export
```

The export is the full state — teams, people, solves and attempts — so re-seeding from a snapshot
taken mid-event restores the scoreboard too, not just the logins. Import only runs when there are
no teams, so it never overwrites a live match.

What the seed does **not** cover: spectators and the chat, which are rebuilt as people rejoin.

### Keeping it awake

A free instance sleeps after 15 minutes without traffic, and the first request after that takes
close to a minute. Point any external pinger at `/health` every 10 minutes — cron-job.org and
UptimeRobot both do it on a free plan. A ping keeps it warm, but it cannot prevent a redeploy or
a platform restart, which is exactly what the seed is for.

### Q5 changes shape

With the checker on the internet and the teams behind NAT, the judge cannot reach a LAN address.
A tunnel stops being a suggestion and becomes the only route, and the question pack says so.

## Things worth knowing

**State lives on the server**, in a SQLite database at `server/data/hackaton.db`. That is what
makes logins work: someone sets their picture on their own laptop and it appears on the
projector.

Every write is a transaction, with WAL journalling and `synchronous=FULL`, so a crash or a pulled
power cable cannot leave a half-written record — verified by `kill -9` mid-run with no loss. Back
it up by copying the `.db`, `.db-wal` and `.db-shm` files together, or with
`sqlite3 hackaton.db ".backup out.db"`.

An older `data/state.json` from the JSON-file era is imported automatically on first start and
kept as `state.json.imported`.

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
