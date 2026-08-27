use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use anyhow::Context;
use rand::distributions::Alphanumeric;
use rand::Rng;
use rusqlite::{params, Connection};

use crate::model::{Role, Solve, State, Team, User};

pub fn random_id(prefix: &str, len: usize) -> String {
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    format!("{prefix}{}", suffix.to_lowercase())
}

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    photo TEXT,
    role TEXT NOT NULL,
    team_id TEXT,
    profile_set INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS teams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    icon TEXT,
    position INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS solves (
    team_id TEXT NOT NULL,
    challenge_id TEXT NOT NULL,
    at INTEGER NOT NULL,
    points INTEGER NOT NULL,
    by TEXT NOT NULL,
    PRIMARY KEY (team_id, challenge_id)
);
CREATE TABLE IF NOT EXISTS attempts (
    team_id TEXT NOT NULL,
    challenge_id TEXT NOT NULL,
    count INTEGER NOT NULL,
    PRIMARY KEY (team_id, challenge_id)
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS effects (
    owner_id TEXT PRIMARY KEY,
    pixelate INTEGER NOT NULL,
    crt INTEGER NOT NULL,
    aberration INTEGER NOT NULL,
    bloom INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS spectators (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    photo TEXT,
    seat INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    last_seen INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    author_id TEXT NOT NULL,
    author_name TEXT NOT NULL,
    author_kind TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
";

pub struct Store {
    connection: Mutex<Connection>,
}

impl Store {
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
        }

        let connection = Connection::open(&path)
            .with_context(|| format!("opening {}", path.display()))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.execute_batch(SCHEMA)?;
        drop_spectator_email(&connection)?;
        add_bloom_effect(&connection)?;
        add_spectator_last_seen(&connection)?;

        let store = Self {
            connection: Mutex::new(connection),
        };
        migrate_from_json(&store, &path)?;
        store.seed_from_env()?;
        store.ensure_admin()?;
        Ok(store)
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock().expect("sqlite mutex poisoned")
    }

    pub fn read(&self) -> anyhow::Result<State> {
        let connection = self.lock();

        let mut statement = connection.prepare(
            "SELECT id, token, name, photo, role, team_id, profile_set FROM users ORDER BY position, rowid",
        )?;
        let users = statement
            .query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    name: row.get(2)?,
                    photo: row.get(3)?,
                    role: if row.get::<_, String>(4)? == "admin" {
                        Role::Admin
                    } else {
                        Role::Participant
                    },
                    team_id: row.get(5)?,
                    profile_set: row.get::<_, i64>(6)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut statement = connection
            .prepare("SELECT id, name, color, icon FROM teams ORDER BY position, rowid")?;
        let teams = statement
            .query_map([], |row| {
                Ok(Team {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    icon: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut statement =
            connection.prepare("SELECT team_id, challenge_id, at, points, by FROM solves")?;
        let solves = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    Solve {
                        at: row.get(2)?,
                        points: row.get(3)?,
                        by: row.get(4)?,
                    },
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut statement =
            connection.prepare("SELECT team_id, challenge_id, count FROM attempts")?;
        let attempts = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? as u32)))?
            .collect::<Result<Vec<_>, _>>()?;

        let locked = connection
            .query_row("SELECT value FROM settings WHERE key = 'locked'", [], |row| {
                row.get::<_, String>(0)
            })
            .map(|value| value == "1")
            .unwrap_or(false);

        let title = connection
            .query_row("SELECT value FROM settings WHERE key = 'title'", [], |row| {
                row.get::<_, String>(0)
            })
            .ok()
            .filter(|value| !value.trim().is_empty());

        Ok(State {
            users,
            teams,
            spectators: Vec::new(),
            solves,
            attempts,
            locked,
            title,
        })
    }

    pub fn with_connection<T>(
        &self,
        apply: impl FnOnce(&rusqlite::Transaction<'_>) -> rusqlite::Result<T>,
    ) -> anyhow::Result<T> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;
        let result = apply(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }

    fn seed_from_env(&self) -> anyhow::Result<()> {
        let existing: i64 = self
            .lock()
            .query_row("SELECT COUNT(*) FROM teams", [], |row| row.get(0))?;
        if existing > 0 {
            return Ok(());
        }

        let (raw, origin) = match std::env::var("SEED_JSON") {
            Ok(inline) if !inline.trim().is_empty() => (inline, "SEED_JSON".to_string()),
            _ => {
                let Ok(path) = std::env::var("SEED_FILE") else {
                    return Ok(());
                };
                let path = PathBuf::from(path);
                if !path.exists() {
                    println!("seed file {} not found, starting empty", path.display());
                    return Ok(());
                }
                let raw = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                (raw, path.display().to_string())
            }
        };

        let seed: State = serde_json::from_str(&raw)
            .with_context(|| format!("invalid seed JSON from {origin}"))?;
        let people = seed.users.len();
        let teams = seed.teams.len();
        self.import(&seed)?;
        println!("seeded {teams} teams and {people} people from {origin}");
        Ok(())
    }

    fn import(&self, state: &State) -> anyhow::Result<()> {
        self.with_connection(|tx| {
            for (index, user) in state.users.iter().enumerate() {
                tx.execute(
                    "INSERT OR IGNORE INTO users (id, token, name, photo, role, team_id, profile_set, position)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        user.id,
                        user.token,
                        user.name,
                        user.photo,
                        if user.role == Role::Admin { "admin" } else { "participant" },
                        user.team_id,
                        user.profile_set as i64,
                        index as i64
                    ],
                )?;
            }
            for (index, team) in state.teams.iter().enumerate() {
                tx.execute(
                    "INSERT OR IGNORE INTO teams (id, name, color, icon, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![team.id, team.name, team.color, team.icon, index as i64],
                )?;
            }
            for spectator in &state.spectators {
                tx.execute(
                    "INSERT OR IGNORE INTO spectators (id, token, name, photo, seat, created_at, last_seen)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        spectator.id,
                        spectator.token,
                        spectator.name,
                        spectator.photo,
                        spectator.seat,
                        now_millis(),
                        i64::MAX
                    ],
                )?;
            }
            for (team_id, question_id, solve) in &state.solves {
                tx.execute(
                    "INSERT OR IGNORE INTO solves (team_id, challenge_id, at, points, by) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![team_id, question_id, solve.at, solve.points, solve.by],
                )?;
            }
            for (team_id, question_id, count) in &state.attempts {
                tx.execute(
                    "INSERT OR IGNORE INTO attempts (team_id, challenge_id, count) VALUES (?1, ?2, ?3)",
                    params![team_id, question_id, *count as i64],
                )?;
            }
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('locked', ?1)",
                params![if state.locked { "1" } else { "0" }],
            )?;
            if let Some(title) = state.title.as_ref().filter(|t| !t.trim().is_empty()) {
                tx.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES ('title', ?1)",
                    params![title],
                )?;
            }
            Ok(())
        })
    }

    fn ensure_admin(&self) -> anyhow::Result<()> {
        let wanted = std::env::var("ADMIN_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let existing: Option<String> = self
            .lock()
            .query_row(
                "SELECT token FROM users WHERE role = 'admin' ORDER BY rowid LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        let token = match (existing, wanted) {
            (Some(current), Some(wanted)) if current != wanted => {
                self.with_connection(|tx| {
                    tx.execute(
                        "UPDATE users SET token = ?1 WHERE role = 'admin'",
                        params![wanted],
                    )
                })?;
                wanted
            }
            (Some(current), _) => current,
            (None, wanted) => {
                let token = wanted.unwrap_or_else(|| random_id("admin-", 10));
                let id = random_id("usr_", 8);
                let stored = token.clone();
                self.with_connection(move |tx| {
                    tx.execute(
                        "INSERT INTO users (id, token, name, photo, role, team_id, profile_set, position)
                         VALUES (?1, ?2, 'Organiser', NULL, 'admin', NULL, 1, -1)",
                        params![id, stored],
                    )
                })?;
                token
            }
        };

        println!("\n  ┌──────────────────────────────────────────────┐");
        println!("  │  admin login hash                            │");
        println!("  │  {token:<43} │");
        println!("  └──────────────────────────────────────────────┘\n");
        Ok(())
    }
}

