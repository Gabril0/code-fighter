use serde_json::{json, Map, Value};

use crate::rng::Rng;

use super::money;

#[derive(Default)]
struct Ledger {
    order: Vec<String>,
    balance: Vec<f64>,
    paid_in: Vec<f64>,
    paid_out: Vec<f64>,
}

impl Ledger {
    fn slot(&mut self, investor: &str) -> usize {
        if let Some(at) = self.order.iter().position(|name| name == investor) {
            return at;
        }
        self.order.push(investor.to_string());
        self.balance.push(0.0);
        self.paid_in.push(0.0);
        self.paid_out.push(0.0);
        self.order.len() - 1
    }

    fn total(&self) -> f64 {
        let mut sum = 0.0;
        for value in &self.balance {
            sum += value;
        }
        sum
    }

    fn accrue(&mut self, factor: f64) {
        for value in &mut self.balance {
            *value *= factor;
        }
    }
}

pub fn solve(input: &str) -> String {
    let mut lines = input.trim().lines();
    let rate: f64 = lines.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0.0);

    let mut ledger = Ledger::default();
    let mut previous_day: Option<i64> = None;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() != 4 {
            continue;
        }
        let day: i64 = parts[0].parse().unwrap_or(0);

        if let Some(last) = previous_day {
            if day > last {
                ledger.accrue((1.0 + rate).powf((day - last) as f64));
            }
        }
        previous_day = Some(day);

        match parts[1] {
            "INTEGRALIZACAO" => {
                let amount: f64 = parts[3].parse().unwrap_or(0.0);
                let at = ledger.slot(parts[2]);
                ledger.balance[at] += amount;
                ledger.paid_in[at] += amount;
            }
            "AMORTIZACAO" => {
                let amount: f64 = parts[3].parse().unwrap_or(0.0);
                let total = ledger.total();
                if total > 0.0 {
                    for at in 0..ledger.order.len() {
                        let share = amount * ledger.balance[at] / total;
                        ledger.balance[at] -= share;
                        ledger.paid_out[at] += share;
                    }
                }
            }
            "AMORTIZACAO_TOTAL" => {
                for at in 0..ledger.order.len() {
                    ledger.paid_out[at] += ledger.balance[at];
                    ledger.balance[at] = 0.0;
                }
            }
            _ => {}
        }
    }

    let mut names: Vec<(usize, &String)> = ledger.order.iter().enumerate().collect();
    names.sort_by(|left, right| left.1.cmp(right.1));

    let mut answer = Map::new();
    for (at, name) in names {
        answer.insert(
            name.clone(),
            json!({
                "total_integralizado": money::round_two(ledger.paid_in[at]),
                "total_recebido": money::round_two(ledger.paid_out[at]),
                "resultado": money::round_two(ledger.paid_out[at] - ledger.paid_in[at]),
            }),
        );
    }

    format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(answer)).unwrap_or_default()
    )
}

const FUND_CEILING: f64 = 1e12;
const GROWTH_BUDGET: f64 = 1e3;

