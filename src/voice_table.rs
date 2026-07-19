// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Nayeem Bin Ahsan
//! Static offline voice tables, language normalization, and label formatting.
//!
//! Fallback voice lists used when the dynamic voice catalogue is unavailable.
//! [`normalize_lang`] maps partial or full language tags to their default
//! BCP-47 locale; [`voices_for_lang`] returns matching voices from the dynamic
//! catalogue or the hardcoded fallback.

use crate::voice_types::{VoiceEntry, VoiceInfo, DYNAMIC_VOICES};
use std::borrow::Cow;

/// Default English voice short-name.
pub const DEFAULT_VOICE_EN: &str = "en-US-EmmaMultilingualNeural";

/// Default Bengali voice short-name.
pub const DEFAULT_VOICE_BN: &str = "bn-BD-NabanitaNeural";

const VOICES_EN: &[VoiceEntry] = &[
    VoiceEntry::new("en-US-EmmaMultilingualNeural", "Emma (US)"),
    VoiceEntry::new("en-US-AndrewMultilingualNeural", "Andrew (US)"),
    VoiceEntry::new("en-US-AvaMultilingualNeural", "Ava (US)"),
    VoiceEntry::new("en-US-BrianNeural", "Brian (US)"),
    VoiceEntry::new("en-US-JennyNeural", "Jenny (US)"),
    VoiceEntry::new("en-US-GuyNeural", "Guy (US)"),
    VoiceEntry::new("en-GB-SoniaNeural", "Sonia (UK)"),
    VoiceEntry::new("en-GB-RyanNeural", "Ryan (UK)"),
    VoiceEntry::new("en-GB-LibbyNeural", "Libby (UK)"),
    VoiceEntry::new("en-AU-NatashaNeural", "Natasha (AU)"),
    VoiceEntry::new("en-AU-WilliamNeural", "William (AU)"),
    VoiceEntry::new("en-IN-NeerjaNeural", "Neerja (IN)"),
    VoiceEntry::new("en-CA-ClaraNeural", "Clara (CA)"),
    VoiceEntry::new("en-IE-EmilyNeural", "Emily (IE)"),
];

const VOICES_BN: &[VoiceEntry] = &[
    VoiceEntry::new("bn-BD-NabanitaNeural", "Nabanita (BD)"),
    VoiceEntry::new("bn-BD-PradeepNeural", "Pradeep (BD)"),
    VoiceEntry::new("bn-IN-TanishaaNeural", "Tanishaa (IN)"),
    VoiceEntry::new("bn-IN-BashkarNeural", "Bashkar (IN)"),
];

const VOICES_AR: &[VoiceEntry] = &[
    VoiceEntry::new("ar-SA-HamedNeural", "Hamed (SA)"),
    VoiceEntry::new("ar-SA-ZariyahNeural", "Zariyah (SA)"),
    VoiceEntry::new("ar-EG-SalmaNeural", "Salma (EG)"),
    VoiceEntry::new("ar-EG-ShakirNeural", "Shakir (EG)"),
];

const VOICES_HI: &[VoiceEntry] = &[
    VoiceEntry::new("hi-IN-SwaraNeural", "Swara (IN)"),
    VoiceEntry::new("hi-IN-MadhurNeural", "Madhur (IN)"),
];

const VOICES_JA: &[VoiceEntry] = &[
    VoiceEntry::new("ja-JP-NanamiNeural", "Nanami (JP)"),
    VoiceEntry::new("ja-JP-KeitaNeural", "Keita (JP)"),
];

const VOICES_TH: &[VoiceEntry] = &[
    VoiceEntry::new("th-TH-PremwadeeNeural", "Premwadee (TH)"),
    VoiceEntry::new("th-TH-NiwatNeural", "Niwat (TH)"),
];

const ALL_FALLBACK: &[&[VoiceEntry]] = &[
    VOICES_EN, VOICES_BN, VOICES_AR, VOICES_HI, VOICES_JA, VOICES_TH,
];

/// Map a language tag (full BCP-47 or just a prefix like `"bn"`) to its
/// default locale. Unknown prefixes fall back to `"en-US"`.
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

