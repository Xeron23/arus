# Validation report — 27 October 2026

## Executed successfully

- Frontend TypeScript strict type check + Vite production bundle.
- Eight Vitest tests: transfer invariance, review/undo, dedup replay, composed search filters, integer/same-wallet validation, email escaping/category rules, delete/undo, reconciliation neutrality.
- Ten Rust integration tests using actual SQLite files: integer Rupiah, transfer conservation, validation/soft delete, two-layer dedup plus UIDVALIDITY reset, review/undo/reapproval, rollback preserves cursor, explicit reconciliation, text-only malformed HTML handling, App Password validation, absence of credential columns.
- Rust core compiled with async-imap 0.11.3, Tokio/rustls, rusqlite bundled SQLite, mailparse, html2text, serde.
- Browser UI: four-tab navigation, dark theme persistence, manual cash form, ledger update, and Undo. Browser HTTP compatibility fix replaces missing randomUUID with crypto.getRandomValues UUID fallback.
- Android manifest patcher tested against an XML fixture: INTERNET present, backup disabled, release cleartext blocked, debug override separate. This fixture is not a real Android build.
- Package versions locked in package-lock.json, src-tauri/Cargo.lock, src-tauri/core/Cargo.lock.

## Not verified / environment limits

- No Android SDK/NDK, emulator or USB-connected device was available. No APK is attached and Android runtime/build/signing have not been verified.
- Parent Tauri host cargo check reached platform dependency compilation and stopped because pkg-config/GTK/WebKit development dependencies are absent. This is not a successful full Tauri application compilation; the successfully compiled/tested crate is arus-core.
- No user App Password was supplied. Successful authentication or real-bank sync was not performed. `test_imap_connection` contains the actual TCP/TLS/auth implementation; use the native UI or imap_check example to verify with your own account locally.
- Bank parser fixtures are synthetic. They are not proof of complete coverage of live Indonesian bank email layouts.

Use `npm run android:init`, `npm run android:dev`, and the acceptance procedure in README to complete device verification. Do not treat this report as a production security audit.

