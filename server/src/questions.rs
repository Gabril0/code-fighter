use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::reference;
use crate::rng::Rng;

pub const PUBLIC_CASES: usize = 2;
pub const GENERATED_CASES: usize = 100;

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
}

fn read_text(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))
}

fn load_cases(dir: &Path) -> anyhow::Result<Vec<Case>> {
    let tests = dir.join("testes");
    if !tests.is_dir() {
        return Ok(Vec::new());
    }

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
    if reference::supports(&question.id) {
        return question.cases.len() + GENERATED_CASES;
    }
    question.cases.len()
}

pub fn case_names(question: &Question) -> Vec<String> {
    (1..=total_cases(question)).map(|index| format!("{index:03}")).collect()
}

pub fn cases_for(question: &Question, team_id: &str) -> Vec<Case> {
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

    if !reference::supports(&question.id) {
        return cases;
    }

    let mut rng = Rng::seeded(&format!("{team_id}:{}", question.id));
    for _ in 0..GENERATED_CASES {
        let Some(input) = reference::generate(&question.id, &mut rng) else {
            break;
        };
        let Some(expected) = reference::solve(&question.id, &input) else {
            break;
        };
        cases.push(Case {
            name: format!("{:03}", cases.len() + 1),
            input,
            expected,
            public: false,
        });
    }
    cases
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

        questions.push(Question {
            id: entry.id,
            title: entry.title,
            difficulty: entry.difficulty,
            points: entry.points,
            mode: entry.mode,
            statement: read_text(&dir.join("enunciado.md"))?,
            cases,
            extras: load_extras(&dir, &entry.extras)?,
        });
    }
    Ok(questions)
}
