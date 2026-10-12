# AGENTS.md

Shutter Hub — Tauri 2 desktop app for local photo asset management. pnpm workspace, React 19 + TS + Vite frontend, Rust backend. The root `README.md` (Chinese) is the product spec; code comments are Chinese too — match surrounding style.

## Layout

- `apps/desktop/` — the only real package: `@shutter-hub/desktop`
- `packages/` — reserved, empty (listed in pnpm workspace)
- Frontend: `apps/desktop/src` — `lib/api.ts` wraps every IPC `invoke`, `lib/store.ts` is the Zustand store
- Global keyboard shortcuts live in `App.tsx` `onKey`, which early-returns while the viewer is open — viewer-only keys (arrows, rating/flag auto-advance, Z zoom toggle) are handled inside `components/Viewer.tsx`
- Rust: `apps/desktop/src-tauri/src` — `db.rs`, `indexer.rs`, `metadata.rs`, `thumbs.rs`, `preview.rs`, `video.rs`, `export.rs`, `collage.rs`, `develop.rs`, `share.rs`, `text.rs`, `ai.rs`, `xmp.rs`, `volumes.rs`, `commands/`

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
- Long operations (scan/ingest/hash/AI) emit progress events (`Emitter` backend + `listenEvent` in `api.ts`). Follow that pattern for new background work.
- ffmpeg/ffprobe are optional system binaries (`video.rs`, availability cached in `OnceLock`); video indexing, dedupe and rating must work without them — thumbnails/duration just stay empty.
- AI (`ai.rs`): models download on demand to `<app_data_dir>/models` (~180MB); features must degrade gracefully when files are missing. ort `Session`s are not `Sync` and are deliberately loaded per call inside `spawn_blocking` (no caching). All AI jobs share one `AppState.ai_busy` single-flight lock.
- Export/collage (`export.rs`, `collage.rs`) reuse the preview pipeline (`preview::ensure`) for decodable full-size input — don't decode originals directly. Filenames go through `export::unique_path` (collision-safe). Live Photo pairing (same-dir same-stem image+video) happens in `indexer::detect_live_pairs`; paired motion clips are intentionally NOT indexed as standalone videos.
- Develop (`develop.rs`): adjustments stored as JSON in `photos.edit` + XMP `crs` fields (bidirectional with Lightroom on rescan). Viewer previews render capped at 2560px and cache as `<id>_edit_<hash>.jpg`; export/share render at full resolution. Keep the pipeline order (exposure → WB → tone → contrast → saturation) consistent between preview and export.
- Text/watermark (`text.rs`): system fonts via fontdb (CJK-aware), rasterized with ab_glyph — glyph draw callbacks are relative to `px_bounds().min`, always add that offset. Missing fonts must degrade silently, never fail the export.

## Testing

- `tests/backend_integration.rs` generates its own JPEG fixtures in temp dirs; no external fixtures or services required
- A global `Mutex` serializes the integration tests (shared WAL DB / temp-dir interference) — don't remove it
- `ai::tests::ai_models_download_and_infer` is `#[ignore]`d (downloads ~180MB from HuggingFace); run manually with `cargo test -- --ignored` when touching the ONNX pipeline

## Misc

- Cargo `edition = "2024"` requires Rust ≥ 1.85; `ort` downloads ONNX Runtime binaries at build time (needs network on first build)
- Release: pushing a `v*` tag triggers `.github/workflows/release.yml` (3-platform tauri-action, draft release). Updater endpoint and `TAURI_SIGNING_PRIVATE_KEY*` secrets point at `Tatekii/shutter-hub`.
