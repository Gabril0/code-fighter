use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State as AxumState};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use axum::http::{HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::model::{Role, State, Team, User};
use crate::questions::{self, Case, Mode, Question};
use crate::store::{now_millis, random_id, Store};
use crate::zip::ZipBuilder;
use crate::{apicheck, judge};

pub struct Ctx {
    pub store: Store,
    pub questions: Vec<Question>,
    /// Cache of the fully-materialised case list per (team, question). Building
    /// generated cases shells out to the question's generator/solver, so we do
    /// it once per team and reuse the result until the contest is reset.
    generated: Mutex<HashMap<(String, String), Arc<Vec<Case>>>>,
}

impl Ctx {
    pub fn new(store: Store, questions: Vec<Question>) -> Self {
        Self {
            store,
            questions,
            generated: Mutex::new(HashMap::new()),
        }
    }

    /// Return the ordered case list for a team, generating and caching the
    /// per-team cases on first use.
    fn cases_for(&self, question: &Question, team_id: &str) -> anyhow::Result<Arc<Vec<Case>>> {
        let key = (team_id.to_string(), question.id.clone());
        if let Some(cached) = self.generated.lock().unwrap().get(&key).cloned() {
            return Ok(cached);
        }
        let cases = Arc::new(questions::cases_for(question, team_id)?);
        self.generated.lock().unwrap().insert(key, cases.clone());
        Ok(cases)
    }

    /// Drop every cached case list (used when the contest is reset).
    fn clear_generated(&self) {
        self.generated.lock().unwrap().clear();
    }
}

type Shared = Arc<Ctx>;
type ApiError = (StatusCode, Json<Value>);

fn err(status: StatusCode, message: &str) -> ApiError {
    (status, Json(json!({ "error": message })))
}

fn oops(problem: impl std::fmt::Display) -> ApiError {
    err(StatusCode::INTERNAL_SERVER_ERROR, &problem.to_string())
}

fn load(ctx: &Shared) -> Result<State, ApiError> {
    ctx.store.read().map_err(oops)
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    Some(raw.trim_start_matches("Bearer ").trim().to_string())
}

fn caller(ctx: &Shared, headers: &HeaderMap) -> Result<User, ApiError> {
    let token = bearer(headers).ok_or_else(|| err(StatusCode::UNAUTHORIZED, "faltou o token"))?;
    load(ctx)?
        .user_by_token(&token)
        .cloned()
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "esse hash de login não existe"))
}

#[derive(Clone)]
struct Spectator {
    id: String,
    token: String,
    name: String,
    photo: Option<String>,
    seat: i64,
}

fn spectator_by_token(ctx: &Shared, token: &str) -> Result<Option<Spectator>, ApiError> {
    ctx.store
        .with_connection(|tx| {
            let mut stmt = tx.prepare(
                "SELECT id, token, name, photo, seat FROM spectators WHERE token = ?1",
            )?;
            let mut rows = stmt.query(params![token])?;
            if let Some(row) = rows.next()? {
                return Ok(Some(Spectator {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    name: row.get(2)?,
                    photo: row.get(3)?,
                    seat: row.get(4)?,
                }));
            }
            Ok(None)
        })
        .map_err(oops)
}

enum Viewer {
    Member(User),
    Watcher(Spectator),
}

impl Viewer {
    fn id(&self) -> String {
        match self {
            Viewer::Member(user) => user.id.clone(),
            Viewer::Watcher(spectator) => spectator.id.clone(),
        }
    }

    fn name(&self) -> String {
        match self {
            Viewer::Member(user) => user.name.clone(),
            Viewer::Watcher(spectator) => spectator.name.clone(),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Viewer::Member(_) => "member",
            Viewer::Watcher(_) => "spectator",
        }
    }

    fn is_admin(&self) -> bool {
        matches!(self, Viewer::Member(user) if user.role == Role::Admin)
    }
}

