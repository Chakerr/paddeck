# PadDeck

Free, open source "Stream Deck" that turns **any USB numpad** into a control surface for OBS and system shortcuts. Target user: non-technical streamers, teachers, podcasters. Full roadmap: `PLAN.md`.

## Communication
- Talk to the maintainer (Luis) in **Spanish**. Be direct, no preambles; when there are options, recommend one and say why.
- Code, identifiers, code comments and commit messages in **English** (international community). User-facing strings go through i18n (`es`, `en`).

## Principles (check every change against these)
1. Simple first: if a feature needs a long explanation, redesign it.
2. Zero technical setup for end users: no drivers by default, no config files to edit, no manual ports.
3. Any numpad: detection by "press a key", never by model lists.
4. Keys from the registered pad must never leak to other apps while the app runs.
5. Readable open source code: small modules, clear names, minimal comments that explain *why*.
6. Secure by default: secrets in the OS keychain, no telemetry, never log keystrokes from other keyboards.

## Stack
Tauri 2 · Rust (tokio) · React + TypeScript + Vite · `windows` crate (Raw Input + WH_KEYBOARD_LL) · `obws` (obs-websocket v5) · `enigo` · `keyring` · `serde` JSON. MVP targets Windows 10/11; input capture stays behind the `InputBackend` trait for future Linux/macOS backends.

## Current phase
**Phase 0 — input capture spike** in `spikes/input-windows` (standalone console crate). Do NOT scaffold the Tauri app until the acceptance criteria in `PLAN.md` §5 are met and recorded in `docs/input-windows.md`.

## Repository layout
- `PLAN.md` — roadmap, architecture, data model, acceptance criteria (source of truth).
- `spikes/` — throwaway experiments; findings get documented before code moves to `src-tauri/`.
- `docs/decisions/` — ADRs (`NNNN-title.md`). Write one for every non-trivial technical choice.
- `docs/input-windows.md` — capture research notes and test results.
- `src-tauri/`, `src/` — created in Phase 1 following `PLAN.md` §4.

## Commands
```
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p input-windows          # spike (from spikes/input-windows)
```
UI (Phase 2+): `npm run dev`, `npm run tauri dev`, `npm run lint`.

## Conventions
- Rust: `rustfmt` defaults, zero clippy warnings, `thiserror` for library errors, `anyhow` only in binaries/spikes. No `unwrap()` outside tests/spikes.
- All Win32 `unsafe` lives in small, documented functions inside `input/windows.rs`; each `unsafe` block gets a `// SAFETY:` note.
- OS-independent logic (mapping engine, profiles, actions) must have unit tests.
- TypeScript strict mode; no `any`.
- Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`). Small, focused commits.
- One branch per task; PR into `main`.

## Working rules for Claude Code
- Read `PLAN.md` before starting a phase. Work in small steps and stop at the end of each step with a short summary and what Luis must test.
- Keyboard capture cannot be verified automatically: when a change affects capture, write the manual test steps and wait for Luis's results before continuing.
- Prefer the simplest solution that satisfies the acceptance criteria. Don't add dependencies without saying why.
- Never commit secrets. The obs-websocket password goes to `keyring` only.
- When a decision changes `PLAN.md`, update `PLAN.md` and add an ADR in the same commit.
