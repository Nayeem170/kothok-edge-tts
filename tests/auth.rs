//! Integration tests for the auth token and date helpers in `auth.rs`.
//!
//! These cover the pure token/date functions (`sec_ms_gec`, `random_hex`,
//! `civil_utc`, `date_string`). They are exposed from the crate root with
//! `#[doc(hidden)]` solely so they can be exercised here without an inline
//! `#[cfg(test)]` block.

use kothok_edge_tts::{civil_utc, date_string, random_hex, sec_ms_gec};

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
