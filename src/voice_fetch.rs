// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Nayeem Bin Ahsan
//! HTTPS fetch of the Edge TTS voice catalogue.

use crate::auth::{self, SEC_MS_GEC_VERSION, TRUSTED_CLIENT_TOKEN};
use crate::error::TtsError;
use crate::voice_types::VoiceInfo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const VOICE_HOST: &str = "speech.platform.bing.com";
const VOICE_PATH: &str = "/consumer/speech/synthesize/readaloud/voices/list";
const HTTPS_PORT: u16 = 443;

const FETCH_USER_AGENT: &str = "Mozilla/5.0";
const FETCH_HOST_HEADER: &str = "Host";
const FETCH_CONNECTION_HEADER: &str = "Connection";
const FETCH_CLOSE_VALUE: &str = "close";
const FETCH_USER_AGENT_HEADER: &str = "User-Agent";

const FETCH_BUFFER_CAPACITY: usize = 64 * 1024;

/// Fetch the complete Edge-TTS voice catalogue.
///
/// Requires [`crate::init_tls()`] to have been called first (installs the
/// `ring` crypto provider).  Returns `Err` on any network or parse failure;
/// callers should fall back to a hardcoded list on error.
pub async fn list_voices() -> Result<Vec<VoiceInfo>, TtsError> {
    let gec = auth::sec_ms_gec(0);
    let path = format!(
        "{VOICE_PATH}?TrustedClientToken={TRUSTED_CLIENT_TOKEN}\
         &Sec-MS-GEC={gec}\
         &Sec-MS-GEC-Version={SEC_MS_GEC_VERSION}"
    );

    let body = https_get(VOICE_HOST, &path).await?;
    serde_json::from_str(&body)
        .map_err(|e| TtsError::Connect(format!("voice list JSON parse: {e}")))
}

/// Spawn a blocking thread that fetches the voice list and returns it
/// via a channel. The caller receives a `Receiver` that yields the
/// voices once the fetch completes, or drops on failure.
pub fn spawn_voice_fetch() -> std::sync::mpsc::Receiver<Vec<VoiceInfo>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                log::warn!("voice fetch: runtime creation failed: {e}");
                return;
            }
        };
        match rt.block_on(list_voices()) {
            Ok(voices) => {
                // best-effort: if the receiver was dropped, no-op
                let _ = tx.send(voices);
            }
            Err(e) => log::warn!("voice list fetch failed: {e}"),
        }
    });
    rx
}

async fn https_get(host: &str, path: &str) -> Result<String, TtsError> {
    let connector = tls_connector()?;

    let dns_name = rustls::pki_types::ServerName::try_from(host.to_owned())
        .map_err(|e| TtsError::Connect(format!("invalid DNS name: {e}")))?;

    let tcp = TcpStream::connect((host, HTTPS_PORT))
        .await
        .map_err(TtsError::Io)?;

    let mut tls = connector.connect(dns_name, tcp).await?;

    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         {FETCH_HOST_HEADER}: {host}\r\n\
         {FETCH_CONNECTION_HEADER}: {FETCH_CLOSE_VALUE}\r\n\
         {FETCH_USER_AGENT_HEADER}: {FETCH_USER_AGENT}\r\n\
         \r\n"
    );
    tls.write_all(request.as_bytes()).await?;

    let mut raw = Vec::with_capacity(FETCH_BUFFER_CAPACITY);
    tls.read_to_end(&mut raw).await?;

    let response = String::from_utf8_lossy(&raw);
    let body_start = response
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .ok_or_else(|| TtsError::Connect("malformed HTTP response".into()))?;

    Ok(response[body_start..].to_string())
}

#[allow(clippy::result_large_err)]
fn tls_connector() -> Result<TlsConnector, TtsError> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    Ok(TlsConnector::from(std::sync::Arc::new(config)))
}