fn viewer(ctx: &Shared, headers: &HeaderMap) -> Result<Viewer, ApiError> {
    let token = bearer(headers).ok_or_else(|| err(StatusCode::UNAUTHORIZED, "faltou o token"))?;
    if let Some(user) = load(ctx)?.user_by_token(&token).cloned() {
        return Ok(Viewer::Member(user));
    }
    if let Some(spectator) = spectator_by_token(ctx, &token)? {
        return Ok(Viewer::Watcher(spectator));
    }
    Err(err(StatusCode::UNAUTHORIZED, "token desconhecido"))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Effects {
    pub pixelate: bool,
    pub crt: bool,
    pub aberration: bool,
    pub bloom: bool,
}

impl Default for Effects {
    fn default() -> Self {
        Self { pixelate: true, crt: true, aberration: true, bloom: true }
    }
}

fn load_effects(ctx: &Shared, owner: &str) -> Result<Effects, ApiError> {
    ctx.store
        .with_connection(|tx| {
            tx.query_row(
                "SELECT pixelate, crt, aberration, bloom FROM effects WHERE owner_id = ?1",
                params![owner],
                |row| {
                    Ok(Effects {
                        pixelate: row.get::<_, i64>(0)? != 0,
                        crt: row.get::<_, i64>(1)? != 0,
                        aberration: row.get::<_, i64>(2)? != 0,
                        bloom: row.get::<_, i64>(3)? != 0,
                    })
                },
            )
            .or_else(|problem| match problem {
                rusqlite::Error::QueryReturnedNoRows => Ok(Effects::default()),
                other => Err(other),
            })
        })
        .map_err(oops)
}

fn save_effects(ctx: &Shared, owner: &str, effects: Effects) -> Result<(), ApiError> {
    ctx.store
        .with_connection(|tx| {
            tx.execute(
                "INSERT INTO effects (owner_id, pixelate, crt, aberration, bloom)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(owner_id) DO UPDATE SET
                     pixelate = excluded.pixelate,
                     crt = excluded.crt,
                     aberration = excluded.aberration,
                     bloom = excluded.bloom",
                params![
                    owner,
                    effects.pixelate as i64,
                    effects.crt as i64,
                    effects.aberration as i64,
                    effects.bloom as i64
                ],
            )
        })
        .map_err(oops)?;
    Ok(())
}

async fn update_effects(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<Effects>,
) -> Result<Json<Value>, ApiError> {
    let owner = match viewer(&ctx, &headers)? {
        Viewer::Member(user) => user.id,
        Viewer::Watcher(spectator) => spectator.id,
    };
    save_effects(&ctx, &owner, payload)?;
    Ok(Json(json!(payload)))
}

fn spectator_view(spectator: &Spectator, effects: Effects) -> Value {
    json!({
        "id": spectator.id,
        "token": spectator.token,
        "name": spectator.name,
        "photo": spectator.photo,
        "seat": spectator.seat,
        "kind": "spectator",
        "effects": effects,
    })
}

fn admin(ctx: &Shared, headers: &HeaderMap) -> Result<User, ApiError> {
    let user = caller(ctx, headers)?;
    if user.role != Role::Admin {
        return Err(err(StatusCode::FORBIDDEN, "somente o organizador"));
    }
    Ok(user)
}

fn user_view(user: &User, team: Option<&Team>, effects: Effects) -> Value {
    json!({
        "id": user.id,
        "effects": effects,
        "name": user.name,
        "photo": user.photo,
        "role": user.role,
        "team_id": user.team_id,
        "profile_set": user.profile_set,
        "team": team.map(|t| json!({ "id": t.id, "name": t.name, "color": t.color })),
    })
}

/// Absent leaves the field alone; `null` clears it; a value replaces it.
///
/// Serde maps an explicit `null` onto `None` for a plain `Option<T>`, which makes
/// "clear this" indistinguishable from "left it out". Deserialising through this
/// keeps the two apart.
fn nullable<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

type PhotoPatch = Option<Option<String>>;

fn validate_photo(photo: &PhotoPatch) -> Result<(), ApiError> {
    if let Some(Some(value)) = photo {
        if value.len() > 400_000 {
            return Err(err(
                StatusCode::PAYLOAD_TOO_LARGE,
                "imagem grande demais, use algo abaixo de ~300 KB",
            ));
        }
    }
    Ok(())
}

fn apply_photo(target: &mut Option<String>, photo: PhotoPatch) {
    if let Some(value) = photo {
        *target = value.filter(|v| !v.trim().is_empty());
    }
}

// ---------------------------------------------------------------- public

async fn health(AxumState(ctx): AxumState<Shared>) -> Result<Json<Value>, ApiError> {
    let state = load(&ctx)?;
    Ok(Json(json!({
        "status": "ok",
        "questions": ctx.questions.len(),
        "teams": state.teams.len(),
        "users": state.users.len(),
        "locked": state.locked,
    })))
}

fn question_summary(question: &Question) -> Value {
    json!({
        "id": question.id,
        "title": question.title,
        "points": question.points,
        "difficulty": question.difficulty,
        "mode": question.mode,
        "cases": questions::total_cases(question),
    })
}

fn resolve_team(user: &User, requested: Option<String>) -> Result<String, ApiError> {
    match (&user.role, requested, user.team_id.clone()) {
        (Role::Admin, Some(id), _) => Ok(id),
        (Role::Admin, None, None) => Err(err(
            StatusCode::BAD_REQUEST,
            "escolha um time para esta questão",
        )),
        (_, _, Some(id)) => Ok(id),
        _ => Err(err(StatusCode::BAD_REQUEST, "você ainda não está em um time")),
    }
}

async fn list_questions(AxumState(ctx): AxumState<Shared>) -> Json<Vec<Value>> {
    Json(ctx.questions.iter().map(question_summary).collect())
}

fn solved_prefix(ctx: &Shared, state: &State, team_id: &str) -> usize {
    ctx.questions
        .iter()
        .take_while(|question| state.solve(team_id, &question.id).is_some())
        .count()
}

fn guard_unlocked(
    ctx: &Shared,
    state: &State,
    user: &User,
    team_id: &str,
    question_id: &str,
) -> Result<(), ApiError> {
    if user.role == Role::Admin {
        return Ok(());
    }
    let position = ctx
        .questions
        .iter()
        .position(|question| question.id == question_id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "questão desconhecida"))?;
    if position <= solved_prefix(ctx, state, team_id) {
        return Ok(());
    }
    Err(err(
        StatusCode::FORBIDDEN,
        "resolva a questão anterior para liberar esta",
    ))
}

fn find_question<'a>(ctx: &'a Shared, id: &str) -> Result<&'a Question, ApiError> {
    ctx.questions
        .iter()
        .find(|question| question.id == id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "questão desconhecida"))
}

