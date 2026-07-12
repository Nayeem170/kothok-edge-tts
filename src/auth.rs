//! `Sec-MS-GEC` DRM token generation and Edge-protocol time helpers.
//!
//! The Edge Read-Aloud endpoint requires a `Sec-MS-GEC` query parameter whose
//! value is a SHA-256 hash derived from the current Windows file-time and a
//! fixed client token.  The token is valid inside a rolling 5-minute window;
//! when the server's clock and ours disagree by more than that, the handshake
//! returns HTTP 403 and the connection layer retries with an adjusted skew.

use rand::RngCore;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Long-stable trusted-client token embedded in the Edge extension.
/// (Source: `edge-tts` / `rany2` - unchanged for years.)
pub(crate) const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";

/// Edge stable-channel version string.  Microsoft rotates this on release
/// cycles; when every synthesis starts returning HTTP 403, re-pin this to the
/// current Edge version and republish the crate.
pub(crate) const SEC_MS_GEC_VERSION: &str = "1-143.0.3650.75";

/// Seconds between 1601-01-01 (Windows epoch) and 1970-01-01 (Unix epoch).
const WIN_EPOCH: u64 = 11_644_473_600;

/// The token is quantised to this window (5 minutes).  Values outside the
/// current window are rejected by the server.
const TICK_WINDOW_SECS: u64 = 300;

/// 100-nanosecond intervals per second (Windows file-time granularity).
const INTERVALS_PER_SEC: u64 = 10_000_000;

/// Generate a random uppercase-hex string of `nbytes * 2` hex digits.
///
/// Used for the `MUID` cookie, `ConnectionId`, and `X-RequestId` values.
pub(crate) fn random_hex(nbytes: usize) -> String {
    let mut buf = vec![0u8; nbytes];
    rand::thread_rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02X}")).collect()
}

/// Current Unix timestamp adjusted by `skew` seconds (0 when clocks agree).
fn unix_secs(skew: i64) -> u64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    (secs + skew).max(0) as u64
}

