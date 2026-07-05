#![doc = include_str!("../README.md")]

mod auth;
mod connection;
mod edge_tts;
mod error;
mod event;
mod protocol;
mod ssml;
mod tts;
mod voice_list;

pub use edge_tts::EdgeTts;
pub use error::TtsError;
pub use event::TtsEvent;
pub use tts::Engine;
pub use voice_list::{
    list_voices, load_voice_cache, normalize_lang, save_voice_cache, set_dynamic_voices,
    spawn_voice_fetch, voice_label, voices_for_lang, VoiceEntry, VoiceInfo, DEFAULT_VOICE_BN,
    DEFAULT_VOICE_EN,
};

/// Install rustls's `ring` crypto provider.
///
/// rustls 0.23 requires an explicit crypto provider before any TLS handshake.
/// Call this **once** at startup, before the first [`EdgeTts`] connect.
/// Idempotent — safe to call multiple times.
pub fn init_tls() {
    // best-effort: idempotent — the provider may already be installed by the host
    let _ = rustls::crypto::ring::default_provider().install_default();
}
