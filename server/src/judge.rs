use serde_json::Value;

use crate::questions::Mode;

const ABSOLUTE_TOLERANCE: f64 = 0.01;
const RELATIVE_TOLERANCE: f64 = 1e-6;

#[derive(Debug, Clone)]
pub struct Mismatch {
    pub line: Option<usize>,
    pub detail: String,
}

impl Mismatch {
    fn at(line: usize, detail: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            detail: detail.into(),
        }
    }

    fn whole(detail: impl Into<String>) -> Self {
        Self {
            line: None,
            detail: detail.into(),
        }
    }
}

pub type Verdict = Result<(), Mismatch>;

fn rows(text: &str) -> Vec<String> {
    let mut out: Vec<String> = text
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();
    while out.last().map(|line| line.is_empty()).unwrap_or(false) {
        out.pop();
    }
    out
}

const MAX_SCALE: usize = 18;
const MAX_DIGITS: usize = 30;
const ECHO_LIMIT: usize = 60;

fn brief(token: &str) -> String {
    if token.chars().count() <= ECHO_LIMIT {
        return token.to_string();
    }
    format!("{}…", token.chars().take(ECHO_LIMIT).collect::<String>())
}

fn plain_parts(token: &str) -> Option<(bool, String, String)> {
    let text = token.trim();
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };

    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], rest[at + 1..].parse::<i32>().ok()?),
        None => (rest, 0),
    };
    let (whole, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    if !whole.bytes().chain(frac.bytes()).all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let digits = format!("{whole}{frac}");
    let point = whole.len() as i32 + exponent;
    if !(-40..=40).contains(&point) {
        return None;
    }

    if point <= 0 {
        let pad = "0".repeat(-point as usize);
        return Some((negative, "0".to_string(), format!("{pad}{digits}")));
    }
    let point = point as usize;
    if point >= digits.len() {
        let pad = "0".repeat(point - digits.len());
        return Some((negative, format!("{digits}{pad}"), String::new()));
    }
    Some((
        negative,
        digits[..point].to_string(),
        digits[point..].to_string(),
    ))
}

fn scaled_value(negative: bool, whole: &str, frac: &str, scale: usize) -> Option<i128> {
    if whole.len() + scale > MAX_DIGITS {
        return None;
    }
    let mut frac = frac.to_string();
    frac.truncate(scale);
    while frac.len() < scale {
        frac.push('0');
    }
    let digits: i128 = format!("{whole}{frac}").parse().ok()?;
    Some(if negative { -digits } else { digits })
}

fn decimal_close(expected: &str, got: &str, scaled: bool) -> Option<bool> {
    let (want_negative, want_whole, want_frac) = plain_parts(expected)?;
    let (have_negative, have_whole, have_frac) = plain_parts(got)?;
    let scale = want_frac.len().max(have_frac.len()).min(MAX_SCALE);

    let want = scaled_value(want_negative, &want_whole, &want_frac, scale)?;
    let have = scaled_value(have_negative, &have_whole, &have_frac, scale)?;
    let gap = (want - have).checked_abs()?;

    let within_absolute = gap.checked_mul(100)? <= 10_i128.checked_pow(scale as u32)?;
    if !scaled {
        return Some(within_absolute);
    }
    let within_relative = gap.checked_mul(1_000_000)? <= want.abs();
    Some(within_absolute || within_relative)
}

fn close_enough(expected: &str, got: &str, scaled: bool) -> bool {
    if let Some(verdict) = decimal_close(expected, got, scaled) {
        return verdict;
    }
    let (Ok(want), Ok(have)) = (expected.parse::<f64>(), got.parse::<f64>()) else {
        return false;
    };
    let allowed = if scaled {
        ABSOLUTE_TOLERANCE.max(RELATIVE_TOLERANCE * want.abs())
    } else {
        ABSOLUTE_TOLERANCE
    };
    (want - have).abs() <= allowed
}

fn shape(rows_expected: usize, rows_got: usize) -> Verdict {
    if rows_expected == rows_got {
        return Ok(());
    }
    Err(Mismatch::whole(format!(
        "esperava {rows_expected} linha(s) e recebi {rows_got}"
    )))
}

