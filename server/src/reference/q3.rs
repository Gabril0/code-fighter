use crate::rng::Rng;

struct Transaction {
    id: String,
    stamp: String,
    kind: String,
    amount: i64,
}

fn priority(kind: &str) -> u8 {
    if kind == "DEBITO" {
        1
    } else {
        0
    }
}

pub fn solve(input: &str) -> String {
    let mut lines = input.trim().lines();
    let mut balance: i64 = lines.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0);

    let mut transactions: Vec<Transaction> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() != 4 {
            continue;
        }
        transactions.push(Transaction {
            id: parts[0].to_string(),
            stamp: parts[1].to_string(),
            kind: parts[2].to_string(),
            amount: parts[3].parse().unwrap_or(0),
        });
    }

    transactions.sort_by(|left, right| {
        let left_key = (
            u8::from(left.kind == "TARIFA"),
            &left.stamp,
            priority(&left.kind),
            &left.id,
        );
        let right_key = (
            u8::from(right.kind == "TARIFA"),
            &right.stamp,
            priority(&right.kind),
            &right.id,
        );
        left_key.cmp(&right_key)
    });

    let mut lowest = balance;
    let mut rejected: Vec<String> = Vec::new();
    for transaction in &transactions {
        if transaction.kind == "CREDITO" {
            balance += transaction.amount;
        } else if balance >= transaction.amount {
            balance -= transaction.amount;
        } else {
            rejected.push(transaction.id.clone());
        }
        lowest = lowest.min(balance);
    }

    let mut out = format!("{balance}\n{lowest}\n{}\n", rejected.len());
    for id in rejected {
        out.push_str(&id);
        out.push('\n');
    }
    out
}