fn add_spectator_last_seen(connection: &Connection) -> anyhow::Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(spectators)")?;
    let has_column = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|column| column.ok())
        .any(|column| column == "last_seen");
    drop(statement);
    if has_column {
        return Ok(());
    }
    connection.execute_batch(
        "ALTER TABLE spectators ADD COLUMN last_seen INTEGER NOT NULL DEFAULT 0;
         UPDATE spectators SET last_seen = created_at;",
    )?;
    println!("spectators gained a last_seen column");
    Ok(())
}

fn add_bloom_effect(connection: &Connection) -> anyhow::Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(effects)")?;
    let has_bloom = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|column| column.ok())
        .any(|column| column == "bloom");
    drop(statement);
    if has_bloom {
        return Ok(());
    }
    connection.execute_batch("ALTER TABLE effects ADD COLUMN bloom INTEGER NOT NULL DEFAULT 1;")?;
    println!("effects gained a bloom column");
    Ok(())
}

fn drop_spectator_email(connection: &Connection) -> anyhow::Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(spectators)")?;
    let has_email = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|column| column.ok())
        .any(|column| column == "email");
    drop(statement);
    if !has_email {
        return Ok(());
    }

    connection.execute_batch(
        "BEGIN;
         CREATE TABLE spectators_rebuilt (
             id TEXT PRIMARY KEY,
             token TEXT NOT NULL UNIQUE,
             name TEXT NOT NULL,
             photo TEXT,
             seat INTEGER NOT NULL,
             created_at INTEGER NOT NULL
         );
         INSERT INTO spectators_rebuilt (id, token, name, photo, seat, created_at)
             SELECT id, token, name, photo, seat, created_at FROM spectators;
         DROP TABLE spectators;
         ALTER TABLE spectators_rebuilt RENAME TO spectators;
         COMMIT;",
    )?;
    println!("spectators no longer carry an email, column dropped");
    Ok(())
}

/// One-shot import of the pre-SQLite `state.json` so an event in progress is not lost.
fn migrate_from_json(store: &Store, db_path: &Path) -> anyhow::Result<()> {
    let json_path = db_path.with_file_name("state.json");
    if !json_path.exists() {
        return Ok(());
    }

    let already: i64 = store
        .lock()
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    if already > 0 {
        return Ok(());
    }

    let raw = std::fs::read_to_string(&json_path)?;
    let legacy: State = match serde_json::from_str(&raw) {
        Ok(state) => state,
        Err(_) => return Ok(()),
    };

    store.import(&legacy)?;

    let backup = json_path.with_extension("json.imported");
    std::fs::rename(&json_path, &backup)?;
    println!(
        "imported {} users and {} teams from the old state.json (kept as {})",
        legacy.users.len(),
        legacy.teams.len(),
        backup.display()
    );
    Ok(())
}