fn check_text(expected: &str, got: &str, numeric: bool) -> Verdict {
    let expected_rows = rows(expected);
    let got_rows = rows(got);
    shape(expected_rows.len(), got_rows.len())?;

    for (index, (want, have)) in expected_rows.iter().zip(got_rows.iter()).enumerate() {
        let line = index + 1;
        if want == have {
            continue;
        }
        if !numeric {
            return Err(Mismatch::at(
                line,
                format!("esperava `{}` e recebi `{}`", brief(want), brief(have)),
            ));
        }

        let want_tokens: Vec<&str> = want.split_whitespace().collect();
        let have_tokens: Vec<&str> = have.split_whitespace().collect();
        if want_tokens.len() != have_tokens.len() {
            return Err(Mismatch::at(
                line,
                format!(
                    "esperava {} valor(es) e recebi {}",
                    want_tokens.len(),
                    have_tokens.len()
                ),
            ));
        }

        for (want_token, have_token) in want_tokens.iter().zip(have_tokens.iter()) {
            if want_token == have_token {
                continue;
            }
            if !close_enough(want_token, have_token, false) {
                return Err(Mismatch::at(
                    line,
                    format!("esperava `{}` e recebi `{}`", brief(want_token), brief(have_token)),
                ));
            }
        }
    }
    Ok(())
}

fn check_json_value(path: &str, expected: &Value, got: &Value) -> Verdict {
    let here = if path.is_empty() { "raiz" } else { path };
    match (expected, got) {
        (Value::Object(want), Value::Object(have)) => {
            for key in want.keys() {
                if !have.contains_key(key) {
                    return Err(Mismatch::whole(format!("falta a chave `{key}` em {here}")));
                }
            }
            for key in have.keys() {
                if !want.contains_key(key) {
                    return Err(Mismatch::whole(format!("chave `{key}` sobrando em {here}")));
                }
            }
            for (key, want_value) in want {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                check_json_value(&child, want_value, &have[key])?;
            }
            Ok(())
        }
        (Value::Array(want), Value::Array(have)) => {
            if want.len() != have.len() {
                return Err(Mismatch::whole(format!(
                    "{here} esperava {} item(ns) e recebi {}",
                    want.len(),
                    have.len()
                )));
            }
            for (index, (want_value, have_value)) in want.iter().zip(have.iter()).enumerate() {
                check_json_value(&format!("{path}[{index}]"), want_value, have_value)?;
            }
            Ok(())
        }
        (Value::Number(want), Value::Number(have)) => {
            if close_enough(&want.to_string(), &have.to_string(), true) {
                return Ok(());
            }
            Err(Mismatch::whole(format!(
                "{here} esperava {want} e recebi {have}"
            )))
        }
        (want, have) if want == have => Ok(()),
        (want, have) => Err(Mismatch::whole(format!(
            "{here} esperava `{want}` e recebi `{have}`"
        ))),
    }
}

fn check_json(expected: &str, got: &str) -> Verdict {
    let want: Value = serde_json::from_str(expected)
        .map_err(|problem| Mismatch::whole(format!("gabarito ilegível: {problem}")))?;
    let have: Value = serde_json::from_str(got.trim())
        .map_err(|problem| Mismatch::whole(format!("isso não é um JSON válido: {problem}")))?;
    check_json_value("", &want, &have)
}