async fn question_detail(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let user = caller(&ctx, &headers)?;
    let state = load(&ctx)?;
    let team_id = resolve_team(&user, None).unwrap_or_default();
    guard_unlocked(&ctx, &state, &user, &team_id, &id)?;
    let question = find_question(&ctx, &id)?;
    let cases: Vec<Value> = questions::case_names(question)
        .into_iter()
        .enumerate()
        .map(|(index, name)| json!({ "name": name, "public": index < questions::PUBLIC_CASES }))
        .collect();

    let mut detail = question_summary(question);
    detail["statement"] = json!(question.statement);
    detail["caseList"] = json!(cases);
    Ok(Json(detail))
}

fn how_to_send(question: &Question, case_count: usize) -> String {
    if question.mode == Mode::Api {
        return format!(
            "# How to submit: {}\n\n\
             This question is checked against your running API. The scoreboard lives on the\n\
             internet and your machine does not, so it can only reach you through a public\n\
             address, a tunnel. Pasting `http://localhost:8000` or your LAN IP **won't work**.\n\n\
             ## 1. Start the API as usual\n\n\
             ```bash\n\
             uvicorn main:app --reload\n\
             ```\n\n\
             ## 2. Open a tunnel, in another terminal\n\n\
             ```bash\n\
             cloudflared tunnel --url http://localhost:8000\n\
             ```\n\n\
             Install it with `brew install cloudflared`. No sign-up required. It prints\n\
             something like `https://something-random.trycloudflare.com`. That address is what\n\
             goes on the scoreboard.\n\n\
             If you prefer ngrok (`brew install ngrok`, needs a free account):\n\n\
             ```bash\n\
             ngrok http 8000\n\
             ```\n\n\
             The process is still yours, so in-memory accounts keep working normally.\n\
             Leave the tunnel open until the scoreboard accepts it. If it drops, the address changes.\n\n\
             ## 3. Before submitting, open the URL in your browser\n\n\
             The tunnel takes a few seconds to propagate. If you submit immediately, the first\n\
             checks may fail for a reason that isn't your code.\n\n\
             ## Checking beforehand\n\n\
             There are 28 checks and all of them must pass. Run it locally as many times as you\n\
             like, straight against your server, without the tunnel:\n\n\
             ```bash\n\
             python3 test_api.py http://127.0.0.1:8000\n\
             ```\n\n\
             ## If it goes wrong\n\n\
             - **`could not connect`**: the tunnel dropped, or you pasted the old address.\n\
             - **Many checks returning 404 right after one that passed**: that's state that\n\
               didn't survive between requests. It happens if you deployed to *serverless* (Vercel\n\
               Functions, Lambda) instead of using the tunnel: each request lands on a different\n\
               instance and the account you just created doesn't exist on the next call.\n\
               A perfect solution loses 13 of 28 this way.\n",
            question.title
        );
    }

    format!(
        "# How to submit: {}\n\n\
         The `inputs/` folder has {} file(s), from 001.in to {:03}.in.\n\
         They are generated for your team; the opposing team gets different ones.\n\n\
         Run your program once for each input, producing an output file with the\n\
         same name and the `.out` extension:\n\n\
         ```bash\n\
         mkdir -p outputs\n\
         for f in inputs/*.in; do\n  \
         python3 solution.py < \"$f\" > \"outputs/$(basename \"${{f%.in}}\").out\"\n\
         done\n\
         ```\n\n\
         (There's a ready-made `run.sh` in this folder: `bash run.sh 'python3 solution.py'`.)\n\n\
         Then, on the scoreboard, pick the question, attach the files from `outputs/` and submit.\n\
         You must send them all at once; the question only counts when every case passes.\n\n\
         ## How grading compares\n\n\
         - Trailing whitespace and a final newline don't count.\n\
         - {}\n\
         - Getting it wrong costs no points. You can try as many times as you like.\n",
        question.title,
        case_count,
        case_count,
        match question.mode {
            Mode::Exact => "Exact comparison: this question is all integers.",
            Mode::Json => "The output is read as JSON: key order and formatting don't count.",
            _ => "Numbers match with a tolerance of 0.01; it's not a text comparison.",
        }
    )
}

const RUNNER: &str = "#!/usr/bin/env bash\n\
# Usage: bash run.sh 'python3 solution.py'\n\
set -euo pipefail\n\
if [ $# -lt 1 ]; then echo \"usage: bash run.sh '<your program command>'\"; exit 1; fi\n\
mkdir -p outputs\n\
for input in inputs/*.in; do\n\
  name=$(basename \"${input%.in}\")\n\
  eval \"$1\" < \"$input\" > \"outputs/$name.out\"\n\
  echo \"wrote outputs/$name.out\"\n\
done\n";

#[derive(Deserialize)]
struct PackQuery {
    #[serde(default)]
    team: Option<String>,
}

async fn question_pack(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<PackQuery>,
) -> Result<Response, ApiError> {
    let user = caller(&ctx, &headers)?;
    let team_id = resolve_team(&user, query.team)?;
    let state = load(&ctx)?;
    guard_unlocked(&ctx, &state, &user, &team_id, &id)?;
    let question = find_question(&ctx, &id)?;
    let cases = ctx.cases_for(question, &team_id).map_err(oops)?;

    let mut zip = ZipBuilder::new();
    zip.add("statement.md", question.statement.clone());
    zip.add("HOW_TO_SUBMIT.md", how_to_send(question, cases.len()));
    if !cases.is_empty() {
        zip.add("run.sh", RUNNER);
    }
    for case in cases.iter() {
        zip.add(format!("inputs/{}.in", case.name), case.input.clone());
    }
    for extra in &question.extras {
        zip.add(extra.path.clone(), extra.body.clone());
    }

    let filename = format!("{}-{}.zip", question.id, slug(&question.title));
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/zip".to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        zip.finish(),
    )
        .into_response())
}

