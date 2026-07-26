//! Token-rotation health check.
//!
//! This integration test performs a **live** synthesis against the Edge
//! endpoint.  It is `#[ignore]`d so it never runs in normal `cargo test` (no
//! network dependency for unit tests).  The CI workflow runs it daily via
//! `cargo test --test token_rotation -- --ignored`.
//!
//! When `SEC_MS_GEC_VERSION` goes stale, Microsoft returns HTTP 403 on the
//! WebSocket handshake and this test fails, triggering a CI alert.

use kothok_edge_tts::{init_tls, EdgeTts, Engine, TtsEvent};
use std::time::Duration;

/// How long to wait for a full synthesis round-trip before declaring the
/// server unreachable.
const SYNTH_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
#[ignore = "live network test - run with: cargo test --test token_rotation -- --ignored"]
async fn sec_ms_gec_version_still_valid() {
    init_tls();

    let result = tokio::time::timeout(
        SYNTH_TIMEOUT,
        EdgeTts.synthesize(
            "Hello world.",
            "en-US-EmmaMultilingualNeural",
            "+0%",
            "en-US",
        ),
    )
    .await;

    let events = match result {
        Ok(Ok(events)) => events,
        Ok(Err(e)) => panic!(
            "SEC_MS_GEC_VERSION is likely STALE.\n\
             \n\
             Synthesis failed: {e}\n\
             \n\
             Fix:\n\
             1. Check https://github.com/rany2/edge-tts/blob/master/src/edge_tts/constants.py\n\
                for the latest SEC_MS_GEC_VERSION value.\n\
             2. Update it in src/auth.rs.\n\
             3. Bump crate version and `cargo publish`."
        ),
        Err(_) => panic!("synthesis timed out after {SYNTH_TIMEOUT:?}, server unreachable"),
    };

    let has_audio = events.iter().any(|e| matches!(e, TtsEvent::Audio(_)));
    assert!(
        has_audio,
        "synthesis returned no audio, SEC_MS_GEC_VERSION may be stale"
    );
}