pub fn check(mode: Mode, expected: &str, got: &str) -> Verdict {
    let got = got.trim_start_matches('\u{feff}');
    if got.trim().is_empty() {
        return Err(Mismatch::whole("resposta vazia".to_string()));
    }
    match mode {
        Mode::Exact => check_text(expected, got, false),
        Mode::Numeric => check_text(expected, got, true),
        Mode::Json => check_json(expected, got),
        Mode::Api => Err(Mismatch::whole(
            "esta questão é conferida na API, não por arquivo".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_whitespace_and_newlines_do_not_matter() {
        assert!(check(Mode::Exact, "3000\n3000\n0\n", "3000  \n3000\n0").is_ok());
        assert!(check(Mode::Exact, "3000\n3000\n0", "3000\r\n3000\r\n0\r\n").is_ok());
    }

    #[test]
    fn a_carriage_return_inside_a_token_is_not_swallowed() {
        assert!(check(Mode::Exact, "3000", "30\r00").is_err());
    }

    #[test]
    fn numeric_tolerance_is_one_cent_even_on_large_values() {
        assert!(check(Mode::Numeric, "22214400.00", "22214400.01").is_ok());
        assert!(check(Mode::Numeric, "22214400.00", "22214400.02").is_err());
        assert!(check(Mode::Numeric, "22214400.00", "22214410.00").is_err());
    }

    #[test]
    fn one_cent_is_inside_the_window_and_two_cents_is_not() {
        assert!(close_enough("22214400.00", "22214400.01", false));
        assert!(!close_enough("22214400.00", "22214400.02", false));
        assert!(close_enough("0.00", "0.01", false));
        assert!(!close_enough("0.00", "0.011", false));
    }

    #[test]
    fn scientific_notation_is_expanded_not_approximated() {
        assert!(close_enough("0.00", "1e-9", false));
        assert!(!close_enough("0.00", "1e9", false));
        assert!(close_enough("25717594645.37", "2.571759464538e10", false));
        assert!(close_enough("22214400.00", "2.221440001e7", false));
        assert!(!close_enough("22214400.00", "2.221440002e7", false));
    }

    #[test]
    fn extra_decimal_places_do_not_flip_the_boundary() {
        assert!(close_enough("25717594645.37", "25717594645.3800000000000", false));
        assert!(close_enough("22214400.00", "22214400.0100000000000", false));
        assert!(!close_enough("749.50", "749.51000000000001", false));
    }

    #[test]
    fn a_byte_order_mark_does_not_break_a_correct_answer() {
        assert!(check(Mode::Numeric, "749.50", "\u{feff}749.50\n").is_ok());
        assert!(check(Mode::Json, r#"{"a":1.0}"#, "\u{feff}{\"a\":1.0}").is_ok());
    }

    #[test]
    fn a_long_token_is_not_echoed_whole() {
        let flood = "9".repeat(200_000);
        let verdict = check(Mode::Numeric, "1.00", &flood);
        let detail = verdict.expect_err("should reject").detail;
        assert!(detail.len() < 200, "detail was {} chars", detail.len());
    }

    #[test]
    fn negative_zero_matches_zero() {
        assert!(check(Mode::Numeric, "0.00", "-0.00").is_ok());
        assert!(check(Mode::Numeric, "3 -0.00", "3 0.00").is_ok());
    }

    #[test]
    fn exact_mode_rejects_a_reformatted_integer() {
        assert!(check(Mode::Exact, "3000", "3000.0").is_err());
    }

    #[test]
    fn numeric_mode_catches_a_wrong_day() {
        assert!(check(Mode::Numeric, "4 3.96", "3 3.96").is_err());
    }

    #[test]
    fn json_ignores_key_order_and_formatting() {
        let expected = r#"{"A":{"resultado":360.58},"B":{"resultado":180.29}}"#;
        let got = "{\n \"B\": {\"resultado\": 180.29},\n \"A\": {\"resultado\": 360.5801}\n}";
        assert!(check(Mode::Json, expected, got).is_ok());
    }

    #[test]
    fn json_relative_tolerance_scales_with_the_number() {
        assert!(check(Mode::Json, r#"{"a":1000000000.0}"#, r#"{"a":1000000500.0}"#).is_ok());
        assert!(check(Mode::Json, r#"{"a":1000000000.0}"#, r#"{"a":1000005000.0}"#).is_err());
    }

    #[test]
    fn json_missing_investor_is_caught() {
        assert!(check(Mode::Json, r#"{"A":1.0,"B":2.0}"#, r#"{"A":1.0}"#).is_err());
    }

    #[test]
    fn line_count_mismatch_is_reported() {
        let verdict = check(Mode::Numeric, "5.00\n4 3.96", "5.00");
        assert!(verdict.is_err());
    }
}