fn slug(title: &str) -> String {
    title
        .chars()
        .map(|letter| match letter {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            other if other.is_ascii_alphanumeric() => other.to_ascii_lowercase(),
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

async fn board(AxumState(ctx): AxumState<Shared>) -> Result<Json<Value>, ApiError> {
    let state = load(&ctx)?;

    let teams: Vec<Value> = state
        .teams
        .iter()
        .enumerate()
        .map(|(index, team)| {
            let members: Vec<Value> = state
                .members_of(&team.id)
                .iter()
                .map(|m| json!({ "id": m.id, "name": m.name, "photo": m.photo }))
                .collect();

            let solves: Value = ctx
                .questions
                .iter()
                .filter_map(|question| {
                    state
                        .solve(&team.id, &question.id)
                        .map(|s| (question.id.clone(), json!({ "at": s.at, "points": s.points })))
                })
                .collect::<serde_json::Map<_, _>>()
                .into();

            let attempts: Value = ctx
                .questions
                .iter()
                .filter_map(|question| {
                    let count = state.attempts_for(&team.id, &question.id);
                    (count > 0).then(|| (question.id.clone(), json!(count)))
                })
                .collect::<serde_json::Map<_, _>>()
                .into();

            json!({
                "id": team.id,
                "name": team.name,
                "color": team.color,
                "icon": team.icon,
                "side": if index == 0 { "left" } else { "right" },
                "members": members,
                "solves": solves,
                "attempts": attempts,
                "score": state.team_score(&team.id),
            })
        })
        .collect();

    Ok(Json(json!({
        "teams": teams,
        "locked": state.locked,
        "title": state.title.clone().unwrap_or_else(default_title),
    })))
}

#[derive(Deserialize)]
struct AuthRequest {
    token: String,
}

async fn auth(
    AxumState(ctx): AxumState<Shared>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<Value>, ApiError> {
    let state = load(&ctx)?;
    let user = state
        .user_by_token(payload.token.trim())
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "esse hash de login não existe"))?;
    let team = user
        .team_id
        .as_ref()
        .and_then(|id| state.teams.iter().find(|t| &t.id == id));
    Ok(Json(user_view(user, team, load_effects(&ctx, &user.id)?)))
}

#[derive(Deserialize)]
struct SpectateRequest {
    name: String,
    photo: Option<String>,
}

const IDLE_SEAT_MS: i64 = 180_000;

fn same_person(left: &str, right: &str) -> bool {
    let tidy = |value: &str| {
        value
            .trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    tidy(left) == tidy(right)
}

fn release_idle_seats(ctx: &Shared) -> Result<(), ApiError> {
    let cutoff = now_millis() - IDLE_SEAT_MS;
    ctx.store
        .with_connection(move |tx| {
            tx.execute("DELETE FROM spectators WHERE last_seen < ?1", params![cutoff])
        })
        .map_err(oops)?;
    Ok(())
}

async fn heartbeat(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let Viewer::Watcher(spectator) = viewer(&ctx, &headers)? else {
        return Ok(Json(json!({ "ok": true })));
    };
    ctx.store
        .with_connection(move |tx| {
            tx.execute(
                "UPDATE spectators SET last_seen = ?1 WHERE id = ?2",
                params![now_millis(), spectator.id],
            )
        })
        .map_err(oops)?;
    Ok(Json(json!({ "ok": true })))
}

async fn spectate(
    AxumState(ctx): AxumState<Shared>,
    Json(payload): Json<SpectateRequest>,
) -> Result<Json<Value>, ApiError> {
    let name = payload.name.trim().chars().take(22).collect::<String>();
    if name.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "escreva o seu nome"));
    }
    let photo = payload.photo.filter(|value| !value.is_empty());
    validate_photo(&Some(photo.clone()))?;

    release_idle_seats(&ctx)?;

    let spectator = ctx
        .store
        .with_connection(move |tx| {
            let mut taken = Vec::new();
            let mut known: Option<(String, String, i64, Option<String>)> = None;
            {
                let mut stmt = tx.prepare("SELECT id, token, name, photo, seat FROM spectators")?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let seat: i64 = row.get(4)?;
                    taken.push(seat);
                    let stored: String = row.get(2)?;
                    if known.is_none() && same_person(&stored, &name) {
                        known = Some((row.get(0)?, row.get(1)?, seat, row.get(3)?));
                    }
                }
            }

            if let Some((id, token, seat, stored_photo)) = known {
                let photo = photo.clone().or(stored_photo);
                tx.execute(
                    "UPDATE spectators SET name = ?1, photo = ?2, last_seen = ?3 WHERE id = ?4",
                    params![name, photo, now_millis(), id],
                )?;
                return Ok(Spectator { id, token, name, photo, seat });
            }

            taken.sort_unstable();
            let seat = taken
                .iter()
                .enumerate()
                .find(|(index, used)| **used != *index as i64)
                .map(|(index, _)| index as i64)
                .unwrap_or(taken.len() as i64);

            let id = random_id("spec", 8);
            let token = random_id("watch", 12);
            tx.execute(
                "INSERT INTO spectators (id, token, name, photo, seat, created_at, last_seen) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, token, name, photo, seat, now_millis()],
            )?;
            Ok(Spectator { id, token, name, photo, seat })
        })
        .map_err(oops)?;

    let effects = load_effects(&ctx, &spectator.id)?;
    Ok(Json(spectator_view(&spectator, effects)))
}

