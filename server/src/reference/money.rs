pub fn round_two(value: f64) -> f64 {
    format!("{value:.2}").parse().unwrap_or(value)
}

pub fn two_places(value: f64) -> String {
    let text = format!("{value:.2}");
    if text == "-0.00" {
        return "0.00".to_string();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_negative_zero() {
        assert_eq!(two_places(-0.001), "0.00");
        assert_eq!(two_places(-0.0), "0.00");
        assert_eq!(two_places(0.0), "0.00");
    }

    #[test]
    fn keeps_real_negatives() {
        assert_eq!(two_places(-0.53), "-0.53");
        assert_eq!(two_places(-15.0), "-15.00");
    }

    #[test]
    fn rounding_is_idempotent() {
        for raw in [360.5849, 0.005, 2.675, 1.005, 100000.0, -0.125] {
            let once = round_two(raw);
            assert_eq!(once, round_two(once), "{raw} was not stable");
            assert_eq!(format!("{once:.2}"), format!("{:.2}", raw));
        }
    }
}
