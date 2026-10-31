# Upscayl Parity Audit and Compatibility Notes

Baseline: `susenayw/upscayl` at `a00d55fee90e0f9435d5eaa86e76700df8199af8` (2.15.0).

## Parity Checklist

- [x] Select one image and display its original preview on Windows.
- [x] Single-image upscale and slider comparison: 32×32 PNG to 128×128 on the NVIDIA GPU.
- [x] Double upscale: 32×32 PNG to 512×512 through the interface and `JobRunner` engine integration test.
- [x] Batch folder: two images processed through the interface, including a transparent PNG; alpha, output dimensions, `Amplifi outputs` subfolder isolation, and extension-preserving filenames verified.
- [x] Protect existing outputs by default; temporary-file no-clobber and `overwrite: true` atomic replacement tested end-to-end with the engine.
- [x] Core model arguments, native model filename scale detection (`2x`, `3x`, `4x`), paths with spaces, EXIF/ICC copying, and 4× engine invocation tested.
- [x] Drag/drop inspection (`inspect_drop`) and clipboard image paste staging (`stage_clipboard_image` + end-to-end engine upscale): verified in automated integration tests; interactive Windows UI drag/paste remains optional for manual QA.
- [x] Custom model folder discovery (`model_list`), 1×–16× scale, custom width, GPU selection (`-g`), tile size (`-t`), TTA (`-x`), and compression (`-c`): verified end-to-end through `JobRunner` against `upscayl-bin.exe` with output dimension checks (`image_dimensions`).
- [x] PNG, JPEG, and WebP engine outputs, EXIF/ICC copying across formats, progress parsing (`parse_progress`), mid-run cancellation (`JobRunner::cancel`), and error paths (invalid GPU ID, malformed files, missing/unwritable output folder) verified in automated engine integration tests.
- [x] Lens comparison, zoom, input/target/output dimensions (`targetDimensions`, `comparisonClipPath`), cache-busting preview reload on overwrite, clear image button, collapsible sidebar, copy log button, remember output folder toggle, and reveal-output action implemented and unit-tested.
- [x] Upstream services decision: Amplifi is intentionally offline-first. Usage statistics (`total`, `batch`, `double`, `lastDurationMs`) are stored locally only; upstream PostHog/Firebase telemetry, cloud upsell modals, and phone-home auto-updaters are omitted by design.
- [x] Attribution and license review completed for all bundled binaries and models in `ATTRIBUTION.md` (`upscayl-bin.exe` under AGPL-3.0, `vcomp140.dll` under Microsoft VC++ Redistributable terms, `upscayl-standard-4x` / `upscayl-lite-4x` / `digital-art-4x` under BSD-3-Clause, `high-fidelity-4x` under CC BY 4.0, and `remacri-4x` / `ultrasharp-4x` / `ultramix-balanced-4x` under non-commercial CC BY-NC-SA 4.0).
- [x] Local photo-versus-drawing recommendation system implemented (`recommend_model_from_rgba` in `amplifi-core` and `recommend_model` Tauri command in UI).
- [ ] Remaining non-goals / partial items:
  - Full metadata parity beyond EXIF/ICC (XMP/IPTC are not copied, and copied EXIF blocks can retain original pixel dimension tags).
  - Multi-language localization and DaisyUI decorative themes beyond Dark/Light mode.

Upstream passes `noImageProcessing` as a payload field but has no user-facing setting or processing implementation for it in this baseline; it is not a separate parity feature.

## Compatibility Limitations and Error Handling

1. **Vulkan GPU requirement and invalid GPU IDs:**
   - `upscayl-bin.exe` requires a Vulkan-capable GPU. Older GPUs without Vulkan support (such as NVIDIA GeForce GTX 7xx / GT 920M or older integrated GPUs) cannot run NCNN Vulkan shaders.
   - On multi-GPU Windows laptops (e.g., `[0 NVIDIA GeForce RTX 3050 Laptop GPU]` and `[1 AMD Radeon(TM) Graphics]`), GPU IDs are zero-indexed in the order reported by Vulkan in the processing log. Passing an out-of-range GPU ID (such as `99`) causes `upscayl-bin.exe` to print `🚨 Error: Invalid GPU Device` and exit with `-1`; `JobRunner` captures this stderr line and surfaces it directly in the error message.
   - Windows Graphics Settings ("High performance" mode) may be needed on hybrid laptops if the OS overrides Vulkan device selection.
