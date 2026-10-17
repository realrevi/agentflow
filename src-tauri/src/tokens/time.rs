//! Calendar math on ms timestamps without a date crate: days since the epoch, months, and
//! civil dates (Howard Hinnant's algorithm). A fixed UTC offset makes "today" local.

const DAY_MS: i64 = 86_400_000;

/// Days since 1970-01-01 at `offset_minutes` east of UTC.
pub fn day_index(at_ms: u64, offset_minutes: i32) -> i64 {
    let local = at_ms as i64 + i64::from(offset_minutes) * 60_000;
    local.div_euclid(DAY_MS)
}

/// (year, month 1-12, day 1-31) of a day index.
pub fn civil(day: i64) -> (i64, u32, u32) {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// "2026-10" for the ledger file name, from a UTC timestamp.
pub fn month_key(at_ms: u64) -> String {
    let (y, m, _) = civil(day_index(at_ms, 0));
    format!("{y:04}-{m:02}")
}

/// `YYYY-MM-DD` of a day index.
pub fn date_label(day: i64) -> String {
    let (y, m, d) = civil(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Parses an RFC 3339 / ISO 8601 UTC timestamp (`2026-10-17T08:50:23.443Z`, an offset is
/// applied when given) into ms since the epoch.
pub fn parse_iso(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    if bytes.len() < 19 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[13] != b':' {
        return None;
    }
    let num = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let mut rest = &text[19..];
    let mut millis = 0i64;
    if let Some(frac) = rest.strip_prefix('.') {
        let digits: String = frac.chars().take_while(char::is_ascii_digit).collect();
        rest = &frac[digits.len()..];
        let padded = format!("{digits:0<3}");
        millis = padded.get(..3)?.parse().ok()?;
    }
    let offset_min = match rest {
        "" | "Z" | "z" => 0,
        tz if tz.len() == 6 && (tz.starts_with('+') || tz.starts_with('-')) => {
            let sign = if tz.starts_with('-') { -1 } else { 1 };
            let hh: i64 = tz.get(1..3)?.parse().ok()?;
            let mm: i64 = tz.get(4..6)?.parse().ok()?;
            sign * (hh * 60 + mm)
        }
        _ => return None,
    };
    let days = days_from_civil(y, mo as u32, d as u32);
    let ms = ((days * 24 + h) * 60 + mi - offset_min) * 60_000 + s * 1000 + millis;
    u64::try_from(ms).ok()
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iso_timestamps() {
        let at = parse_iso("2026-10-17T08:50:23.443Z").expect("parse");
        assert_eq!(month_key(at), "2026-10");
        assert_eq!(date_label(day_index(at, 0)), "2026-10-17");
        assert_eq!(parse_iso("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso("1970-01-01T01:00:00+01:00"), Some(0));
        assert_eq!(
            parse_iso("2026-10-05T19:46:24.5Z").map(|ms| ms % 1000),
            Some(500)
        );
        assert_eq!(parse_iso("not a date"), None);
        assert_eq!(parse_iso("2026-13-01T00:00:00Z"), None);
    }

    #[test]
    fn local_days_and_months() {
        let at = parse_iso("2026-10-31T23:30:00Z").expect("parse");
        assert_eq!(date_label(day_index(at, 0)), "2026-10-31");
        assert_eq!(date_label(day_index(at, 60)), "2026-11-01");
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    }
}
