use amplifi_core::{self as engine, JobRunner, UpscaleJob};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct ActiveJob(Arc<JobRunner>);

#[derive(Default)]
struct ClipboardImage(Mutex<Option<tempfile::TempPath>>);

#[tauri::command]
fn paste_image(
    app: tauri::AppHandle,
    state: State<'_, ClipboardImage>,
    request: tauri::ipc::Request<'_>,
) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("Expected raw image data.".into());
    };
    let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    let file = engine::stage_clipboard_image(&cache, bytes)?;
    let path = file.to_string_lossy().into_owned();
    app.asset_protocol_scope()
        .allow_file(&file)
        .map_err(|e| e.to_string())?;
    *state.0.lock().map_err(|e| e.to_string())? = Some(file);
    Ok(path)
}

#[tauri::command]
async fn upscale(
    app: tauri::AppHandle,
    state: State<'_, ActiveJob>,
    job: UpscaleJob,
) -> Result<String, String> {
    let executable = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("resources")
        .join("bin")
        .join("upscayl-bin.exe");
    let runner = state.0.clone();
    let saved = tauri::async_runtime::spawn_blocking(move || {
        runner.run_job(&executable, &job, move |line| {
            let _ = app.emit("upscale-log", line);
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(saved.to_string_lossy().into_owned())
}

#[tauri::command]
fn inspect_drop(path: String) -> Result<&'static str, String> {
    engine::inspect_drop(std::path::Path::new(&path))
}

#[tauri::command]
fn allow_preview(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    if !engine::is_supported_image_path(path) {
        return Err("The selected image is unavailable or unsupported.".into());
    }
    engine::verify_input_image(path)?;
    app.asset_protocol_scope()
        .allow_file(path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn batch_output_folder(folder: String) -> Result<String, String> {
    engine::batch_output_folder(std::path::Path::new(&folder))
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn list_images(folder: String) -> Result<Vec<String>, String> {
    engine::list_images(std::path::Path::new(&folder))
        .map(|items| items.iter().map(|p| p.to_string_lossy().into_owned()).collect())
}

#[tauri::command]
fn model_list(folder: String) -> Result<Vec<String>, String> {
    engine::model_list(std::path::Path::new(&folder))
}

#[tauri::command]
fn recommend_model(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    models: Vec<String>,
) -> Result<engine::ModelRecommendation, String> {
    engine::recommend_model_from_rgba(width, height, &rgba, &models)
}

#[tauri::command]
fn cancel(state: State<'_, ActiveJob>) -> Result<(), String> {
    state.0.cancel()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ActiveJob::default())
        .manage(ClipboardImage::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            upscale,
            cancel,
            model_list,
            recommend_model,
            list_images,
            allow_preview,
            inspect_drop,
            paste_image,
            batch_output_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running Amplifi");
}