/// Return voices matching `lang` from the dynamic catalogue, or from the
/// hardcoded fallback tables if no dynamic voices are loaded.
pub fn voices_for_lang(lang: &str) -> Cow<'static, [VoiceEntry]> {
    let normalized = normalize_lang(lang);
    let prefix = normalized.split('-').next().unwrap_or(normalized);

    if let Some(dynamic) = DYNAMIC_VOICES.get() {
        let matching: Vec<VoiceEntry> = dynamic
            .iter()
            .filter(|v| v.locale().starts_with(prefix))
            .map(|v| {
                VoiceEntry::new(
                    Box::leak(v.short_name().to_owned().into_boxed_str()),
                    Box::leak(format_voice_label(v).into_boxed_str()),
                )
            })
            .collect();
        if !matching.is_empty() {
            return Cow::Owned(matching);
        }
    }

    Cow::Borrowed(fallback_voices_for_lang(normalized))
}

/// Look up a human-readable label for a voice by its short-name.
pub fn voice_label(voice: &str) -> String {
    if let Some(dynamic) = DYNAMIC_VOICES.get() {
        if let Some(v) = dynamic.iter().find(|v| v.short_name() == voice) {
            return format_voice_label(v);
        }
    }

    for table in ALL_FALLBACK {
        if let Some(e) = table.iter().find(|e| e.id() == voice) {
            return e.label().to_string();
        }
    }
    "Default voice".to_string()
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

pub(crate) fn format_voice_label(v: &VoiceInfo) -> String {
    let name = v
        .short_name()
        .splitn(3, '-')
        .nth(2)
        .map(|s| s.strip_suffix("Neural").unwrap_or(s))
        .map(|s| s.strip_suffix("Multilingual").unwrap_or(s))
        .unwrap_or("Voice");
    format!("{name} ({})", v.locale())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_known_prefixes() {
        assert_eq!(normalize_lang("bn"), "bn-BD");
        assert_eq!(normalize_lang("ar"), "ar-SA");
        assert_eq!(normalize_lang("ja"), "ja-JP");
        assert_eq!(normalize_lang("en"), "en-US");
    }

    #[test]
    fn normalize_full_tag_passthrough() {
        assert_eq!(normalize_lang("bn-BD"), "bn-BD");
        assert_eq!(normalize_lang("hi-IN"), "hi-IN");
    }

    #[test]
    fn normalize_unknown_falls_back_to_en() {
        assert_eq!(normalize_lang("zz"), "en-US");
        assert_eq!(normalize_lang(""), "en-US");
    }

    #[test]
    fn normalize_case_insensitive() {
        assert_eq!(normalize_lang("BN"), "bn-BD");
        assert_eq!(normalize_lang("Ja-JP"), "ja-JP");
    }

    #[test]
    fn fallback_voices_returns_tables() {
        assert!(!fallback_voices_for_lang("bn-BD").is_empty());
        assert!(!fallback_voices_for_lang("en-US").is_empty());
        assert!(!fallback_voices_for_lang("ar-SA").is_empty());
    }

    #[test]
    fn fallback_voices_unknown_lang_returns_en() {
        let table = fallback_voices_for_lang("zz-ZZ");
        assert_eq!(table, VOICES_EN);
    }

    #[test]
    fn format_label_extracts_name() {
        let v = VoiceInfo::new(
            "en-US-EmmaMultilingualNeural".into(),
            "en-US".into(),
            "Female".into(),
            "Emma".into(),
        );
        assert_eq!(format_voice_label(&v), "Emma (en-US)");
    }

    #[test]
    fn format_label_strips_neural_suffix() {
        let v = VoiceInfo::new(
            "ja-JP-NanamiNeural".into(),
            "ja-JP".into(),
            "Female".into(),
            "Nanami".into(),
        );
        assert_eq!(format_voice_label(&v), "Nanami (ja-JP)");
    }

    #[test]
    fn format_label_unknown_format() {
        let v = VoiceInfo::new("x-Y-Z".into(), "x-Y".into(), "Male".into(), "Z".into());
        assert_eq!(format_voice_label(&v), "Z (x-Y)");
    }

    #[test]
    fn voice_label_finds_fallback_entry() {
        assert_eq!(voice_label("en-US-EmmaMultilingualNeural"), "Emma (US)");
        assert_eq!(voice_label("bn-BD-NabanitaNeural"), "Nabanita (BD)");
    }

    #[test]
    fn voice_label_unknown_returns_default() {
        assert_eq!(voice_label("nonexistent-Voice"), "Default voice");
    }

    #[test]
    fn default_voice_consts() {
        assert!(DEFAULT_VOICE_EN.starts_with("en-US-"));
        assert!(DEFAULT_VOICE_BN.starts_with("bn-BD-"));
    }
}