pub fn generate(rng: &mut Rng) -> String {
    let rate = if rng.chance(15) {
        0
    } else {
        rng.between(1, 10_000)
    };
    let rate_text = format!("{:.6}", rate as f64 / 1_000_000.0);
    let rate_value: f64 = rate_text.parse().unwrap_or(0.0);

    let investors = rng.between(2, 8) as usize;
    let names: Vec<String> = (0..investors).map(|index| format!("INV{:03}", index + 1)).collect();
    let events = if rng.chance(25) {
        rng.between(200, 600) as usize
    } else {
        rng.between(3, 40) as usize
    };

    let span = if rate_value > 0.0 {
        (GROWTH_BUDGET.ln() / (1.0 + rate_value).ln()) as i64
    } else {
        99_000
    };
    let gap = (span / (events as i64 + 1)).clamp(0, 40);
    let small_money = rng.chance(30);
    let entry_ceiling = if small_money {
        rng.between(50, 5_000) as f64
    } else {
        (FUND_CEILING / GROWTH_BUDGET / investors as f64).min(5e8)
    };

    let mut ledger = Ledger::default();
    let mut lines = vec![rate_text.clone()];
    let mut day: i64 = rng.between(1, 5);
    let mut previous_day: Option<i64> = None;
    let mut joined = 0_usize;

    for step in 0..events {
        if step > 0 {
            day += rng.between(0, gap);
        }
        if let Some(last) = previous_day {
            if day > last {
                ledger.accrue((1.0 + rate_value).powf((day - last) as f64));
            }
        }
        previous_day = Some(day);

        let total = ledger.total();
        let cents_available = (total * 100.0).floor();
        let can_amortise = joined > 0 && cents_available >= 1.0 && cents_available.is_finite();
        let room = (entry_ceiling - total).max(0.0);
        let bring_someone_in = joined == 0 || (joined < investors && rng.chance(45));

        let entry_floor = if small_money { 1.0 } else { 1_000.0 };
        if (bring_someone_in || !can_amortise) && room >= entry_floor {
            let investor = if bring_someone_in {
                joined += 1;
                names[joined - 1].clone()
            } else {
                rng.pick(&names[..joined]).clone()
            };
            let cents = rng.between(
                if small_money { 100 } else { 100_000 },
                (room * 100.0).min(5e10) as i64,
            );
            let amount = cents as f64 / 100.0;
            let at = ledger.slot(&investor);
            ledger.balance[at] += amount;
            ledger.paid_in[at] += amount;
            lines.push(format!("{day};INTEGRALIZACAO;{investor};{amount:.2}"));
            continue;
        }

        if !can_amortise {
            continue;
        }

        let amount = rng.between(1, cents_available as i64) as f64 / 100.0;
        let total_now = ledger.total();
        for at in 0..ledger.order.len() {
            let share = amount * ledger.balance[at] / total_now;
            ledger.balance[at] -= share;
            ledger.paid_out[at] += share;
        }
        lines.push(format!("{day};AMORTIZACAO;;{amount:.2}"));
    }

    day += rng.between(1, gap.max(1));
    lines.push(format!("{day};AMORTIZACAO_TOTAL;;"));
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(input: &str) -> Value {
        serde_json::from_str(&solve(input)).expect("solver emits json")
    }

    #[test]
    fn reproduces_the_first_statement_example() {
        let answer = parsed(
            "0.001000\n1;INTEGRALIZACAO;INV001;100000.00\n1;INTEGRALIZACAO;INV002;50000.00\n3;AMORTIZACAO;;30000.00\n5;AMORTIZACAO_TOTAL;;\n",
        );
        assert_eq!(answer["INV001"]["total_recebido"], json!(100360.58));
        assert_eq!(answer["INV001"]["resultado"], json!(360.58));
        assert_eq!(answer["INV002"]["total_recebido"], json!(50180.29));
        assert_eq!(answer["INV002"]["resultado"], json!(180.29));
    }

    #[test]
    fn reproduces_the_second_statement_example() {
        let answer = parsed(
            "0.001000\n1;INTEGRALIZACAO;INV001;100000.00\n3;AMORTIZACAO;;50000.00\n3;INTEGRALIZACAO;INV002;50000.00\n6;AMORTIZACAO_TOTAL;;\n",
        );
        assert_eq!(answer["INV001"]["total_recebido"], json!(100350.85));
        assert_eq!(answer["INV001"]["resultado"], json!(350.85));
        assert_eq!(answer["INV002"]["total_recebido"], json!(50150.15));
        assert_eq!(answer["INV002"]["resultado"], json!(150.15));
    }

    #[test]
    fn a_zero_rate_leaves_nobody_with_a_profit() {
        let answer = parsed(
            "0.000000\n1;INTEGRALIZACAO;INV001;1000.00\n9;INTEGRALIZACAO;INV002;500.00\n20;AMORTIZACAO_TOTAL;;\n",
        );
        assert_eq!(answer["INV001"]["resultado"], json!(0.0));
        assert_eq!(answer["INV002"]["resultado"], json!(0.0));
    }

    #[test]
    fn money_in_on_day_d_does_not_earn_on_day_d() {
        let answer = parsed(
            "0.010000\n1;INTEGRALIZACAO;INV001;1000.00\n1;AMORTIZACAO_TOTAL;;\n",
        );
        assert_eq!(answer["INV001"]["total_recebido"], json!(1000.0));
    }

    #[test]
    fn generated_cases_never_amortise_more_than_the_fund_holds() {
        let mut rng = Rng::seeded("q4-invariants");
        for _ in 0..400 {
            let case = generate(&mut rng);
            let mut lines = case.lines();
            let rate: f64 = lines.next().unwrap().parse().unwrap();
            assert!((0.0..=0.01).contains(&rate));

            let mut ledger = Ledger::default();
            let mut previous: Option<i64> = None;
            let mut days: Vec<i64> = Vec::new();
            for line in lines {
                let parts: Vec<&str> = line.split(';').collect();
                assert_eq!(parts.len(), 4, "malformed event {line}");
                let day: i64 = parts[0].parse().unwrap();
                days.push(day);
                if let Some(last) = previous {
                    if day > last {
                        ledger.accrue((1.0 + rate).powf((day - last) as f64));
                    }
                }
                previous = Some(day);
                match parts[1] {
                    "INTEGRALIZACAO" => {
                        let amount: f64 = parts[3].parse().unwrap();
                        assert!(amount >= 0.01);
                        let at = ledger.slot(parts[2]);
                        ledger.balance[at] += amount;
                    }
                    "AMORTIZACAO" => {
                        let amount: f64 = parts[3].parse().unwrap();
                        let total = ledger.total();
                        assert!(total < FUND_CEILING, "fund reached {total}");
                        assert!(amount >= 0.01);
                        assert!(
                            amount <= total,
                            "amortisation of {amount} exceeds the fund's {total}"
                        );
                        for at in 0..ledger.order.len() {
                            ledger.balance[at] -= amount * ledger.balance[at] / total;
                        }
                    }
                    "AMORTIZACAO_TOTAL" => {
                        assert!(parts[2].is_empty() && parts[3].is_empty());
                        for at in 0..ledger.order.len() {
                            ledger.balance[at] = 0.0;
                        }
                    }
                    other => panic!("unexpected event {other}"),
                }
            }
            assert!(days.windows(2).all(|pair| pair[1] >= pair[0]), "days went back");
            assert!(
                ledger.total() < FUND_CEILING && ledger.total().is_finite(),
                "fund reached {}",
                ledger.total()
            );
            assert!(case.trim().ends_with("AMORTIZACAO_TOTAL;;"));
            assert!(*days.last().unwrap() <= 100_000);
        }
    }

    #[test]
    fn some_generated_cases_deal_in_small_amounts() {
        let mut rng = Rng::seeded("q4-small");
        let mut small = 0;
        for _ in 0..300 {
            let case = generate(&mut rng);
            let biggest = case
                .lines()
                .skip(1)
                .filter(|line| line.contains("INTEGRALIZACAO"))
                .filter_map(|line| line.split(';').nth(3))
                .filter_map(|value| value.parse::<f64>().ok())
                .fold(0.0_f64, f64::max);
            if biggest > 0.0 && biggest < 10_000.0 {
                small += 1;
            }
        }
        assert!(small > 40, "only {small} of 300 cases used small amounts");
    }

    #[test]
    fn some_generated_cases_run_long_enough_to_compound() {
        let mut rng = Rng::seeded("q4-long");
        let mut long = 0;
        for _ in 0..300 {
            if generate(&mut rng).lines().count() > 200 {
                long += 1;
            }
        }
        assert!(long > 10, "only {long} of 300 cases were long");
    }

    #[test]
    fn every_investor_who_pays_in_shows_up_in_the_answer() {
        let mut rng = Rng::seeded("q4-coverage");
        for _ in 0..200 {
            let case = generate(&mut rng);
            let expected: std::collections::BTreeSet<String> = case
                .lines()
                .skip(1)
                .filter(|line| line.contains("INTEGRALIZACAO"))
                .map(|line| line.split(';').nth(2).unwrap().to_string())
                .collect();
            let answer = parsed(&case);
            let got: std::collections::BTreeSet<String> =
                answer.as_object().unwrap().keys().cloned().collect();
            assert_eq!(expected, got);
        }
    }
}
