# Amplifi

Offline-first Windows image upscaler built with Tauri, React, and Rust.

## Run

From `amplifi-app/`:

```powershell
npm ci
npm run tauri dev
```

Build the standalone Windows app and installer with:

```powershell
npm run tauri build
```

## License

GNU AGPL-3.0. The bundled engine, runtime, and model files have additional license terms; see `amplifi-app/src-tauri/resources/licenses/`.
