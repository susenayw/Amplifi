# Amplifi development

Install Node dependencies with `npm install`. Run the development app with `npm run tauri dev` and check the Rust application with `cargo check --manifest-path src-tauri/Cargo.toml`. Run core tests with `cargo test --manifest-path amplifi-core/Cargo.toml` and, on Windows with a compatible GPU and bundled resources, include the engine tests with `cargo test --manifest-path amplifi-core/Cargo.toml -- --include-ignored --test-threads=1`.

This is still a work in progress. Review `../PARITY.md` and `../ATTRIBUTION.md` before distributing builds. The `src-tauri/resources` directory contains the upstream Windows engine and model weights.
