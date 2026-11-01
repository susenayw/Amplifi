# Amplifi development

Install Node dependencies with `npm install`. Run the development app with `npm run tauri dev`. Check the frontend with `npm run build`, the Tauri shell with `cargo check --manifest-path src-tauri/Cargo.toml`, and the core with `cargo test --manifest-path amplifi-core/Cargo.toml`. On Windows with a compatible GPU and bundled resources, run the ignored engine integration tests with `cargo test --manifest-path amplifi-core/Cargo.toml -- --include-ignored --test-threads=1`.

This is still a work in progress. Review `src-tauri/resources/licenses/` before distributing builds. The `src-tauri/resources` directory contains the Windows engine and model weights.
