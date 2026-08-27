use crate::rng::Rng;

use super::money;

fn show(value: f64) -> String {
    money::two_places(value)
}

pub fn solve(input: &str) -> String {
    let mut tokens = input.split_whitespace();
    let days: usize = tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let quotas: Vec<f64> = (0..days)
        .filter_map(|_| tokens.next().and_then(|v| v.parse::<f64>().ok()))
        .collect();
    if quotas.len() < 2 {
        return String::new();
    }

    let accumulated = (quotas[quotas.len() - 1] / quotas[0] - 1.0) * 100.0;

    let mut best_day = 2_usize;
    let mut best_change = (quotas[1] / quotas[0] - 1.0) * 100.0;
    for day in 3..=quotas.len() {
        let change = (quotas[day - 1] / quotas[day - 2] - 1.0) * 100.0;
        if change > best_change {
            best_change = change;
            best_day = day;
        }
    }

    format!("{}\n{} {}\n", show(accumulated), best_day, show(best_change))
}

pub fn generate(rng: &mut Rng) -> String {
    let days = rng.between(2, 30) as usize;
    let only_falls = rng.chance(20);
    let flat_stretch = rng.chance(20);

    let mut quota = rng.between(50_000_000, 300_000_000);
    let mut lines = vec![days.to_string()];
    let mut steps: Vec<i64> = vec![quota];
    for _ in 1..days {
        let drift = if only_falls {
            -rng.between(1, 4_000_000)
        } else if flat_stretch && rng.chance(40) {
            0
        } else {
            rng.between(-4_000_000, 4_000_000)
        };
        quota = (quota + drift).max(1_000_000);
        steps.push(quota);
    }

    if days > 3 && rng.chance(45) {
        let mut best = 0_usize;
        for index in 1..steps.len() - 1 {
            let candidate = steps[index + 1] as f64 / steps[index] as f64;
            let current = steps[best + 1] as f64 / steps[best] as f64;
            if candidate > current {
                best = index;
            }
        }
        let mut target = rng.between(1, steps.len() as i64 - 2) as usize;
        if target == best {
            target = if best + 1 < steps.len() - 1 { best + 1 } else { best - 1 };
        }
        steps[target] = steps[best];
        steps[target + 1] = steps[best + 1];
    }

    for value in steps {
        lines.push(format!("{:.6}", value as f64 / 1_000_000.0));
    }

    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproduces_the_statement_examples() {
        let rising = "4\n100.000000\n102.000000\n101.000000\n105.000000\n";
        assert_eq!(solve(rising), "5.00\n4 3.96\n");
        let falling = "4\n200.000000\n190.000000\n189.000000\n170.000000\n";
        assert_eq!(solve(falling), "-15.00\n3 -0.53\n");
    }

    #[test]
    fn a_tie_on_the_best_change_keeps_the_earlier_day() {
        let doubled = "4\n100.000000\n110.000000\n121.000000\n133.100000\n";
        let answer = solve(doubled);
        assert!(answer.lines().nth(1).unwrap().starts_with("2 "), "{answer}");
    }

    #[test]
    fn negative_zero_is_normalised() {
        assert_eq!(show(-0.001), "0.00");
        assert_eq!(show(-0.0), "0.00");
    }

    #[test]
    fn some_generated_cases_tie_on_the_best_day() {
        let mut rng = Rng::seeded("q2-ties");
        let mut ties = 0;
        for _ in 0..400 {
            let case = generate(&mut rng);
            let values: Vec<f64> = case
                .lines()
                .skip(1)
                .filter_map(|line| line.parse().ok())
                .collect();
            let changes: Vec<f64> = values
                .windows(2)
                .map(|pair| (pair[1] / pair[0] - 1.0) * 100.0)
                .collect();
            let best = changes.iter().cloned().fold(f64::MIN, f64::max);
            if changes.iter().filter(|change| **change == best).count() > 1 {
                ties += 1;
            }
        }
        assert!(ties > 40, "only {ties} of 400 cases tied on the best day");
    }

    #[test]
    fn generated_cases_respect_the_limits() {
        let mut rng = Rng::seeded("q2-limits");
        for _ in 0..300 {
            let case = generate(&mut rng);
            let mut lines = case.lines();
            let days: usize = lines.next().unwrap().parse().unwrap();
            assert!((2..=1000).contains(&days));
            let quotas: Vec<&str> = lines.collect();
            assert_eq!(quotas.len(), days);
            for quota in quotas {
                let value: f64 = quota.parse().expect("quota parses");
                assert!(value > 0.0);
                assert_eq!(quota.split('.').nth(1).unwrap().len(), 6);
            }
            let answer = solve(&case);
            let best = answer.lines().nth(1).unwrap();
            let day: usize = best.split(' ').next().unwrap().parse().unwrap();
            assert!((2..=days).contains(&day));
        }
    }
}