async fn me(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    if let Viewer::Watcher(spectator) = viewer(&ctx, &headers)? {
        let effects = load_effects(&ctx, &spectator.id)?;
        return Ok(Json(spectator_view(&spectator, effects)));
    }
    let user = caller(&ctx, &headers)?;
    let effects = load_effects(&ctx, &user.id)?;
    let state = load(&ctx)?;
    let team = user
        .team_id
        .as_ref()
        .and_then(|id| state.teams.iter().find(|t| &t.id == id));
    Ok(Json(user_view(&user, team, effects)))
}

#[derive(Deserialize)]
struct ProfileRequest {
    name: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    photo: PhotoPatch,
}

async fn update_me(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<ProfileRequest>,
) -> Result<Json<Value>, ApiError> {
    let mut user = caller(&ctx, &headers)?;
    validate_photo(&payload.photo)?;

    if let Some(name) = payload.name {
        if name.trim().is_empty() {
            return Err(err(StatusCode::BAD_REQUEST, "o nome não pode ficar vazio"));
        }
        user.name = name.trim().to_string();
    }
    apply_photo(&mut user.photo, payload.photo);

    ctx.store
        .with_connection(|tx| {
            tx.execute(
                "UPDATE users SET name = ?1, photo = ?2, profile_set = 1 WHERE id = ?3",
                params![user.name, user.photo, user.id],
            )
        })
        .map_err(oops)?;

    me(AxumState(ctx), headers).await
}

#[derive(Deserialize)]
struct SubmissionRequest {
    question_id: String,
    #[serde(default)]
    outputs: HashMap<String, String>,
    #[serde(default)]
    api_url: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
}

fn case_key(raw: &str) -> String {
    let name = raw.rsplit('/').next().unwrap_or(raw);
    name.trim()
        .trim_end_matches(".out")
        .trim_end_matches(".txt")
        .trim()
        .to_string()
}

fn judge_files(
    question: &Question,
    cases: &[crate::questions::Case],
    outputs: &HashMap<String, String>,
) -> (bool, Vec<Value>) {
    let mut submitted: HashMap<String, Vec<&String>> = HashMap::new();
    for (name, body) in outputs {
        submitted.entry(case_key(name)).or_default().push(body);
    }

    let mut report = Vec::new();
    let mut all_passed = true;

    for case in cases {
        let bodies = submitted.get(&case.name);
        let ambiguous = bodies
            .map(|list| list.iter().any(|body| *body != list[0]))
            .unwrap_or(false);
        if ambiguous {
            all_passed = false;
            report.push(json!({
                "name": case.name,
                "ok": false,
                "public": case.public,
                "detail": "você anexou mais de um arquivo diferente para este caso",
            }));
            continue;
        }

        let entry = match bodies.map(|list| list[0]) {
            None => {
                all_passed = false;
                json!({
                    "name": case.name,
                    "ok": false,
                    "public": case.public,
                    "detail": "faltou o arquivo desta saída",
                })
            }
            Some(body) => match judge::check(question.mode, &case.expected, body) {
                Ok(()) => json!({ "name": case.name, "ok": true, "public": case.public }),
                Err(mismatch) => {
                    all_passed = false;
                    let detail = if case.public {
                        match mismatch.line {
                            Some(line) => format!("linha {line}: {}", mismatch.detail),
                            None => mismatch.detail.clone(),
                        }
                    } else {
                        match mismatch.line {
                            Some(line) => format!("resposta incorreta (linha {line})"),
                            None => "resposta incorreta".to_string(),
                        }
                    };
                    json!({
                        "name": case.name,
                        "ok": false,
                        "public": case.public,
                        "detail": detail,
                    })
                }
            },
        };
        report.push(entry);
    }

    (all_passed, report)
}

