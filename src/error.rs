// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Nayeem Bin Ahsan
//! Crate-level error type for Edge-TTS synthesis failures.

use thiserror::Error;

/// All ways a synthesis request can fail.
///
/// Callers typically propagate this with `?` or match on a variant to
/// decide whether to retry (e.g. [`TtsError::Connect`] after a network blip).
#[derive(Debug, Error)]
pub enum TtsError {
    /// The WebSocket handshake or DRM-token auth failed after all retry
    /// attempts. The inner string is the last underlying error message.
    #[error("ws connect failed after retries: {0}")]
    Connect(String),

    /// Transport-level WebSocket error (handshake, frame decode, TLS).
    #[error("ws: {0}")]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),

    /// I/O error on the underlying TCP stream, including receive idle timeout.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// The turn completed (or the stream closed) without any audio frames.
    #[error("no audio received")]
    NoAudio,
}
