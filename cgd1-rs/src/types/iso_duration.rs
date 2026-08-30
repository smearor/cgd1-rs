/// Parse an ISO 8601 duration string into total seconds.
///
/// Supports the subset: `P[n]W`, `P[n]D`, `PT[n]H`, `PT[n]M`, `PT[n]S`
/// (and combinations like `PT1H30M`, `P1DT12H`).
///
/// Returns `None` if the string is not a valid ISO 8601 duration.
pub fn parse_iso_duration_to_seconds(s: &str) -> Option<u64> {
    let s = s.strip_prefix('P')?;
    let mut total_seconds: u64 = 0;
    let mut in_time = false;
    let mut num_buf = String::new();
    let mut seen_component = false;

    for ch in s.chars() {
        match ch {
            'T' => {
                if in_time || !num_buf.is_empty() {
                    return None;
                }
                in_time = true;
            }
            '0'..='9' | '.' => {
                num_buf.push(ch);
            }
            'W' if !in_time => {
                let n: f64 = num_buf.parse().ok()?;
                total_seconds = total_seconds.checked_add((n * 604800.0) as u64)?;
                num_buf.clear();
                seen_component = true;
            }
            'D' if !in_time => {
                let n: f64 = num_buf.parse().ok()?;
                total_seconds = total_seconds.checked_add((n * 86400.0) as u64)?;
                num_buf.clear();
                seen_component = true;
            }
            'H' if in_time => {
                let n: f64 = num_buf.parse().ok()?;
                total_seconds = total_seconds.checked_add((n * 3600.0) as u64)?;
                num_buf.clear();
                seen_component = true;
            }
            'M' if in_time => {
                let n: f64 = num_buf.parse().ok()?;
                total_seconds = total_seconds.checked_add((n * 60.0) as u64)?;
                num_buf.clear();
                seen_component = true;
            }
            'S' if in_time => {
                let n: f64 = num_buf.parse().ok()?;
                total_seconds = total_seconds.checked_add(n as u64)?;
                num_buf.clear();
                seen_component = true;
            }
            _ => return None,
        }
    }

    if !num_buf.is_empty() || !seen_component {
        return None;
    }

    Some(total_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seconds() {
        assert_eq!(parse_iso_duration_to_seconds("PT30S"), Some(30));
        assert_eq!(parse_iso_duration_to_seconds("PT0S"), Some(0));
    }

    #[test]
    fn parse_minutes() {
        assert_eq!(parse_iso_duration_to_seconds("PT5M"), Some(300));
        assert_eq!(parse_iso_duration_to_seconds("PT1M30S"), Some(90));
    }

    #[test]
    fn parse_hours() {
        assert_eq!(parse_iso_duration_to_seconds("PT1H"), Some(3600));
        assert_eq!(parse_iso_duration_to_seconds("PT1H30M"), Some(5400));
        assert_eq!(parse_iso_duration_to_seconds("PT1H30M15S"), Some(5415));
    }

    #[test]
    fn parse_days_weeks() {
        assert_eq!(parse_iso_duration_to_seconds("P1D"), Some(86400));
        assert_eq!(parse_iso_duration_to_seconds("P1W"), Some(604800));
        assert_eq!(parse_iso_duration_to_seconds("P1DT12H"), Some(129600));
    }

    #[test]
    fn parse_invalid() {
        assert_eq!(parse_iso_duration_to_seconds("30"), None);
        assert_eq!(parse_iso_duration_to_seconds("PT"), None);
        assert_eq!(parse_iso_duration_to_seconds("P"), None);
        assert_eq!(parse_iso_duration_to_seconds("XYZ"), None);
        assert_eq!(parse_iso_duration_to_seconds("P1H"), None);
        assert_eq!(parse_iso_duration_to_seconds("PT1D"), None);
    }
}
