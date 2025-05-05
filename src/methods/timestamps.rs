//! Helpers for turning provider‑specific timestamp strings into `DateTime<Utc>`.
//! Each provider declares a `timestamp_format` (strftime pattern) and an optional
//! `tz_hint` stored in `transponder_companies`.

use chrono::{DateTime, FixedOffset, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Convert a timestamp string to **UTC**.
///
/// * `raw`  – literal text from the CSV.
/// * `fmt`  – chrono/strftime pattern that matches `raw` exactly.
/// * `tz_hint` –
///   * `Some("America/Chicago")` → attach that IANA zone (DST‑aware)
///   * `Some("-5")`              → attach a fixed offset in **hours**
///   * `None`                     → allowed only if `fmt` already contains `%z` or a literal `Z`
pub fn to_utc(raw: &str, fmt: &str, tz_hint: Option<String>) -> anyhow::Result<DateTime<Utc>> {
    //--------------------------------------------------------------------
    // 1. Pattern already carries numeric offset ( %z or %:z ) → parse directly.
    if fmt.contains("%z") || fmt.contains("%:z") {
        return Ok(DateTime::parse_from_str(raw, fmt)?.with_timezone(&Utc));
    }

    // 1‑b Pattern ends with a literal 'Z' (UTC designator).
    if fmt.ends_with('Z') {
        // Remove the trailing 'Z' in both raw & fmt, parse as Naive, then tag UTC.
        let raw_trim = raw.trim_end_matches('Z');
        let fmt_trim = fmt.trim_end_matches('Z');
        let naive = NaiveDateTime::parse_from_str(raw_trim, fmt_trim)?;
        return Ok(Utc.from_utc_datetime(&naive));
    }

    //--------------------------------------------------------------------
    // 2. Parse as NaiveDateTime first (no zone yet).
    //--------------------------------------------------------------------
    let naive = NaiveDateTime::parse_from_str(raw, fmt)?;

    match tz_hint {
        //--------------------------------------------------------
        // 2‑a IANA zone name (America/Chicago) – DST aware
        //--------------------------------------------------------
        Some(name) if name.contains('/') => {
            let tz: Tz = name.parse()?;
            match tz.from_local_datetime(&naive) {
                LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
                LocalResult::Ambiguous(dt, _) => Ok(dt.with_timezone(&Utc)), // pick earliest
                LocalResult::None => anyhow::bail!("{} is not a valid local time in {}", raw, name),
            }
        }
        //--------------------------------------------------------
        // 2‑b Fixed offset in hours ("-5", "+2", "0")
        //--------------------------------------------------------
        Some(hours) => {
            let h: i32 = hours.parse()?;
            let offset = FixedOffset::east_opt(h * 3600)
                .ok_or_else(|| anyhow::anyhow!("invalid offset {}", hours))?;
            match offset.from_local_datetime(&naive) {
                LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
                LocalResult::Ambiguous(dt, _) => Ok(dt.with_timezone(&Utc)),
                LocalResult::None => {
                    anyhow::bail!("{} is not a valid local time with offset {}", raw, hours)
                }
            }
        }
        //--------------------------------------------------------
        // 2‑c No hint → cannot disambiguate.
        //--------------------------------------------------------
        None => anyhow::bail!(
            "timestamp '{}' has no zone info and no tz_hint supplied",
            raw
        ),
    }
}

pub fn from_seconds(ts: i64) -> DateTime<Utc> {
    // ts have been in seconds since the Unix epoch
    let ndt = DateTime::from_timestamp(ts, 0).expect("invalid timestamp");
    ndt.with_timezone(&Utc)
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_z() {
        let t = to_utc("2025-04-08T17:31:13Z", "%Y-%m-%dT%H:%M:%SZ", None).unwrap();
        assert_eq!(t.to_rfc3339(), "2025-04-08T17:31:13+00:00");
    }

    #[test]
    fn iso_numeric_offset() {
        let t = to_utc("2025-04-08T13:31:13-04:00", "%Y-%m-%dT%H:%M:%S%:z", None).unwrap();
        assert_eq!(t.to_rfc3339(), "2025-04-08T17:31:13+00:00");
    }

    #[test]
    fn slash_fixed_offset() {
        let t = to_utc(
            "2025/04/13 17:44:24",
            "%Y/%m/%d %H:%M:%S",
            Some("-5".parse().unwrap()),
        )
        .unwrap();
        assert_eq!(t.to_rfc3339(), "2025-04-13T22:44:24+00:00");
    }

    #[test]
    fn slash_iana_zone() {
        let t = to_utc(
            "2025/07/13 17:44:24",
            "%Y/%m/%d %H:%M:%S",
            Some("America/Chicago".parse().unwrap()),
        )
        .unwrap();
        assert_eq!(t.to_rfc3339(), "2025-07-13T22:44:24+00:00");
    }

    #[test]
    fn missing_zone_error() {
        let err = to_utc("2025/04/13 17:44:24", "%Y/%m/%d %H:%M:%S", None).unwrap_err();
        assert!(err.to_string().contains("no zone info"));
    }

    #[test]
    fn iso_numeric_offset_no_colon() {
        // format with %z (±HHMM) instead of %:z
        let raw = "2025-04-08T133113-0400";
        let fmt = "%Y-%m-%dT%H%M%S%z";
        let dt = to_utc(raw, fmt, None).unwrap();
        assert_eq!(dt.to_rfc3339(), "2025-04-08T17:31:13+00:00");
    }
}
