//! Edge TTS voice catalogue — fetch the full list of available neural voices
//! from the Microsoft endpoint, with offline fallback tables and disk cache.
//!
//! [`list_voices`] performs a single HTTPS GET against the Edge voice-list
//! endpoint and returns every voice Microsoft currently offers (~400 voices,
//! ~140 locales).  [`voices_for_lang`] filters by locale, using dynamic voices
//! if available or a hardcoded fallback otherwise.

use crate::auth::{self, SEC_MS_GEC_VERSION, TRUSTED_CLIENT_TOKEN};
use crate::error::TtsError;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::OnceLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const VOICE_HOST: &str = "speech.platform.bing.com";
const VOICE_PATH: &str = "/consumer/speech/synthesize/readaloud/voices/list";
const HTTPS_PORT: u16 = 443;

pub const DEFAULT_VOICE_EN: &str = "en-US-EmmaMultilingualNeural";
pub const DEFAULT_VOICE_BN: &str = "bn-BD-NabanitaNeural";

/// One voice entry from the Edge voice-list response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VoiceInfo {
    #[serde(rename = "ShortName")]
    pub short_name: String,
    #[serde(rename = "Locale")]
    pub locale: String,
    #[serde(rename = "Gender")]
    pub gender: String,
    #[serde(rename = "FriendlyName")]
    pub friendly_name: String,
}

/// A voice with a short ID and a human-readable label for UI display.
#[derive(Clone, Debug)]
pub struct VoiceEntry {
    pub id: &'static str,
    pub label: &'static str,
}

static DYNAMIC_VOICES: OnceLock<Vec<VoiceInfo>> = OnceLock::new();

const VOICES_EN: &[VoiceEntry] = &[
    VoiceEntry {
        id: DEFAULT_VOICE_EN,
        label: "Emma (English)",
    },
    VoiceEntry {
        id: "en-US-AndrewMultilingualNeural",
        label: "Andrew (English)",
    },
    VoiceEntry {
        id: "en-US-AvaMultilingualNeural",
        label: "Ava (English)",
    },
];

const VOICES_BN: &[VoiceEntry] = &[
    VoiceEntry {
        id: DEFAULT_VOICE_BN,
        label: "Nabanita (Bengali)",
    },
    VoiceEntry {
        id: "bn-BD-PradeepNeural",
        label: "Pradeep (Bengali)",
    },
];

const VOICES_AR: &[VoiceEntry] = &[
    VoiceEntry {
        id: "ar-SA-HamedNeural",
        label: "Hamed (Arabic)",
    },
    VoiceEntry {
        id: "ar-SA-ZariyahNeural",
        label: "Zariyah (Arabic)",
    },
];

const VOICES_HI: &[VoiceEntry] = &[
    VoiceEntry {
        id: "hi-IN-SwaraNeural",
        label: "Swara (Hindi)",
    },
    VoiceEntry {
        id: "hi-IN-MadhurNeural",
        label: "Madhur (Hindi)",
    },
];

const VOICES_JA: &[VoiceEntry] = &[
    VoiceEntry {
        id: "ja-JP-NanamiNeural",
        label: "Nanami (Japanese)",
    },
    VoiceEntry {
        id: "ja-JP-KeitaNeural",
        label: "Keita (Japanese)",
    },
];

const VOICES_TH: &[VoiceEntry] = &[
    VoiceEntry {
        id: "th-TH-PremwadeeNeural",
        label: "Premwadee (Thai)",
    },
    VoiceEntry {
        id: "th-TH-NiwatNeural",
        label: "Niwat (Thai)",
    },
];

pub fn normalize_lang(lang: &str) -> &'static str {
    let lower = lang.to_lowercase();
    let prefix = lower.split('-').next().unwrap_or(&lower);
    match prefix {
        "bn" => "bn-BD",
        "hi" => "hi-IN",
        "mr" => "mr-IN",
        "ar" => "ar-SA",
        "ur" => "ur-PK",
        "ja" => "ja-JP",
        "zh" => "zh-CN",
        "ko" => "ko-KR",
        "th" => "th-TH",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "es" => "es-ES",
        "it" => "it-IT",
        "pt" => "pt-BR",
        "ru" => "ru-RU",
        "tr" => "tr-TR",
        "id" => "id-ID",
        "vi" => "vi-VN",
        _ => "en-US",
    }
}

fn fallback_voices_for_lang(normalized: &str) -> &'static [VoiceEntry] {
    match normalized {
        "bn-BD" => VOICES_BN,
        "ar-SA" => VOICES_AR,
        "hi-IN" => VOICES_HI,
        "ja-JP" => VOICES_JA,
        "th-TH" => VOICES_TH,
        _ => VOICES_EN,
    }
}

