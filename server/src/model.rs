use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Participant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub token: String,
    pub name: String,
    #[serde(default)]
    pub photo: Option<String>,
    pub role: Role,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub profile_set: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solve {
    pub at: i64,
    pub points: i64,
    pub by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedSpectator {
    pub id: String,
    pub token: String,
    pub name: String,
    #[serde(default)]
    pub photo: Option<String>,
    pub seat: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub users: Vec<User>,
    #[serde(default)]
    pub teams: Vec<Team>,
    #[serde(default)]
    pub spectators: Vec<SeedSpectator>,
    #[serde(default)]
    pub solves: Vec<(String, String, Solve)>,
    #[serde(default)]
    pub attempts: Vec<(String, String, u32)>,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub title: Option<String>,
}

impl State {
    pub fn user_by_token(&self, token: &str) -> Option<&User> {
        self.users.iter().find(|u| u.token == token)
    }

    pub fn solve(&self, team_id: &str, question_id: &str) -> Option<&Solve> {
        self.solves
            .iter()
            .find(|(t, c, _)| t == team_id && c == question_id)
            .map(|(_, _, solve)| solve)
    }

    pub fn attempts_for(&self, team_id: &str, question_id: &str) -> u32 {
        self.attempts
            .iter()
            .find(|(t, c, _)| t == team_id && c == question_id)
            .map(|(_, _, n)| *n)
            .unwrap_or(0)
    }

    pub fn members_of(&self, team_id: &str) -> Vec<&User> {
        self.users
            .iter()
            .filter(|u| u.team_id.as_deref() == Some(team_id) && u.role == Role::Participant)
            .collect()
    }

    pub fn team_score(&self, team_id: &str) -> i64 {
        self.solves
            .iter()
            .filter(|(t, _, _)| t == team_id)
            .map(|(_, _, solve)| solve.points)
            .sum()
    }
}