/// Compute the `Sec-MS-GEC` token for the given clock skew.
///
/// Algorithm: round the current Windows file-time down to a 5-min boundary,
/// concatenate with the trusted client token, SHA-256 hash, uppercase-hex
/// encode.  Matches `edge-tts` `DRM.generate_sec_ms_gec`.
pub(crate) fn sec_ms_gec(skew: i64) -> String {
    let ticks = unix_secs(skew) + WIN_EPOCH;
    let floored = ticks - (ticks % TICK_WINDOW_SECS);
    let intervals = floored * INTERVALS_PER_SEC;

    let mut hasher = Sha256::new();
    hasher.update(format!("{intervals}{TRUSTED_CLIENT_TOKEN}").as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect()
}

// the Edge endpoint expects a JavaScript-style date string in
// the X-Timestamp header; we compute it manually (no chrono dep).

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const SECS_PER_MINUTE: u64 = 60;
const SECS_PER_HOUR: u64 = 3_600;
const SECS_PER_DAY: u64 = 86_400;
const HOURS_PER_DAY: u64 = 24;
const DAYS_PER_WEEK: usize = 7;
const DAYS_PER_400_YEARS: i64 = 146_097;

/// Broken-down UTC date-time used to build the `X-Timestamp` header.
struct CivilDateTime {
    weekday: usize,
    year: u64,
    month: usize,
    day: u64,
    hour: u64,
    minute: u64,
    second: u64,
}

/// Convert a Unix timestamp (UTC) into a broken-down date-time.
///
/// Uses Howard Hinnant's `days_from_civil` inverse algorithm - no external
/// date crate required.
fn civil_utc(secs: u64) -> CivilDateTime {
    let second = secs % SECS_PER_MINUTE;
    let minute = (secs / SECS_PER_MINUTE) % SECS_PER_MINUTE;
    let hour = (secs / SECS_PER_HOUR) % HOURS_PER_DAY;
    let days = (secs / SECS_PER_DAY) as i64;

    // 1970-01-01 was a Thursday (index 4).
    let weekday = ((days + 4).rem_euclid(DAYS_PER_WEEK as i64)) as usize;

    // Howard Hinnant's civil_from_days: 719_468 = days from 0000-03-01 to
    // 1970-01-01. Remaining constants are integral to the algorithm -
    // see https://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = if z >= 0 {
        z / DAYS_PER_400_YEARS
    } else {
        (z - (DAYS_PER_400_YEARS - 1)) / DAYS_PER_400_YEARS
    };
    let doe = (z - era * DAYS_PER_400_YEARS) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as usize;
    let year = (y + if month <= 2 { 1 } else { 0 }) as u64;

    CivilDateTime {
        weekday,
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

/// Format a Unix timestamp as a JavaScript-style date string (Edge
/// `X-Timestamp` header format).  Pure - no clock access.
pub(crate) fn date_string(secs: u64) -> String {
    let dt = civil_utc(secs);
    format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        WEEKDAYS.get(dt.weekday).copied().unwrap_or("Sun"),
        MONTHS
            .get(dt.month.wrapping_sub(1))
            .copied()
            .unwrap_or("Jan"),
        dt.day,
        dt.year,
        dt.hour,
        dt.minute,
        dt.second,
    )
}

/// Current time formatted for the `X-Timestamp` header.
pub(crate) fn current_date_string() -> String {
    date_string(unix_secs(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_utc_known_epoch() {
        let dt = civil_utc(0);
        assert_eq!((dt.year, dt.month, dt.day), (1970, 1, 1));
        assert_eq!(dt.weekday, 4); // Thursday
    }

    #[test]
    fn civil_utc_known_date() {
        // 1_735_689_600 = 2025-01-01 00:00:00 UTC (Wednesday)
        let dt = civil_utc(1_735_689_600);
        assert_eq!((dt.year, dt.month, dt.day), (2025, 1, 1));
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.weekday, 3); // Wednesday
    }

    #[test]
    fn sec_ms_gec_is_hex_uppercase() {
        let token = sec_ms_gec(0);
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!token.chars().any(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn sec_ms_gec_deterministic_within_window() {
        let a = sec_ms_gec(0);
        let b = sec_ms_gec(1);
        assert_eq!(a, b, "tokens within the same 5-min window must match");
    }

    #[test]
    fn date_string_format() {
        let s = date_string(0);
        assert!(s.starts_with("Thu Jan 01 1970 00:00:00 GMT+0000"));
    }

    #[test]
    fn date_string_midday() {
        // 43_200 = 12 hours after epoch
        let s = date_string(43_200);
        assert!(s.starts_with("Thu Jan 01 1970 12:00:00 GMT+0000"));
    }

    #[test]
    fn random_hex_length() {
        assert_eq!(random_hex(16).len(), 32);
        assert_ne!(random_hex(16), random_hex(16));
    }

    #[test]
    fn random_hex_zero_bytes() {
        assert_eq!(random_hex(0), "");
    }

    #[test]
    fn civil_utc_leap_day() {
        // 1_709_164_800 = 2024-02-29 00:00:00 UTC (leap year)
        let dt = civil_utc(1_709_164_800);
        assert_eq!((dt.year, dt.month, dt.day), (2024, 2, 29));
    }

    #[test]
    fn civil_utc_end_of_year() {
        // 1_735_689_599 = 2024-12-31 23:59:59 UTC
        let dt = civil_utc(1_735_689_599);
        assert_eq!((dt.year, dt.month, dt.day), (2024, 12, 31));
        assert_eq!(dt.hour, 23);
        assert_eq!(dt.minute, 59);
        assert_eq!(dt.second, 59);
    }

    #[test]
    fn date_string_full_timestamp() {
        // 1_709_164_800 = 2024-02-29 00:00:00 UTC (Thursday)
        let s = date_string(1_709_164_800);
        assert!(s.starts_with("Thu Feb 29 2024 00:00:00 GMT+0000"));
    }
}