async fn submit(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<SubmissionRequest>,
) -> Result<Json<Value>, ApiError> {
    let user = caller(&ctx, &headers)?;

    let team_id = resolve_team(&user, payload.team_id.clone())?;
    let question = find_question(&ctx, &payload.question_id)?;

    let state = load(&ctx)?;
    guard_unlocked(&ctx, &state, &user, &team_id, &question.id)?;
    if state.locked {
        return Err(err(StatusCode::FORBIDDEN, "os envios estão travados"));
    }
    if !state.teams.iter().any(|t| t.id == team_id) {
        return Err(err(StatusCode::NOT_FOUND, "time desconhecido"));
    }
    if state.solve(&team_id, &question.id).is_some() {
        return Ok(Json(json!({
            "correct": true,
            "note": "esta questão já está resolvida",
            "attempts": state.attempts_for(&team_id, &question.id),
        })));
    }

    let (correct, report) = if question.mode == Mode::Api {
        let url = payload
            .api_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                err(
                    StatusCode::BAD_REQUEST,
                    "informe o endereço da sua API, por exemplo http://127.0.0.1:8000",
                )
            })?;
        let (checks, passed) = apicheck::run(url)
            .await
            .map_err(|problem| err(StatusCode::BAD_REQUEST, &problem))?;
        let total = checks.len();
        let report: Vec<Value> = checks
            .into_iter()
            .map(|check| {
                json!({
                    "name": check.name,
                    "ok": check.ok,
                    "public": true,
                    "detail": check.detail,
                })
            })
            .collect();
        (passed == total, report)
    } else {
        if payload.outputs.is_empty() {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "anexe os arquivos de saída antes de enviar",
            ));
        }
        let cases = ctx.cases_for(question, &team_id).map_err(oops)?;
        judge_files(question, &cases, &payload.outputs)
    };

    let passed = report
        .iter()
        .filter(|case| case["ok"].as_bool().unwrap_or(false))
        .count();
    let total = report.len();

    let points = question.points;
    let question_id = question.id.clone();
    let by = user.id.clone();
    let team = team_id.clone();

    let attempts = ctx
        .store
        .with_connection(move |tx| {
            tx.execute(
                "INSERT INTO attempts (team_id, challenge_id, count) VALUES (?1, ?2, 1)
                 ON CONFLICT(team_id, challenge_id) DO UPDATE SET count = count + 1",
                params![team, question_id],
            )?;
            if correct {
                tx.execute(
                    "INSERT OR IGNORE INTO solves (team_id, challenge_id, at, points, by)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![team, question_id, now_millis(), points, by],
                )?;
            }
            tx.query_row(
                "SELECT count FROM attempts WHERE team_id = ?1 AND challenge_id = ?2",
                params![team, question_id],
                |row| row.get::<_, i64>(0),
            )
        })
        .map_err(oops)?;

    if !correct {
        record_event(&ctx, &Viewer::Member(user), "miss", team_id.clone())?;
    }

    Ok(Json(json!({
        "correct": correct,
        "attempts": attempts,
        "team_id": team_id,
        "passed": passed,
        "total": total,
        "cases": report,
    })))
}

// ---------------------------------------------------------------- admin

async fn admin_state(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    let state = load(&ctx)?;

    let users: Vec<Value> = state
        .users
        .iter()
        .map(|u| {
            json!({
                "id": u.id, "name": u.name, "photo": u.photo, "role": u.role,
                "team_id": u.team_id, "profile_set": u.profile_set, "token": u.token,
            })
        })
        .collect();

    let teams: Vec<Value> = state
        .teams
        .iter()
        .enumerate()
        .map(|(index, t)| {
            json!({
                "id": t.id, "name": t.name, "color": t.color, "icon": t.icon,
                "side": if index == 0 { "left" } else { "right" },
                "score": state.team_score(&t.id),
                "member_count": state.members_of(&t.id).len(),
            })
        })
        .collect();

    Ok(Json(json!({
        "users": users,
        "teams": teams,
        "locked": state.locked,
        "title": state.title.clone().unwrap_or_else(default_title),
        "questions": ctx.questions.iter().map(question_summary).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
    #[serde(default)]
    team_id: Option<String>,
}

async fn create_user(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<NewUser>,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "o nome não pode ficar vazio"));
    }

    let id = random_id("usr_", 8);
    let token = random_id("", 12);
    let stored = (id.clone(), token.clone(), name.clone());

    ctx.store
        .with_connection(move |tx| {
            let position: i64 = tx
                .query_row("SELECT COALESCE(MAX(position), 0) + 1 FROM users", [], |r| r.get(0))
                .unwrap_or(0);
            tx.execute(
                "INSERT INTO users (id, token, name, photo, role, team_id, profile_set, position)
                 VALUES (?1, ?2, ?3, NULL, 'participant', ?4, 0, ?5)",
                params![stored.0, stored.1, stored.2, payload.team_id, position],
            )
        })
        .map_err(oops)?;

    Ok(Json(json!({ "id": id, "name": name, "token": token })))
}

#[derive(Deserialize)]
struct PatchUser {
    name: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    team_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    photo: PhotoPatch,
}

async fn patch_user(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<PatchUser>,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    validate_photo(&payload.photo)?;

    let mut user = load(&ctx)?
        .users
        .into_iter()
        .find(|u| u.id == id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "pessoa desconhecida"))?;

    if let Some(name) = payload.name {
        if !name.trim().is_empty() {
            user.name = name.trim().to_string();
        }
    }
    if let Some(team_id) = payload.team_id {
        user.team_id = team_id;
    }
    apply_photo(&mut user.photo, payload.photo);

    ctx.store
        .with_connection(move |tx| {
            tx.execute(
                "UPDATE users SET name = ?1, photo = ?2, team_id = ?3 WHERE id = ?4",
                params![user.name, user.photo, user.team_id, user.id],
            )
        })
        .map_err(oops)?;

    Ok(Json(json!({ "ok": true })))
}

async fn delete_user(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let me = admin(&ctx, &headers)?;
    if me.id == id {
        return Err(err(StatusCode::BAD_REQUEST, "você não pode se apagar"));
    }
    ctx.store
        .with_connection(move |tx| tx.execute("DELETE FROM users WHERE id = ?1", params![id]))
        .map_err(oops)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct NewTeam {
    name: String,
    #[serde(default)]
    color: Option<String>,
}

async fn create_team(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<NewTeam>,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "o nome não pode ficar vazio"));
    }

    let existing = load(&ctx)?.teams.len();
    if existing >= 2 {
        return Err(err(
            StatusCode::CONFLICT,
            "o ringue só cabe 2 times, apague um antes",
        ));
    }

    let id = random_id("team_", 6);
    let color = payload
        .color
        .unwrap_or_else(|| if existing == 0 { "#3b82f6".into() } else { "#ef4444".into() });
    let stored = (id.clone(), name.clone(), color.clone());

    ctx.store
        .with_connection(move |tx| {
            tx.execute(
                "INSERT INTO teams (id, name, color, icon, position) VALUES (?1, ?2, ?3, NULL, ?4)",
                params![stored.0, stored.1, stored.2, existing as i64],
            )
        })
        .map_err(oops)?;

    Ok(Json(json!({ "id": id, "name": name, "color": color })))
}

