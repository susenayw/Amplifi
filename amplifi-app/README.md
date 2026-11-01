# Amplifi

Amplifi is a Windows-first desktop app for enlarging images with local AI models. It uses a Tauri interface and a bundled NCNN/Vulkan engine, so image processing runs on your computer without uploading images to a cloud service.

![Amplifi desktop app](screenshots/Amplifi.png)

## How lightweight is it?

Amplifi uses Tauri for its desktop shell rather than packaging a full Chromium browser runtime as Electron apps commonly do. That can make the application shell and installed runtime footprint smaller than comparable Electron-based upscalers. Amplifi bundles its own engine and model weights, though, so the complete download is larger than the shell alone. No controlled size or memory benchmark is published here; actual memory use and speed vary with image dimensions, selected model, tile size, and GPU.

Upscaling itself is demanding work. Large images and high output scales can use substantial GPU memory and system memory, and may take a long time. Tiling can help with memory pressure, usually with a speed tradeoff. A compatible Vulkan GPU is recommended for the bundled NCNN/Vulkan engine; performance and compatibility depend on the device and drivers.

## Pros and tradeoffs

### Pros

- **Local processing:** images stay on your device during processing; no account or cloud upload is required.
- **Compact desktop shell:** Tauri avoids bundling a full Chromium runtime in the application shell.
- **Bundled workflow:** engine and model files are included with the Windows app, so there is no separate Python environment to set up.
- **Useful controls:** single-image and batch processing, model selection, output formats, optional tiling, and a before/after preview.

### Tradeoffs

- **Hardware dependent:** the NCNN/Vulkan engine needs a supported Vulkan device and suitable drivers for GPU acceleration. Compatibility varies, and CPU-only processing can be very slow or unavailable on some setups.
- **Large jobs need resources:** high-resolution inputs and outputs can consume considerable memory and processing time, even when the app shell is lightweight.
- **Bundled files add download size:** the engine and model weights increase the installed footprint beyond the Tauri UI itself.
- **Focused feature set:** dedicated upscalers may offer additional restoration tools, broader model libraries, cloud processing, or more polished batch-management features.
- **No benchmark claims:** there are no reproducible, same-hardware size, speed, or memory comparisons against other upscalers in this README.

## Build and run from source

Install Node.js and Rust, then install the frontend dependencies:

```sh
npm install
```

Run the desktop app in development mode:

```sh
npm run tauri dev
```

Build the frontend:

```sh
npm run build
```

Check the Tauri Rust shell:

```sh
cargo check --manifest-path src-tauri/Cargo.toml
```

Run core tests:

```sh
cargo test --manifest-path amplifi-core/Cargo.toml
```

Windows engine integration checks require the bundled engine resources and a compatible environment. The bundled resources and model weights live in `src-tauri/resources`.

## Project status

Amplifi is a work in progress. Review the notices in `src-tauri/resources/licenses/` before redistributing builds or bundled resources.
