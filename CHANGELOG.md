# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-29

### Added
- add archive/unarchive, reference validation and UX improvements
- Implement R1-R20: mandatory reference, archive/unarchive, duplicate warning,
  load-error state with retry, isSubmitting guard, showArchived toggle
- Rename factory methods: update_from->with_id, from_storage->restore (B1)
- Store fetches all assets (active+archived); AssetTable filters by showArchived
- Add tests: validateAsset (R9), presenter (R11); fix no-op R9 test in useAddAsset

## [0.1.0] - 2026-03-29

### Added

- Initial release — portfolio management desktop app (Tauri 2 + React 19 + Rust)
- Asset CRUD with categories, currency, risk level, and asset class
- Category management with system-protected default category
- Dashboard scaffold
- i18n support (fr / en)