#[derive(Deserialize)]
struct PatchTeam {
    name: Option<String>,
    color: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    icon: PhotoPatch,
}

async fn patch_team(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<PatchTeam>,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    validate_photo(&payload.icon)?;

    let mut team = load(&ctx)?
        .teams
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "time desconhecido"))?;

    if let Some(name) = payload.name {
        if !name.trim().is_empty() {
            team.name = name.trim().to_string();
        }
    }
    if let Some(color) = payload.color {
        team.color = color;
    }
    apply_photo(&mut team.icon, payload.icon);

    ctx.store
        .with_connection(move |tx| {
            tx.execute(
                "UPDATE teams SET name = ?1, color = ?2, icon = ?3 WHERE id = ?4",
                params![team.name, team.color, team.icon, team.id],
            )
        })
        .map_err(oops)?;

    Ok(Json(json!({ "ok": true })))
}

async fn delete_team(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    ctx.store
        .with_connection(move |tx| {
            tx.execute("DELETE FROM solves WHERE team_id = ?1", params![id])?;
            tx.execute("DELETE FROM attempts WHERE team_id = ?1", params![id])?;
            tx.execute("UPDATE users SET team_id = NULL WHERE team_id = ?1", params![id])?;
            tx.execute("DELETE FROM teams WHERE id = ?1", params![id])
        })
        .map_err(oops)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct LockRequest {
    locked: bool,
}

async fn set_lock(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<LockRequest>,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    let value = if payload.locked { "1" } else { "0" };
    ctx.store
        .with_connection(move |tx| {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('locked', ?1)",
                params![value],
            )
        })
        .map_err(oops)?;
    Ok(Json(json!({ "locked": payload.locked })))
}

fn default_title() -> String {
    "Code Fighter".to_string()
}

#[derive(Deserialize)]
struct TitleRequest {
    title: String,
}

async fn set_title(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<TitleRequest>,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    let trimmed = payload.title.trim();
    if trimmed.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "title cannot be empty"));
    }
    let title = trimmed.to_string();
    let stored = title.clone();
    ctx.store
        .with_connection(move |tx| {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('title', ?1)",
                params![stored],
            )
        })
        .map_err(oops)?;
    Ok(Json(json!({ "title": title })))
}

async fn export_state(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    let state = load(&ctx)?;
    serde_json::to_value(&state).map(Json).map_err(oops)
}

async fn reset_match(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    ctx.store
        .with_connection(|tx| {
            tx.execute("DELETE FROM solves", [])?;
            tx.execute("DELETE FROM attempts", [])
        })
        .map_err(oops)?;
    ctx.clear_generated();
    Ok(Json(json!({ "ok": true })))
}

const SUBMISSION_LIMIT: usize = 16 * 1024 * 1024;
const CHAT_LIMIT: usize = 60;
const EMOTES: [&str; 3] = ["wave", "laugh", "dance"];

#[derive(Deserialize)]
struct SayRequest {
    body: String,
}

fn record_event(
    ctx: &Shared,
    who: &Viewer,
    kind: &str,
    body: String,
) -> Result<i64, ApiError> {
    let author_id = who.id();
    let author_name = who.name();
    let author_kind = who.kind().to_string();
    let kind = kind.to_string();
    ctx.store
        .with_connection(move |tx| {
            tx.execute(
                "INSERT INTO events (kind, author_id, author_name, author_kind, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![kind, author_id, author_name, author_kind, body, now_millis()],
            )?;
            Ok(tx.last_insert_rowid())
        })
        .map_err(oops)
}

async fn say(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<SayRequest>,
) -> Result<Json<Value>, ApiError> {
    let who = viewer(&ctx, &headers)?;
    if who.is_admin() {
        return Err(err(StatusCode::FORBIDDEN, "o organizador não usa o chat"));
    }
    let body = payload.body.trim().chars().take(160).collect::<String>();
    if body.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "escreva alguma coisa"));
    }
    let id = record_event(&ctx, &who, "chat", body)?;
    Ok(Json(json!({ "id": id })))
}

async fn emote(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
    Json(payload): Json<SayRequest>,
) -> Result<Json<Value>, ApiError> {
    let who = viewer(&ctx, &headers)?;
    if who.is_admin() {
        return Err(err(StatusCode::FORBIDDEN, "o organizador não usa emotes"));
    }
    let body = normalize(&payload.body);
    if !EMOTES.contains(&body.as_str()) {
        return Err(err(StatusCode::BAD_REQUEST, "emote desconhecido"));
    }
    let id = record_event(&ctx, &who, "emote", body)?;
    Ok(Json(json!({ "id": id })))
}