pub fn generate(rng: &mut Rng) -> String {
    let count = rng.between(3, 60) as usize;
    let starved = rng.chance(20);
    let flush = rng.chance(20);
    let collision_heavy = rng.chance(60);

    let mut balance = if starved {
        rng.between(0, 5_000)
    } else {
        rng.between(100_000, 50_000_000)
    };
    let start = balance;

    let day = rng.between(1, 28);
    let month = rng.between(1, 12);
    let slot_count = if collision_heavy {
        rng.between(1, 4)
    } else {
        count as i64
    };
    let slots: Vec<String> = (0..slot_count)
        .map(|_| {
            format!(
                "2026-{month:02}-{day:02}T{:02}:{:02}:{:02}",
                rng.between(0, 23),
                rng.between(0, 59),
                rng.between(0, 59)
            )
        })
        .collect();

    let mut draft: Vec<(String, String, i64)> = Vec::new();
    for _ in 0..count {
        let kind = if rng.chance(12) {
            "TARIFA"
        } else if flush || rng.chance(45) {
            "CREDITO"
        } else {
            "DEBITO"
        };
        let amount = match kind {
            "TARIFA" => rng.between(50, 5_000),
            "CREDITO" => rng.between(1, 5_000_000),
            _ if starved => rng.between(1, 20_000_000),
            _ => rng.between(1, 3_000_000),
        };
        draft.push((kind.to_string(), rng.pick(&slots).clone(), amount));
    }

    for position in (1..draft.len()).rev() {
        let swap = rng.below(position as u64 + 1) as usize;
        draft.swap(position, swap);
    }
    let mut items: Vec<(String, String, String, i64)> = draft
        .into_iter()
        .enumerate()
        .map(|(index, (kind, stamp, amount))| {
            (format!("TXN{:04}", index + 1), stamp, kind, amount)
        })
        .collect();

    let mut settlement: Vec<usize> = (0..items.len()).collect();
    settlement.sort_by(|left, right| {
        let key = |at: &usize| {
            let item = &items[*at];
            (
                u8::from(item.2 == "TARIFA"),
                item.1.clone(),
                if item.2 == "DEBITO" { 1_u8 } else { 0 },
                item.0.clone(),
            )
        };
        key(left).cmp(&key(right))
    });

    for at in &settlement {
        let kind = items[*at].2.clone();
        if kind == "DEBITO" && balance > 0 && rng.chance(12) {
            items[*at].3 = balance;
        }
        let amount = items[*at].3;
        if kind == "CREDITO" {
            balance += amount;
        } else if balance >= amount {
            balance -= amount;
        }
    }

    let mut rows: Vec<String> = items
        .iter()
        .map(|(id, stamp, kind, amount)| format!("{id};{stamp};{kind};{amount}"))
        .collect();
    for position in (1..rows.len()).rev() {
        let swap = rng.below(position as u64 + 1) as usize;
        rows.swap(position, swap);
    }

    let mut lines = vec![start.to_string()];
    lines.extend(rows);
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproduces_the_first_statement_example() {
        let input = "10000\nTXN0003;2026-03-14T09:00:00;DEBITO;15000\nTXN0001;2026-03-14T09:00:00;CREDITO;20000\nTXN0002;2026-03-14T10:30:00;DEBITO;12000\n";
        assert_eq!(solve(input), "3000\n3000\n0\n");
    }

    #[test]
    fn reproduces_the_second_statement_example() {
        let input = "5000\nTXN0002;2026-03-14T08:00:00;DEBITO;7000\nTXN0005;2026-03-14T07:00:00;TARIFA;500\nTXN0001;2026-03-14T08:00:00;DEBITO;3000\nTXN0004;2026-03-14T12:00:00;CREDITO;10000\nTXN0003;2026-03-14T12:00:00;DEBITO;9000\n";
        assert_eq!(solve(input), "2500\n2000\n1\nTXN0002\n");
    }

    #[test]
    fn a_fee_is_always_settled_last() {
        let input = "1000\nTXN0002;2026-01-01T23:00:00;CREDITO;500\nTXN0001;2026-01-01T01:00:00;TARIFA;100\n";
        assert_eq!(solve(input), "1400\n1000\n0\n");
    }

    #[test]
    fn a_debit_equal_to_the_balance_is_accepted() {
        let input = "500\nTXN0001;2026-01-01T10:00:00;DEBITO;500\n";
        assert_eq!(solve(input), "0\n0\n0\n");
    }

    #[test]
    fn the_balance_never_goes_negative_on_generated_cases() {
        let mut rng = Rng::seeded("q3-invariants");
        let mut saw_rejection = false;
        let mut saw_clean = false;
        for _ in 0..400 {
            let case = generate(&mut rng);
            let answer = solve(&case);
            let rows: Vec<&str> = answer.lines().collect();
            let balance: i64 = rows[0].parse().unwrap();
            let lowest: i64 = rows[1].parse().unwrap();
            let rejected: usize = rows[2].parse().unwrap();
            assert!(balance >= 0, "negative balance in {case}");
            assert!(lowest >= 0);
            assert!(lowest <= balance || rejected > 0 || lowest <= balance);
            assert_eq!(rows.len(), 3 + rejected);
            if rejected > 0 {
                saw_rejection = true;
            } else {
                saw_clean = true;
            }
        }
        assert!(saw_rejection, "no generated case rejected a transaction");
        assert!(saw_clean, "every generated case had a rejection");
    }

    #[test]
    fn ids_do_not_encode_the_credit_before_debit_rule() {
        let mut rng = Rng::seeded("q3-priority");
        let mut leaky = 0;
        let mut informative = 0;
        for _ in 0..300 {
            let case = generate(&mut rng);
            let mut groups: std::collections::HashMap<String, Vec<(String, String)>> =
                std::collections::HashMap::new();
            for line in case.lines().skip(1) {
                let parts: Vec<&str> = line.split(';').collect();
                if parts[2] == "TARIFA" {
                    continue;
                }
                groups
                    .entry(parts[1].to_string())
                    .or_default()
                    .push((parts[0].to_string(), parts[2].to_string()));
            }
            for members in groups.values() {
                let credits: Vec<&String> = members
                    .iter()
                    .filter(|(_, kind)| kind == "CREDITO")
                    .map(|(id, _)| id)
                    .collect();
                let debits: Vec<&String> = members
                    .iter()
                    .filter(|(_, kind)| kind == "DEBITO")
                    .map(|(id, _)| id)
                    .collect();
                if credits.is_empty() || debits.is_empty() {
                    continue;
                }
                informative += 1;
                if debits.iter().any(|debit| credits.iter().any(|credit| debit < credit)) {
                    leaky += 1;
                }
            }
        }
        assert!(informative > 50, "only {informative} mixed groups appeared");
        assert!(
            leaky * 3 > informative,
            "only {leaky} of {informative} mixed groups need the priority rule"
        );
    }

    #[test]
    fn generated_cases_do_not_arrive_in_settlement_order() {
        let mut rng = Rng::seeded("q3-shuffled");
        let mut shuffled = 0;
        for _ in 0..200 {
            let case = generate(&mut rng);
            let ids: Vec<&str> = case
                .lines()
                .skip(1)
                .filter_map(|line| line.split(';').next())
                .collect();
            let mut sorted = ids.clone();
            sorted.sort();
            if ids != sorted {
                shuffled += 1;
            }
        }
        assert!(shuffled > 180, "only {shuffled} of 200 cases were shuffled");
    }

    #[test]
    fn some_generated_cases_debit_exactly_the_balance() {
        let mut rng = Rng::seeded("q3-exact");
        let mut exact = 0;
        for _ in 0..300 {
            let case = generate(&mut rng);
            let answer = solve(&case);
            if answer.lines().next() == Some("0") {
                exact += 1;
            }
        }
        assert!(exact > 0, "no case ever emptied the account exactly");
    }

    #[test]
    fn generated_cases_contain_timestamp_collisions() {
        let mut rng = Rng::seeded("q3-collisions");
        let mut collided = 0;
        for _ in 0..200 {
            let case = generate(&mut rng);
            let mut stamps: Vec<&str> = case
                .lines()
                .skip(1)
                .filter_map(|line| line.split(';').nth(1))
                .collect();
            let total = stamps.len();
            stamps.sort();
            stamps.dedup();
            if stamps.len() < total {
                collided += 1;
            }
        }
        assert!(collided > 50, "only {collided} of 200 cases had ties");
    }
}
