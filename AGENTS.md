# AGENTS.md

Shutter Hub — Tauri 2 desktop app for local photo asset management. pnpm workspace, React 19 + TS + Vite frontend, Rust backend. The root `README.md` (Chinese) is the product spec; code comments are Chinese too — match surrounding style.

## Layout

- `apps/desktop/` — the only real package: `@shutter-hub/desktop`
- `packages/` — reserved, empty (listed in pnpm workspace)
- Frontend: `apps/desktop/src` — `lib/api.ts` wraps every IPC `invoke`, `lib/store.ts` is the Zustand store
- Global keyboard shortcuts live in `App.tsx` `onKey`, which early-returns while the viewer is open — viewer-only keys (arrows, rating/flag auto-advance, Z zoom toggle) are handled inside `components/Viewer.tsx`
- Rust: `apps/desktop/src-tauri/src` — `db.rs`, `indexer.rs`, `metadata.rs`, `thumbs.rs`, `preview.rs`, `xmp.rs`, `volumes.rs`, `commands/`

## Commands

- `pnpm dev` runs **Vite only** (port 1420, `strictPort`) — not the desktop app. To run the app: `cd apps/desktop && pnpm tauri dev`
- `pnpm build` / `pnpm typecheck` are frontend-only (`tsc && vite build`); they don't touch Rust
- Root `pnpm check` is dead (desktop has no `check` script; exits 0 with a warning). CI order is `cargo test` → `typecheck` → `build`
- Rust: `cd apps/desktop/src-tauri && cargo test` (unit + integration) / `cargo check`
- Bundle: `cd apps/desktop && pnpm tauri build`
- No linter or formatter is configured anywhere (no eslint/prettier/rustfmt/clippy config)

## Backend conventions

- Every `#[tauri::command]` must be added to `generate_handler!` in `src-tauri/src/lib.rs` and wrapped in `src/lib/api.ts`, or the frontend `invoke` fails at runtime
- Commands are thin wrappers around `*_inner(state: &AppState)` functions taking db/cache paths; integration tests call `_inner` directly. Keep logic testable this way.
- DB: SQLite (WAL) at `<app_data_dir>/shutter_hub.db`. Schema = ordered SQL batches in `db.rs` `MIGRATIONS` (batch i = user_version i→i+1). Append new batches; never edit old ones.
- Filesystem is the source of truth; the DB is a rebuildable index. Ratings/colors/flags are also written to XMP sidecars (`xmp.rs`) — don't make metadata DB-only.
- Thumbnails: `<app_cache_dir>/thumbs/<photo_id>.jpg`; viewer previews: `<app_cache_dir>/previews/<photo_id>.jpg` (lazy-generated, reuse while source mtime unchanged). Both served via `convertFileSrc` (asset protocol); `tauri.conf.json` scopes it to `$APPCACHE/**` — serving new paths requires extending that scope.
- Long operations (scan/ingest/hash) emit progress events (`Emitter` backend + `listenEvent` in `api.ts`). Follow that pattern for new background work.

## Testing

- `tests/backend_integration.rs` generates its own JPEG fixtures in temp dirs; no external fixtures or services required
- A global `Mutex` serializes the integration tests (shared WAL DB / temp-dir interference) — don't remove it

## Misc

- Cargo `edition = "2024"` requires Rust ≥ 1.85 (README's "Rust ≥ 1.77" is stale)
- Release: pushing a `v*` tag triggers `.github/workflows/release.yml` (3-platform tauri-action, draft release). Updater endpoint and `TAURI_SIGNING_PRIVATE_KEY*` secrets point at `Tatekii/shutter-hub`.
