# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **403 on every synthesis since October 2026**: Microsoft upgraded the Edge
  Read-Aloud handshake to require **v2 `Sec-MS-GEC` tokens**; the legacy v1
  token is rejected regardless of `SEC_MS_GEC_VERSION`
  ([rany2/edge-tts#490](https://github.com/rany2/edge-tts/issues/490)).
  `sec_ms_gec()` now generates v2 tokens (`<16 hex>-<64 hex>`, CS64 +
  adjusted SHA-256, ported from edge-tts 7.3.0's MIT-licensed `drm.py`) and
  `SEC_MS_GEC_VERSION` became `2-143.0.3650.75`.

### Changed
- Removed the `sha2` dependency (the v2 adjusted digest is implemented in
  `src/token_v2.rs`).
- `tests/auth.rs` now asserts the v2 token format.

## [0.2.10] - 2026-07-XX

### Added
- Voice fetch and table updates
- Honest comment on hy/pa multilingual limitation

## [0.2.9] - 2026-0X-XX

### Added
- Rate helper module
- SPDX license headers

## [0.2.8] - 2026-0X-XX

### Fixed
- Replaced Unicode artifacts with ASCII throughout

## [0.2.7] - 2026-0X-XX

### Fixed
- Hinnant month formula correction
- Voice label formatting fixes
- Clippy warnings resolved

### Changed
- Updated dependencies

## [0.2.6] - 2026-0X-XX

### Added
- Refactored voice modules with encapsulated fields
- Named all constants
- UTF-8 safe string indexing

## [0.2.5] - 2026-0X-XX

### Added
- Initial release: Microsoft Edge online TTS client for Rust