2. **Malformed or corrupted input files:**
   - Pre-flight check (`verify_input_image`): Rejects non-image files or unsupported headers before spawning `upscayl-bin.exe`.
   - Post-engine check (`verify_image_file` + `image_dimensions` + stderr capture): `upscayl-bin.exe` exits with code `0` even when `stb_image`/`libwebp` fails to decode a truncated or corrupted image body (printing `🚨 Error: Couldn't read the image '...'!`). `JobRunner` verifies both the output image header/dimensions and stderr error lines, ensuring corrupted inputs fail cleanly and leave no partial files behind.
3. **Output folder and permission errors:**
   - `upscayl-bin.exe` has an upstream Windows path bug when asked to create missing parent directories (`📂 Creating directory: C`) and still exits `0`. Amplifi avoids this completely by verifying that `output_dir` exists and creating a temporary staging file (`.amplifi-*`) in `output_dir` before invoking `upscayl-bin.exe`. Read-only or missing output directories fail immediately before GPU execution.

## Model Evaluation and Local Photo-vs-Drawing Recommendation

### Local Recommendation System
- `amplifi-core::recommend_model_from_rgba` analyzes a downsampled `64 × 64` RGBA preview of the selected image completely locally in Rust using four signal metrics:
  1. Alpha transparency ratio (`transparent_ratio`).
  2. Quantized 12-bit RGB palette concentration (`top16_coverage`).
  3. Flat adjacent pixel-pair ratio (`diff <= 2`).
  4. Bimodal sharp-edge ratio (`diff >= 40`) versus natural micro-texture gradient ratio (`4 <= diff <= 24`).
- Images exhibiting flat color fills, concentrated palettes, crisp line transitions, or transparency are classified as **drawing** and recommended `digital-art-4x` (`Digital Art`).
- Images exhibiting continuous tonal gradients and surface micro-texture are classified as **photo** and recommended `upscayl-standard-4x` (`Upscayl Standard`).
- Both default recommendations prioritize commercial-permissive models (`BSD-3-Clause`) over `CC BY-NC-SA 4.0` models.

### Evaluation of Newer NCNN/Vulkan Models
- **Engine constraints:** The bundled `upscayl-bin.exe` (NCNN Vulkan) supports ESRGAN (`RRDBNet` with `input.1`/`data` input and `output` blob) and Real-ESRGAN compact (`SRVGGNetCompact`) architectures. Transformer-based super-resolution architectures (`SwinIR`, `HAT`, `DAT`, `SRFormer`) and `SPAN` / `RealPLKSR` require PyTorch/ONNX/TensorRT or custom NCNN operators not compiled into `upscayl-bin.exe`.
- **Evaluated NCNN-compatible candidates:**
  - `realesr-animevideov3` (`SRVGGNetCompact`, BSD-3-Clause): Already represented by `upscayl-lite-4x`; fastest execution on integrated and entry-level GPUs (~2–3× faster than 23-block RRDBNet).
  - `realesrgan-x4plus-anime` (6-block `RRDBNet`, BSD-3-Clause): Bundled as `digital-art-4x`; optimal speed/quality trade-off for drawings, line art, and illustrations (8.9 MB vs 33.4 MB for 23-block models).
  - `4x-HFA2k` / `high-fidelity-4x` (23-block `RRDBNet`, CC BY 4.0 by Helaman): High detail retention on both clean artwork and sharp photographs while remaining commercial-permissive with attribution.
  - Community `Compact` (`2x`/`4x` `SRVGGNetCompact`) models on OpenModelDB: Can be loaded directly via Amplifi's **Custom model folder** feature (`native_scale` automatically detects `2x`, `3x`, or `4x` from the model filename).