pub fn voices_for_lang(lang: &str) -> Cow<'static, [VoiceEntry]> {
    let normalized = normalize_lang(lang);
    let prefix = normalized.split('-').next().unwrap_or(&normalized);

    if let Some(dynamic) = DYNAMIC_VOICES.get() {
        let matching: Vec<VoiceEntry> = dynamic
            .iter()
            .filter(|v| v.locale.starts_with(prefix))
            .map(|v| VoiceEntry {
                id: Box::leak(v.short_name.clone().into_boxed_str()),
                label: Box::leak(format_voice_label(v).into_boxed_str()),
            })
            .collect();
        if !matching.is_empty() {
            return Cow::Owned(matching);
        }
    }

    Cow::Borrowed(fallback_voices_for_lang(&normalized))
}

pub fn voice_label(voice: &str) -> String {
    if let Some(dynamic) = DYNAMIC_VOICES.get() {
        if let Some(v) = dynamic.iter().find(|v| v.short_name == voice) {
            return format_voice_label(v);
        }
    }

    for e in VOICES_EN
        .iter()
        .chain(VOICES_BN)
        .chain(VOICES_AR)
        .chain(VOICES_HI)
        .chain(VOICES_JA)
        .chain(VOICES_TH)
    {
        if e.id == voice {
            return e.label.to_string();
        }
    }
    "Default voice".to_string()
}

fn format_voice_label(v: &VoiceInfo) -> String {
    let name = v
        .short_name
        .splitn(3, '-')
        .nth(2)
        .and_then(|s| s.strip_suffix("Neural"))
        .unwrap_or("Voice");
    format!("{name} ({})", v.locale)
}

pub fn set_dynamic_voices(voices: Vec<VoiceInfo>) {
    let _ = DYNAMIC_VOICES.set(voices);
}

pub fn load_voice_cache(path: &str) -> Vec<VoiceInfo> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_voice_cache(path: &str, voices: &[VoiceInfo]) {
    if let Ok(json) = serde_json::to_string(voices) {
        let _ = std::fs::write(path, json);
    }
}

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
                let _ = tx.send(voices);
            }
            Err(e) => log::warn!("voice list fetch failed: {e}"),
        }
    });
    rx
}

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

async fn https_get(host: &str, path: &str) -> Result<String, TtsError> {
    let connector = tls_connector()?;

    let dns_name = rustls::pki_types::ServerName::try_from(host.to_owned())
        .map_err(|e| TtsError::Connect(format!("invalid DNS name: {e}")))?;

    let tcp = TcpStream::connect((host, HTTPS_PORT))
        .await
        .map_err(|e| TtsError::Io(e))?;

    let mut tls = connector.connect(dns_name, tcp).await?;

    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Connection: close\r\n\
         User-Agent: Mozilla/5.0\r\n\
         \r\n"
    );
    tls.write_all(request.as_bytes()).await?;

    let mut raw = Vec::with_capacity(64 * 1024);
    tls.read_to_end(&mut raw).await?;

    let response = String::from_utf8_lossy(&raw);
    let body_start = response
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .ok_or_else(|| TtsError::Connect("malformed HTTP response".into()))?;

    Ok(response[body_start..].to_string())
}

fn tls_connector() -> Result<TlsConnector, TtsError> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    Ok(TlsConnector::from(std::sync::Arc::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_voice_info() {
        let json = r#"{
            "Name": "Microsoft Server Speech Text to Speech Voice (en-US, EmmaMultilingual)",
            "ShortName": "en-US-EmmaMultilingualNeural",
            "Locale": "en-US",
            "Gender": "Female",
            "FriendlyName": "Microsoft Emma Multilingual Online (Natural)",
            "Status": "GA"
        }"#;
        let v: VoiceInfo = serde_json::from_str(json).unwrap();
        assert_eq!(v.short_name, "en-US-EmmaMultilingualNeural");
        assert_eq!(v.locale, "en-US");
        assert_eq!(v.gender, "Female");
    }

    #[test]
    fn deserialize_voice_array() {
        let json = r#"[
            {"ShortName":"en-US-EmmaMultilingualNeural","Locale":"en-US","Gender":"Female","FriendlyName":"Emma"},
            {"ShortName":"bn-BD-NabanitaNeural","Locale":"bn-BD","Gender":"Female","FriendlyName":"Nabanita"}
        ]"#;
        let voices: Vec<VoiceInfo> = serde_json::from_str(json).unwrap();
        assert_eq!(voices.len(), 2);
        assert_eq!(voices[1].locale, "bn-BD");
    }

    #[test]
    fn deserialize_voice_with_extra_fields() {
        let json = r#"{
            "ShortName":"ja-JP-NanamiNeural",
            "Locale":"ja-JP",
            "Gender":"Female",
            "FriendlyName":"Nanami",
            "VoiceTag":{"ContentCategories":["General"],"VoicePersonalities":["Friendly"]}
        }"#;
        let v: VoiceInfo = serde_json::from_str(json).unwrap();
        assert_eq!(v.short_name, "ja-JP-NanamiNeural");
    }
}
