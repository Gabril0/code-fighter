use hackaton_checker::reference;
use hackaton_checker::rng::Rng;
use serde_json::json;

fn main() {
    let mut args = std::env::args().skip(1);
    let question = args.next().unwrap_or_else(|| "q1".to_string());
    let count: usize = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(500);
    let label = args.next().unwrap_or_else(|| "equivalence".to_string());

    let mut rng = Rng::seeded(&format!("{label}:{question}"));
    let mut cases = Vec::new();
    for index in 0..count {
        let input = reference::generate(&question, &mut rng).expect("question has a generator");
        let expected = reference::solve(&question, &input).expect("question has a solver");
        cases.push(json!({ "index": index, "input": input, "expected": expected }));
    }

    println!(
        "{}",
        serde_json::to_string(&json!({ "question": question, "cases": cases })).unwrap()
    );
}
