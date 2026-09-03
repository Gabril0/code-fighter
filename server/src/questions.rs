use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::generation;

pub const PUBLIC_CASES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Numeric,
    Exact,
    Json,
    Api,
}

#[derive(Debug, Deserialize)]
struct Entry {
    id: String,
    dir: String,
    title: String,
    difficulty: String,
    points: i64,
    mode: Mode,
    #[serde(default)]
    extras: Vec<String>,
    /// How many extra per-team cases to generate (0 = static only).
    #[serde(default)]
    generated_cases: usize,
    /// Command that prints one test input to stdout given a seed argument.
    #[serde(default)]
    generator: Option<Vec<String>>,
    /// Command that reads an input on stdin and prints the expected output.
    #[serde(default)]
    solver: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Case {
    pub name: String,
    pub input: String,
    pub expected: String,
    pub public: bool,
}

#[derive(Debug, Clone)]
pub struct PackFile {
    pub path: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Question {
    pub id: String,
    pub title: String,
    pub difficulty: String,
    pub points: i64,
    pub mode: Mode,
    pub statement: String,
    pub cases: Vec<Case>,
    pub extras: Vec<PackFile>,
    /// Absolute path to the question folder (used to run generator/solver).
    pub dir: PathBuf,
    pub generated_cases: usize,
    pub generator: Option<Vec<String>>,
    pub solver: Option<Vec<String>>,
}

/// Does this question ship its own generator + solver to build per-team cases?
pub fn supports_generation(question: &Question) -> bool {
    question.generated_cases > 0
        && question.generator.is_some()
        && question.solver.is_some()
}

fn read_text(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))
}

/// Accept either the English name or the legacy Portuguese one, so existing
/// question folders keep working while new ones use the English convention.
fn first_existing(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names.iter().map(|name| dir.join(name)).find(|path| path.exists())
}

fn load_cases(dir: &Path) -> anyhow::Result<Vec<Case>> {
    let tests = match first_existing(dir, &["tests", "testes"]) {
        Some(path) if path.is_dir() => path,
        _ => return Ok(Vec::new()),
    };

    let mut names: Vec<String> = std::fs::read_dir(&tests)
        .with_context(|| format!("could not list {}", tests.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "in" {
                return None;
            }
            Some(path.file_stem()?.to_str()?.to_string())
        })
        .collect();
    names.sort();

    let mut cases = Vec::new();
    for (index, name) in names.into_iter().enumerate() {
        let input = read_text(&tests.join(format!("{name}.in")))?;
        let expected = read_text(&tests.join(format!("{name}.out")))?;
        cases.push(Case {
            name,
            input,
            expected,
            public: index < PUBLIC_CASES,
        });
    }
    Ok(cases)
}

fn load_extras(dir: &Path, allowed: &[String]) -> anyhow::Result<Vec<PackFile>> {
    let mut extras = Vec::new();

    for entry in allowed {
        anyhow::ensure!(
            !entry.contains("..") && !entry.starts_with('/'),
            "extra `{entry}` must stay inside the question folder"
        );
        let root = dir.join(entry);
        anyhow::ensure!(root.exists(), "missing extra {}", root.display());

        let mut walk = vec![(root, entry.clone())];
        while let Some((current, relative)) = walk.pop() {
            if current.is_dir() {
                let listing = std::fs::read_dir(&current)
                    .with_context(|| format!("could not list {}", current.display()))?;
                for child in listing.filter_map(|child| child.ok()) {
                    let name = child.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }
                    walk.push((child.path(), format!("{relative}/{name}")));
                }
                continue;
            }
            extras.push(PackFile {
                path: relative,
                body: std::fs::read(&current)
                    .with_context(|| format!("could not read {}", current.display()))?,
            });
        }
    }

    extras.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(extras)
}

pub fn total_cases(question: &Question) -> usize {
    if supports_generation(question) {
        return question.cases.len() + question.generated_cases;
    }
    question.cases.len()
}

pub fn case_names(question: &Question) -> Vec<String> {
    (1..=total_cases(question)).map(|index| format!("{index:03}")).collect()
}

/// The full ordered case list a team is judged against: the static cases first,
/// then any per-team cases produced by the question's own generator + solver.
pub fn cases_for(question: &Question, team_id: &str) -> anyhow::Result<Vec<Case>> {
    let mut cases: Vec<Case> = question
        .cases
        .iter()
        .enumerate()
        .map(|(index, case)| Case {
            name: format!("{:03}", index + 1),
            input: case.input.clone(),
            expected: case.expected.clone(),
            public: index < PUBLIC_CASES,
        })
        .collect();

    if !supports_generation(question) {
        return Ok(cases);
    }

    let generator = question.generator.as_ref().expect("checked by supports_generation");
    let solver = question.solver.as_ref().expect("checked by supports_generation");

    for index in 0..question.generated_cases {
        let seed = generation::case_seed(team_id, &question.id, index);
        let input = generation::run_generator(&question.dir, generator, seed)
            .with_context(|| format!("generating case {} for {}", index + 1, question.id))?;
        let expected = generation::run_solver(&question.dir, solver, &input)
            .with_context(|| format!("solving case {} for {}", index + 1, question.id))?;
        cases.push(Case {
            name: format!("{:03}", cases.len() + 1),
            input,
            expected,
            public: false,
        });
    }
    Ok(cases)
}

pub fn load(root: &PathBuf) -> anyhow::Result<Vec<Question>> {
    let manifest_path = root.join("manifest.json");
    let raw = read_text(&manifest_path)?;
    let entries: Vec<Entry> = serde_json::from_str(&raw)
        .with_context(|| format!("invalid JSON in {}", manifest_path.display()))?;
    anyhow::ensure!(!entries.is_empty(), "the question manifest is empty");

    let mut questions = Vec::new();
    for entry in entries {
        let dir = root.join(&entry.dir);
        anyhow::ensure!(dir.is_dir(), "missing question folder {}", dir.display());

        let cases = load_cases(&dir)?;
        if entry.mode == Mode::Api {
            anyhow::ensure!(
                cases.is_empty(),
                "question {} is checked live and must not ship test cases",
                entry.id
            );
        } else {
            anyhow::ensure!(!cases.is_empty(), "question {} has no test cases", entry.id);
        }

        if entry.generated_cases > 0 {
            anyhow::ensure!(
                entry.generator.is_some() && entry.solver.is_some(),
                "question {} sets generated_cases but is missing a generator or solver command",
                entry.id
            );
            anyhow::ensure!(
                entry.mode != Mode::Api,
                "question {} is checked live and cannot generate cases",
                entry.id
            );
        }

        let statement = first_existing(&dir, &["statement.md", "enunciado.md"])
            .with_context(|| format!("question {} has no statement.md", entry.id))?;

        questions.push(Question {
            id: entry.id,
            title: entry.title,
            difficulty: entry.difficulty,
            points: entry.points,
            mode: entry.mode,
            statement: read_text(&statement)?,
            cases,
            extras: load_extras(&dir, &entry.extras)?,
            dir,
            generated_cases: entry.generated_cases,
            generator: entry.generator,
            solver: entry.solver,
        });
    }
    Ok(questions)
}
