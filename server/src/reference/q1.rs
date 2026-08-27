use crate::rng::Rng;

pub fn solve(input: &str) -> String {
    let mut tokens = input.split_whitespace();
    let count: usize = tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0);

    let mut assets = 0.0_f64;
    for _ in 0..count {
        tokens.next();
        let quantity: i64 = tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let price: f64 = tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        assets += quantity as f64 * price;
    }

    let liabilities_count: usize = tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut liabilities = 0.0_f64;
    for _ in 0..liabilities_count {
        tokens.next();
        liabilities += tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    }

    format!("{:.2}\n", assets - liabilities)
}

pub fn generate(rng: &mut Rng) -> String {
    let heavy_liabilities = rng.chance(25);
    let assets = rng.between(1, 30) as usize;
    let liabilities = if rng.chance(15) { 0 } else { rng.between(1, 8) as usize };

    let mut lines = vec![assets.to_string()];
    let mut asset_total = 0.0_f64;
    for index in 0..assets {
        let quantity = rng.between(1, 5_000);
        let cents = rng.between(1, 2_000_000);
        let price = cents as f64 / 100.0;
        asset_total += quantity as f64 * price;
        lines.push(format!("ATIVO-{index:04} {quantity} {price:.2}"));
    }

    lines.push(liabilities.to_string());
    let ceiling = if heavy_liabilities {
        (asset_total * 1.6).max(1.0)
    } else {
        (asset_total * 0.4).max(1.0)
    };
    for index in 0..liabilities {
        let cents = rng.between(1, (ceiling * 100.0 / liabilities as f64).max(1.0) as i64);
        lines.push(format!("PASSIVO-{index:02} {:.2}", cents as f64 / 100.0));
    }

    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproduces_the_statement_example() {
        let input = "3\nLFT-2029 1500 14320.50\nDEB-XPTO21 800 1050.00\nCDB-BANCOX 200 1230.75\n2\nTAXA_ADMIN 12500.00\nRESGATES_A_PAGAR 340000.00\n";
        assert_eq!(solve(input), "22214400.00\n");
    }

    #[test]
    fn handles_no_liabilities() {
        assert_eq!(solve("1\nCDB-BANCOY 10 100.00\n0\n"), "1000.00\n");
    }

    #[test]
    fn generated_cases_stay_inside_the_stated_limits() {
        let mut rng = Rng::seeded("q1-limits");
        for _ in 0..300 {
            let case = generate(&mut rng);
            let mut lines = case.lines();
            let assets: usize = lines.next().unwrap().parse().unwrap();
            assert!((1..=1000).contains(&assets));
            for _ in 0..assets {
                let row = lines.next().unwrap();
                assert_eq!(row.split_whitespace().count(), 3);
            }
            let liabilities: usize = lines.next().unwrap().parse().unwrap();
            assert!(liabilities <= 100);
            for _ in 0..liabilities {
                assert_eq!(lines.next().unwrap().split_whitespace().count(), 2);
            }
            assert!(lines.next().is_none());
            assert!(solve(&case).ends_with('\n'));
        }
    }

    #[test]
    fn some_generated_cases_land_on_a_negative_result() {
        let mut rng = Rng::seeded("q1-negative");
        let negatives = (0..200)
            .filter(|_| solve(&generate(&mut rng)).starts_with('-'))
            .count();
        assert!(negatives > 0, "no generated case produced a negative PL");
    }
}
