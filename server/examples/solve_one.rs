use std::io::Read;

use hackaton_checker::reference;

fn main() {
    let question = std::env::args().nth(1).unwrap_or_else(|| "q1".to_string());
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("input arrives on stdin");
    print!(
        "{}",
        reference::solve(&question, &input).expect("question has a solver")
    );
}