async fn summon_ghost(
    AxumState(ctx): AxumState<Shared>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    admin(&ctx, &headers)?;
    let who = viewer(&ctx, &headers)?;
    let id = record_event(&ctx, &who, "ghost", "summon".to_string())?;
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
struct LiveQuery {
    #[serde(default)]
    since: i64,
}

async fn live(
    AxumState(ctx): AxumState<Shared>,
    axum::extract::Query(query): axum::extract::Query<LiveQuery>,
) -> Result<Json<Value>, ApiError> {
    let since = query.since;
    let state = load(&ctx)?;
    let mut tint: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for user in &state.users {
        if let Some(team) = user
            .team_id
            .as_ref()
            .and_then(|id| state.teams.iter().find(|t| &t.id == id))
        {
            tint.insert(user.id.clone(), team.color.clone());
        }
    }

    release_idle_seats(&ctx)?;

    let payload = ctx
        .store
        .with_connection(move |tx| {
            let mut people = Vec::new();
            {
                let mut stmt = tx.prepare(
                    "SELECT id, name, photo, seat FROM spectators ORDER BY seat ASC",
                )?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    people.push(json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "photo": row.get::<_, Option<String>>(2)?,
                        "seat": row.get::<_, i64>(3)?,
                    }));
                }
            }

            let mut chat = Vec::new();
            let mut emotes = Vec::new();
            let mut ghosts = Vec::new();
            let mut misses = Vec::new();
            let mut cursor = since;
            {
                let mut stmt = tx.prepare(
                    "SELECT id, kind, author_id, author_name, author_kind, body, created_at FROM events WHERE id > ?1 ORDER BY id ASC LIMIT 200",
                )?;
                let mut rows = stmt.query(params![since])?;
                while let Some(row) = rows.next()? {
                    let id: i64 = row.get(0)?;
                    let kind: String = row.get(1)?;
                    let author_id: String = row.get(2)?;
                    let entry = json!({
                        "id": id,
                        "authorId": author_id.clone(),
                        "author": row.get::<_, String>(3)?,
                        "authorKind": row.get::<_, String>(4)?,
                        "color": tint.get(&author_id),
                        "body": row.get::<_, String>(5)?,
                        "at": row.get::<_, i64>(6)?,
                    });
                    cursor = cursor.max(id);
                    match kind.as_str() {
                        "emote" => emotes.push(entry),
                        "ghost" => ghosts.push(entry),
                        "miss" => misses.push(entry),
                        _ => chat.push(entry),
                    }
                }
            }

            let mut history = Vec::new();
            if since == 0 {
                let mut stmt = tx.prepare(
                    "SELECT id, author_id, author_name, author_kind, body, created_at FROM (SELECT * FROM events WHERE kind = 'chat' ORDER BY id DESC LIMIT ?1) ORDER BY id ASC",
                )?;
                let mut rows = stmt.query(params![CHAT_LIMIT as i64])?;
                while let Some(row) = rows.next()? {
                    let author_id: String = row.get(1)?;
                    history.push(json!({
                        "id": row.get::<_, i64>(0)?,
                        "authorId": author_id.clone(),
                        "author": row.get::<_, String>(2)?,
                        "authorKind": row.get::<_, String>(3)?,
                        "color": tint.get(&author_id),
                        "body": row.get::<_, String>(4)?,
                        "at": row.get::<_, i64>(5)?,
                    }));
                }
            }

            Ok(json!({
                "now": now_millis(),
                "cursor": cursor,
                "spectators": people,
                "chat": if since == 0 { history } else { chat },
                "emotes": if since == 0 { Vec::new() } else { emotes },
                "ghosts": if since == 0 { Vec::new() } else { ghosts },
                "misses": if since == 0 { Vec::new() } else { misses },
            }))
        })
        .map_err(oops)?;

    Ok(Json(payload))
}

fn cors_layer() -> CorsLayer {
    let configured = std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:5173,http://127.0.0.1:5173".to_string()
    });

    if configured.trim() == "*" {
        return CorsLayer::very_permissive();
    }

    let origins: Vec<HeaderValue> = configured
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
        .expose_headers([axum::http::header::CONTENT_DISPOSITION])
}

pub fn router(ctx: Shared) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/questions", get(list_questions))
        .route("/v1/questions/{id}", get(question_detail))
        .route("/v1/questions/{id}/pack", get(question_pack))
        .route("/v1/board", get(board))
        .route("/v1/auth", post(auth))
        .route("/v1/spectate", post(spectate))
        .route("/v1/live", get(live))
        .route("/v1/chat", post(say))
        .route("/v1/emotes", post(emote))
        .route("/v1/ghost", post(summon_ghost))
        .route("/v1/me", get(me).patch(update_me))
        .route("/v1/me/effects", put(update_effects))
        .route("/v1/heartbeat", post(heartbeat))
        .route(
            "/v1/submissions",
            post(submit).layer(axum::extract::DefaultBodyLimit::max(SUBMISSION_LIMIT)),
        )
        .route("/v1/admin/state", get(admin_state))
        .route("/v1/admin/users", post(create_user))
        .route("/v1/admin/users/{id}", patch(patch_user).delete(delete_user))
        .route("/v1/admin/teams", post(create_team))
        .route("/v1/admin/teams/{id}", patch(patch_team).delete(delete_team))
        .route("/v1/admin/lock", post(set_lock))
        .route("/v1/admin/title", post(set_title))
        .route("/v1/admin/reset", post(reset_match))
        .route("/v1/admin/export", get(export_state))
        .layer(cors_layer())
        .with_state(ctx)
}
