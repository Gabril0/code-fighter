pub mod money;
pub mod q1;
pub mod q2;
pub mod q3;
pub mod q4;

use crate::rng::Rng;

pub fn solve(question_id: &str, input: &str) -> Option<String> {
    match question_id {
        "q1" => Some(q1::solve(input)),
        "q2" => Some(q2::solve(input)),
        "q3" => Some(q3::solve(input)),
        "q4" => Some(q4::solve(input)),
        _ => None,
    }
}

pub fn generate(question_id: &str, rng: &mut Rng) -> Option<String> {
    match question_id {
        "q1" => Some(q1::generate(rng)),
        "q2" => Some(q2::generate(rng)),
        "q3" => Some(q3::generate(rng)),
        "q4" => Some(q4::generate(rng)),
        _ => None,
    }
}

pub fn supports(question_id: &str) -> bool {
    matches!(question_id, "q1" | "q2" | "q3" | "q4")
}
