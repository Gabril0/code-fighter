//! Reference generators and solvers for the shipped contest questions.
//!
//! This is the Rust replacement for the per-question Python scripts. The
//! generation engine (see `crate::generation`) runs it just like any other
//! program:
//!
//!   qtool <question-id> gen <seed>   -> prints one test input to stdout
//!   qtool <question-id> solve        -> reads an input on stdin,
//!                                        prints the expected output to stdout
//!
//! `gen` must be deterministic for a given seed; `solve` is the reference
//! answer and must match the statement's worked examples.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::exit;

/// A tiny deterministic PRNG (SplitMix64). Seeded from the engine's per-case
/// seed so a question regenerates identical cases after a restart.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n` (returns 0 when `n == 0`).
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    /// An inclusive range `[lo, hi]`.
    fn between(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            lo
        } else {
            lo + self.below((hi - lo + 1) as u64) as i64
        }
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

fn read_stdin() -> String {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .expect("reading stdin");
    buffer
}

fn write_stdout(text: &str) {
    let out = std::io::stdout();
    let mut lock = out.lock();
    lock.write_all(text.as_bytes()).expect("writing stdout");
}

// ---------------------------------------------------------------------------
// q1 — Maximum Subarray Sum (mode: numeric)
// ---------------------------------------------------------------------------

fn q1_generate(rng: &mut Rng) -> String {
    let count = rng.between(1, 200);
    let all_negative = rng.chance(15);
    let numbers: Vec<i64> = (0..count)
        .map(|_| {
            if all_negative {
                rng.between(-1000, -1)
            } else {
                rng.between(-1000, 1000)
            }
        })
        .collect();

    let body = numbers
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    format!("{count}\n{body}\n")
}

fn q1_solve(input: &str) -> String {
    let mut tokens = input.split_whitespace();
    let count: usize = tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let numbers: Vec<i64> = (0..count)
        .filter_map(|_| tokens.next().and_then(|v| v.parse().ok()))
        .collect();

    let mut best = numbers[0];
    let mut current = numbers[0];
    for &value in &numbers[1..] {
        current = value.max(current + value);
        best = best.max(current);
    }
    format!("{best}\n")
}

// ---------------------------------------------------------------------------
// q2 — FizzBuzz (mode: exact)
// ---------------------------------------------------------------------------

fn q2_generate(rng: &mut Rng) -> String {
    let count = if rng.chance(20) {
        rng.between(1, 100_000)
    } else {
        rng.between(1, 100)
    };
    format!("{count}\n")
}

fn q2_solve(input: &str) -> String {
    let count: i64 = input
        .split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut out = String::new();
    for value in 1..=count {
        if value % 15 == 0 {
            out.push_str("FizzBuzz");
        } else if value % 3 == 0 {
            out.push_str("Fizz");
        } else if value % 5 == 0 {
            out.push_str("Buzz");
        } else {
            out.push_str(&value.to_string());
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// q3 — Word Frequency Count (mode: json)
// ---------------------------------------------------------------------------

const VOCAB: &[&str] = &[
    "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog", "code",
    "fighter", "ring", "punch", "data", "json", "test", "array", "graph",
    "queue", "stack", "hash", "tree", "node", "edge", "alpha", "beta",
    "gamma", "delta", "42", "loop", "byte",
];
const SEPARATORS: &[&str] = &[" ", "  ", ", ", ". ", "! ", "\n", "; ", " - "];

fn q3_generate(rng: &mut Rng) -> String {
    let count = rng.between(3, 120);
    let mut text = String::new();
    for index in 0..count {
        let word = *rng.pick(VOCAB);
        if rng.chance(30) {
            text.push_str(&word.to_uppercase());
        } else {
            text.push_str(word);
        }
        if index < count - 1 {
            text.push_str(*rng.pick(SEPARATORS));
        }
    }
    text.push('\n');
    text
}

fn q3_solve(input: &str) -> String {
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut word = String::new();
    for ch in input.chars() {
        let lowered = ch.to_ascii_lowercase();
        if lowered.is_ascii_lowercase() || lowered.is_ascii_digit() {
            word.push(lowered);
        } else if !word.is_empty() {
            *counts.entry(std::mem::take(&mut word)).or_insert(0) += 1;
        }
    }
    if !word.is_empty() {
        *counts.entry(word).or_insert(0) += 1;
    }

    // Match the pretty two-space JSON the reference used, with sorted keys.
    if counts.is_empty() {
        return "{}\n".to_string();
    }
    let mut out = String::from("{\n");
    let last = counts.len() - 1;
    for (index, (key, value)) in counts.iter().enumerate() {
        let comma = if index == last { "" } else { "," };
        out.push_str(&format!("  \"{key}\": {value}{comma}\n"));
    }
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// SCAFFOLD:FUNCTIONS (new question generate/solve functions are inserted here)
// ---------------------------------------------------------------------------

fn usage() -> ! {
    eprintln!("usage: qtool <id> <gen|solve> [seed]");
    exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        usage();
    }
    let question = args[1].as_str();
    let mode = args[2].as_str();

    match mode {
        "gen" => {
            let seed: u64 = args
                .get(3)
                .and_then(|value| value.parse().ok())
                .or_else(|| {
                    std::env::var("CASE_SEED")
                        .ok()
                        .and_then(|value| value.parse().ok())
                })
                .unwrap_or(0);
            let mut rng = Rng::new(seed);
            let case = match question {
                "q1" => q1_generate(&mut rng),
                "q2" => q2_generate(&mut rng),
                "q3" => q3_generate(&mut rng),
                // SCAFFOLD:GEN_ARMS
                _ => usage(),
            };
            write_stdout(&case);
        }
        "solve" => {
            let input = read_stdin();
            let answer = match question {
                "q1" => q1_solve(&input),
                "q2" => q2_solve(&input),
                "q3" => q3_solve(&input),
                // SCAFFOLD:SOLVE_ARMS
                _ => usage(),
            };
            write_stdout(&answer);
        }
        _ => usage(),
    }
}
