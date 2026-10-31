# Amplifi Handoff

Last updated: 2026-10-31

## Project goal

Rewrite Upscayl as **Amplifi**, a Windows-first desktop image upscaler implemented with Rust/Tauri and a web-style UI. Keep the existing NCNN/Vulkan engine initially. Reach parity with the fork's user-facing features, document component/model licensing and compatibility constraints, and provide local photo-versus-drawing model recommendations.

## Important owner instructions

- Minimize computer/UI use; prefer repository inspection and small local code changes. The user said Luna can be used for QA.
- **Do not push any code to GitHub** unless the user explicitly says to.
- Repository: `https://github.com/susenayw/Amplifi` (separate, public, empty repository; not a fork). Local work is in `D:\Agent Dependencies\Codex\Apps\Amplifi`.
- User requested a Rust rewrite, with React/TypeScript inside Tauri for the UI and Rust (`amplifi-core` + `src-tauri`) for orchestration and core logic.

## Current structure and baseline

- `amplifi-app/`: Tauri v2 + React/TypeScript application.
- `amplifi-app/src-tauri/src/lib.rs`: Thin Tauri command layer delegating execution, staging, cancellation, model discovery, drop inspection, and model recommendations to `amplifi-core`.
- `amplifi-app/amplifi-core/`: Rust engine orchestration (`JobRunner`), argument builder (`UpscaleJob`), image header/dimension verification (`image_dimensions`, `verify_input_image`, `verify_image_file`), EXIF/ICC metadata copying (`copy_exif_and_icc`), clipboard staging (`stage_clipboard_image`), drop/folder/model scanning, and local photo-vs-drawing model recommendations (`recommend_model_from_rgba`).
- `amplifi-app/src/App.tsx` and `amplifi-app/src/naming.ts`: Main UI, comparison viewer (slider and lens), target/output dimension calculation, cache-busting preview reload on overwrite, local photo-vs-drawing recommendation prompt, collapsible sidebar, log copy, and local usage statistics.
- `PARITY.md`: Parity checklist, compatibility limitations, and model evaluation notes.
- `PRODUCT.md`: Product constraints.
- `ATTRIBUTION.md`, `LICENSE`, `Real-ESRGAN_LICENSE.txt`: Complete attribution and license audit for all bundled binaries and models.
- Reference upstream is `susenayw/upscayl` commit `a00d55fee90e0f9435d5eaa86e76700df8199af8` (2.15.0); local sparse reference checkout is `D:\Agent Dependencies\Codex\Apps\Amplifi-source`.
- Bundled Windows NCNN/Vulkan executable, seven upstream model pairs, and license files are under `amplifi-app/src-tauri/resources/`.

## Implemented and verified

- Amplifi product branding in Tauri config (`com.susenayw.amplifi`) and app UI.
- Single image selection, preview, original/target/output dimensions, and original/result comparison (slider and lens with adjustable lens size and zoom), plus reveal output in folder and clear selection.
- Collapsible sidebar, copyable processing log, remember-output-folder option, and local-only usage statistics (omitting upstream PostHog/Firebase telemetry, cloud upsells, and phone-home auto-updaters).
- Folder batch processing into `Amplifi outputs` with extension-preserving filenames to avoid collisions and re-ingestion.
- Model selection, scale 1–16, custom width, double upscale, GPU ID, tile size, TTA, compression, PNG/JPEG/WebP, custom model folder selection, and built-in model restore.
- Safe temp-file staging (`.amplifi-*`), overwrite protection by default, atomic replacement when overwrite is enabled, and cache-busting preview refresh.
- Pre-flight input signature check plus post-engine output verification and stderr error capture for invalid GPU IDs, malformed/corrupted images, and unwritable output paths.
- Mid-run job cancellation (`JobRunner::cancel`) that terminates the active `upscayl-bin.exe` process, cleans up temporary staging files, and reports cancellation cleanly.
- Local photo-versus-drawing recommendation system (`recommend_model_from_rgba` in `amplifi-core` and `recommend_model` Tauri command), recommending `digital-art-4x` for drawings/illustrations and `upscayl-standard-4x` for photographs.
- Complete component and model license audit in `ATTRIBUTION.md` (identifying `upscayl-standard-4x`, `upscayl-lite-4x`, `digital-art-4x` as BSD-3-Clause; `high-fidelity-4x` as CC BY 4.0 by Helaman; `remacri-4x`, `ultrasharp-4x`, `ultramix-balanced-4x` as non-commercial CC BY-NC-SA 4.0; `upscayl-bin.exe` as AGPL-3.0; and `vcomp140.dll` as Microsoft VC++ OpenMP runtime).

## Verification summary

- Rust core unit + GPU engine integration test suite passes **16 tests** (5 unit + 11 integration):
  `cargo test --manifest-path amplifi-core/Cargo.toml -- --include-ignored --test-threads=1`
- Frontend unit test suite passes **4 test groups** (`naming.test.ts`):
  `npm test`
- `npm run build` and `cargo check --manifest-path src-tauri/Cargo.toml` pass cleanly.
- Debug Windows NSIS installer rebuilt and verified with updated binary, models, and `ATTRIBUTION.md`.

## Git status / publishing

The local Git repo has `origin` set to `https://github.com/susenayw/Amplifi.git`. `.gitignore` excludes `**/target/` and `**/target-package/`. **Never run `git push` without the user's explicit instruction.**
