//! Construction of arguments and safe execution for Amplifi's NCNN image engine.
//! The engine is invoked without a shell so filenames are passed as individual arguments.
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpscaleJob {
    pub input: PathBuf,
    pub output: PathBuf,
    pub models_dir: PathBuf,
    pub model: String,
    pub scale: u8,
    pub format: String,
    pub gpu_id: Option<String>,
    pub custom_width: Option<u32>,
    pub compression: u8,
    pub tile_size: Option<u32>,
    pub tta: bool,
    #[serde(default)]
    pub double_upscale: bool,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub copy_metadata: bool,
}

impl UpscaleJob {
    pub fn validate(&self) -> Result<(), String> {
        if !self.input.is_file() {
            return Err("Input image does not exist or is not a file".into());
        }
        verify_input_image(&self.input)?;
        if !self.models_dir.is_dir() {
            return Err("Models folder does not exist".into());
        }
        if self.model.is_empty()
            || self.model.contains(['/', '\\'])
            || self.model == "."
            || self.model == ".."
        {
            return Err("Invalid model name".into());
        }
        if !self.models_dir.join(format!("{}.param", self.model)).is_file()
            || !self.models_dir.join(format!("{}.bin", self.model)).is_file()
        {
            return Err("Model files (.param and .bin) are missing".into());
        }
        if !matches!(self.scale, 1..=16) {
            return Err("Scale must be between 1 and 16".into());
        }
        if !matches!(self.format.as_str(), "png" | "jpg" | "webp") {
            return Err("Unsupported output format".into());
        }
        if self.input == self.output
            || (self.output.exists()
                && std::fs::canonicalize(&self.input).ok()
                    == std::fs::canonicalize(&self.output).ok())
        {
            return Err("Output must not be the input file".into());
        }
        if self.custom_width == Some(0) {
            return Err("Width must be greater than zero".into());
        }
        if self.tile_size.is_some_and(|n| n < 32) {
            return Err("Tile size must be at least 32 pixels".into());
        }
        if self.compression > 100 {
            return Err("Compression must be between 0 and 100".into());
        }
        if self.gpu_id.as_ref().is_some_and(|id| {
            !id.is_empty()
                && !id
                    .split(',')
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        }) {
            return Err("GPU ID must contain numbers separated by commas".into());
        }
        Ok(())
    }

    pub fn arguments(&self) -> Vec<std::ffi::OsString> {
        let mut a: Vec<std::ffi::OsString> = vec![
            "-i".into(),
            self.input.as_os_str().into(),
            "-o".into(),
            self.output.as_os_str().into(),
            "-m".into(),
            self.models_dir.as_os_str().into(),
            "-n".into(),
            self.model.clone().into(),
            "-f".into(),
            self.format.clone().into(),
            "-c".into(),
            self.compression.to_string().into(),
        ];
        if let Some(width) = self.custom_width {
            a.extend(["-w".into(), width.to_string().into()]);
        } else if native_scale(&self.model) != self.scale {
            a.extend(["-s".into(), self.scale.to_string().into()]);
        }
        if let Some(ref gpu) = self.gpu_id {
            if !gpu.is_empty() {
                a.extend(["-g".into(), gpu.into()]);
            }
        }
        if let Some(tile) = self.tile_size {
            a.extend(["-t".into(), tile.to_string().into()]);
        }
        if self.tta {
            a.push("-x".into());
        }
        a
    }
}

pub fn native_scale(model: &str) -> u8 {
    let lower = model.to_ascii_lowercase();
    if lower.contains("2x") || lower.contains("x2") {
        2
    } else if lower.contains("3x") || lower.contains("x3") {
        3
    } else {
        4
    }
}

/// Copy EXIF and ICC without recompressing image pixels. Other metadata is not copied.
pub fn copy_exif_and_icc(input: &Path, output: &Path) -> Result<(), String> {
    use img_parts::{DynImage, ImageEXIF, ImageICC};
    let source = DynImage::from_bytes(std::fs::read(input).map_err(|e| e.to_string())?.into())
        .map_err(|e| e.to_string())?
        .ok_or("Unsupported source image")?;
    let exif = source.exif();
    let icc = source.icc_profile();
    if exif.is_none() && icc.is_none() {
        return Ok(());
    }
    if output
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg"))
        && exif.as_ref().is_some_and(|v| v.len() > 65527)
    {
        return Err("EXIF data is too large to fit in a JPEG output".into());
    }
    let mut target = DynImage::from_bytes(std::fs::read(output).map_err(|e| e.to_string())?.into())
        .map_err(|e| e.to_string())?
        .ok_or("Unsupported result image")?;
    if exif.is_some() {
        target.set_exif(exif);
    }
    if icc.is_some() {
        target.set_icc_profile(icc);
    }
    let mut encoded = Vec::new();
    target
        .encoder()
        .write_to(&mut encoded)
        .map_err(|e| e.to_string())?;
    std::fs::write(output, encoded).map_err(|e| e.to_string())
}

pub fn batch_output_folder(root: &Path) -> Result<PathBuf, String> {
    if !root.is_dir() {
        return Err("Output folder does not exist.".into());
    }
    let output = root.join("Amplifi outputs");
    std::fs::create_dir_all(&output)
        .map_err(|e| format!("Could not create the batch output folder: {e}"))?;
    Ok(output)
}

pub fn is_supported_image_path(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp"))
            .unwrap_or(false)
}

pub fn inspect_drop(path: &Path) -> Result<&'static str, String> {
    if path.is_dir() {
        return Ok("folder");
    }
    if is_supported_image_path(path) {
        Ok("image")
    } else {
        Err("Drop a PNG, JPEG, WebP image or a folder of images.".into())
    }
}

pub fn list_images(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut images = Vec::new();
    for item in std::fs::read_dir(folder).map_err(|e| e.to_string())? {
        let path = item.map_err(|e| e.to_string())?.path();
        if is_supported_image_path(&path) {
            images.push(path);
        }
    }
    images.sort();
    Ok(images)
}

pub fn model_list(folder: &Path) -> Result<Vec<String>, String> {
    let mut models = Vec::new();
    for item in std::fs::read_dir(folder).map_err(|e| e.to_string())? {
        let path = item.map_err(|e| e.to_string())?.path();
        if path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("param"))
            && path.with_extension("bin").is_file()
        {
            if let Some(name) = path.file_stem().and_then(|x| x.to_str()) {
                models.push(name.to_owned());
            }
        }
    }
    models.sort();
    Ok(models)
}

pub fn clipboard_image_extension(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.is_empty() || bytes.len() > 32 * 1024 * 1024 {
        return Err("Clipboard image must be 32 MiB or smaller.".into());
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Ok("jpg")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Ok("webp")
    } else {
        Err("Clipboard data is not a supported image.".into())
    }
}

pub fn stage_clipboard_image(
    cache_dir: &Path,
    bytes: &[u8],
) -> Result<tempfile::TempPath, String> {
    let extension = clipboard_image_extension(bytes)?;
    std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let mut file = tempfile::Builder::new()
        .prefix("amplifi-paste-")
        .suffix(&format!(".{extension}"))
        .tempfile_in(cache_dir)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.flush())
        .map_err(|e| e.to_string())?;
    Ok(file.into_temp_path())
}

pub fn verify_input_image(path: &Path) -> Result<(), String> {
    let mut header = [0u8; 12];
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let n = file.read(&mut header).map_err(|e| e.to_string())?;
    let valid = (n >= 8 && header.starts_with(b"\x89PNG\r\n\x1a\n"))
        || (n >= 3 && header.starts_with(b"\xff\xd8\xff"))
        || (n >= 12 && header.starts_with(b"RIFF") && &header[8..12] == b"WEBP");
    if valid {
        Ok(())
    } else {
        Err("Input file is not a valid PNG, JPEG, or WebP image.".into())
    }
}

pub fn image_dimensions(path: &Path) -> Result<(u32, u32), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    image_dimensions_from_bytes(&bytes)
}

pub fn image_dimensions_from_bytes(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
            return Err("Malformed PNG header.".into());
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width == 0 || height == 0 {
            return Err("PNG has zero dimensions.".into());
        }
        return Ok((width, height));
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        let mut i = 2;
        while i + 3 < bytes.len() {
            if bytes[i] != 0xFF {
                i += 1;
                continue;
            }
            while i < bytes.len() && bytes[i] == 0xFF {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }
            let marker = bytes[i];
            i += 1;
            if marker == 0xD8
                || marker == 0xD9
                || (0xD0..=0xD7).contains(&marker)
                || marker == 0x01
            {
                continue;
            }
            if i + 2 > bytes.len() {
                break;
            }
            let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
            if len < 2 || i + len > bytes.len() {
                break;
            }
            if matches!(
                marker,
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
            ) && len >= 7
            {
                let height = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]) as u32;
                let width = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                if width > 0 && height > 0 {
                    return Ok((width, height));
                }
            }
            i += len;
        }
        return Err("Malformed JPEG header.".into());
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") && bytes.len() >= 25 {
        match &bytes[12..16] {
            b"VP8 " if bytes.len() >= 30 && &bytes[23..26] == b"\x9d\x01\x2a" => {
                let width = (u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF) as u32;
                let height = (u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF) as u32;
                if width > 0 && height > 0 {
                    return Ok((width, height));
                }
            }
            b"VP8L" if bytes.len() >= 25 && bytes[20] == 0x2F => {
                let bits = u32::from_le_bytes([bytes[21], bytes[22], bytes[23], bytes[24]]);
                let width = (bits & 0x3FFF) + 1;
                let height = ((bits >> 14) & 0x3FFF) + 1;
                return Ok((width, height));
            }
            b"VP8X" if bytes.len() >= 30 => {
                let width = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]) + 1;
                let height = u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]) + 1;
                return Ok((width, height));
            }
            _ => {}
        }
        return Err("Malformed WebP header.".into());
    }
    Err("Unsupported or corrupted image file.".into())
}

pub fn verify_image_file(path: &Path, format: &str) -> Result<(), String> {
    let mut header = [0u8; 12];
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let n = file.read(&mut header).map_err(|e| e.to_string())?;
    let valid = match format {
        "png" => n >= 8 && header.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" => n >= 3 && header.starts_with(b"\xff\xd8\xff"),
        "webp" => n >= 12 && header.starts_with(b"RIFF") && &header[8..12] == b"WEBP",
        _ => false,
    };
    if !valid || image_dimensions(path).is_err() {
        return Err("The engine did not produce a valid image; check the processing log.".into());
    }
    Ok(())
}

pub fn parse_progress(line: &str) -> Option<f32> {
    let trimmed = line.trim().strip_suffix('%').unwrap_or(line.trim());
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    trimmed.parse::<f32>().ok().map(|v| v.clamp(0.0, 100.0))
}

pub fn engine_available(path: &Path) -> bool {
    path.is_file()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelRecommendation {
    pub category: String,
    pub recommended_model: String,
    pub reason: String,
}

/// Classify a downsampled RGBA image buffer locally as `"drawing"` or `"photo"`
/// and recommend an appropriate model from `available_models`.
pub fn recommend_model_from_rgba(
    width: u32,
    height: u32,
    rgba: &[u8],
    available_models: &[String],
) -> Result<ModelRecommendation, String> {
    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or("Invalid image dimensions for recommendation.")?;
    if width == 0 || height == 0 || rgba.len() != expected_len {
        return Err("RGBA buffer length does not match width and height.".into());
    }
    let total_pixels = (width * height) as f32;
    let mut transparent_pixels = 0u32;
    let mut bins = vec![0u32; 4096];
    for px in rgba.chunks_exact(4) {
        if px[3] < 245 {
            transparent_pixels += 1;
        }
        let idx = (((px[0] >> 4) as usize) << 8)
            | (((px[1] >> 4) as usize) << 4)
            | ((px[2] >> 4) as usize);
        bins[idx] += 1;
    }
    bins.sort_unstable_by(|a, b| b.cmp(a));
    let top16_sum: u32 = bins.iter().take(16).sum();
    let top16_coverage = top16_sum as f32 / total_pixels;
    let transparent_ratio = transparent_pixels as f32 / total_pixels;

    let mut total_pairs = 0u32;
    let mut flat_pairs = 0u32;
    let mut texture_pairs = 0u32;
    let mut sharp_edge_pairs = 0u32;
    let w = width as usize;
    let h = height as usize;
    let mut inspect_pair = |a: &[u8], b: &[u8]| {
        total_pairs += 1;
        let diff = a[0]
            .abs_diff(b[0])
            .max(a[1].abs_diff(b[1]))
            .max(a[2].abs_diff(b[2]))
            .max(a[3].abs_diff(b[3]));
        if diff <= 2 {
            flat_pairs += 1;
        } else if (4..=24).contains(&diff) {
            texture_pairs += 1;
        } else if diff >= 40 {
            sharp_edge_pairs += 1;
        }
    };
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let cur = &rgba[idx..idx + 4];
            if x + 1 < w {
                let right = &rgba[idx + 4..idx + 8];
                inspect_pair(cur, right);
            }
            if y + 1 < h {
                let down_idx = ((y + 1) * w + x) * 4;
                let down = &rgba[down_idx..down_idx + 4];
                inspect_pair(cur, down);
            }
        }
    }
    let pairs_f = (total_pairs.max(1)) as f32;
    let flat_ratio = flat_pairs as f32 / pairs_f;
    let texture_ratio = texture_pairs as f32 / pairs_f;
    let edge_ratio = sharp_edge_pairs as f32 / pairs_f;

    let is_drawing = transparent_ratio >= 0.02
        || flat_ratio >= 0.35
        || (top16_coverage >= 0.58 && texture_ratio < 0.35)
        || (edge_ratio >= 0.06 && flat_ratio >= 0.22 && texture_ratio < 0.30);

    let pick_model = |preferred: &[&str]| -> String {
        for &candidate in preferred {
            if let Some(found) = available_models
                .iter()
                .find(|m| m.eq_ignore_ascii_case(candidate))
            {
                return found.clone();
            }
        }
        for &keyword in preferred {
            if let Some(found) = available_models
                .iter()
                .find(|m| m.to_ascii_lowercase().contains(keyword))
            {
                return found.clone();
            }
        }
        available_models
            .first()
            .cloned()
            .unwrap_or_else(|| preferred[0].to_string())
    };

    if is_drawing {
        let recommended_model = pick_model(&["digital-art-4x", "anime", "art", "amplifi-lite-4x"]);
        let reason = if transparent_ratio >= 0.02 {
            "Detected alpha transparency and graphic regions typical of illustrations or sprites."
        } else {
            "Detected flat color regions and crisp edges typical of drawings or digital art."
        };
        Ok(ModelRecommendation {
            category: "drawing".into(),
            recommended_model,
            reason: reason.into(),
        })
    } else {
        let recommended_model =
            pick_model(&["amplifi-standard-4x", "high-fidelity-4x", "standard"]);
        Ok(ModelRecommendation {
            category: "photo".into(),
            recommended_model,
            reason: "Detected continuous gradients and natural surface texture typical of real photographs.".into(),
        })
    }
}

#[derive(Default)]
pub struct JobRunner {
    child: Mutex<Option<Arc<Mutex<Child>>>>,
    busy: AtomicBool,
    cancelled: AtomicBool,
}

struct JobGuard<'a>(&'a JobRunner);
impl Drop for JobGuard<'_> {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}

impl JobRunner {
    pub fn cancel(&self) -> Result<(), String> {
        let current = self.child.lock().map_err(|e| e.to_string())?;
        if !self.busy.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(child) = current.as_ref() {
            let mut process = child.lock().map_err(|e| e.to_string())?;
            let _ = process.kill();
        }
        Ok(())
    }

    fn run_pass<F>(
        &self,
        executable: &Path,
        job: &UpscaleJob,
        on_log: &Arc<F>,
    ) -> Result<(), String>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let engine_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let (child, stderr_thread) = {
            let mut current = self.child.lock().map_err(|e| e.to_string())?;
            if self.cancelled.load(Ordering::SeqCst) {
                return Err("Upscaling cancelled.".into());
            }
            if current.is_some() {
                return Err("An upscaling job is already running.".into());
            }
            let mut command = Command::new(executable);
            command
                .args(job.arguments())
                .stdout(Stdio::null())
                .stderr(Stdio::piped());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            let mut child = command
                .spawn()
                .map_err(|e| format!("Could not start the upscaling engine: {e}"))?;
            let stderr_thread = child.stderr.take().map(|stderr| {
                let on_log = on_log.clone();
                let engine_error = engine_error.clone();
                std::thread::spawn(move || {
                    use std::io::BufReader;
                    let mut reader = BufReader::new(stderr);
                    let mut buffer = [0u8; 1024];
                    let mut line = Vec::new();
                    let record_line = |raw: &[u8]| {
                        let text = String::from_utf8_lossy(raw).trim().to_owned();
                        if text.is_empty() {
                            return;
                        }
                        let lower = text.to_ascii_lowercase();
                        if text.contains("Error")
                            || lower.contains("failed")
                            || lower.contains("invalid gpu")
                            || lower.contains("couldn't read")
                            || lower.contains("couldn't write")
                            || lower.contains("invalid tile")
                        {
                            if let Ok(mut slot) = engine_error.lock() {
                                *slot = Some(text.clone());
                            }
                        }
                        on_log(text);
                    };
                    while let Ok(count) = reader.read(&mut buffer) {
                        if count == 0 {
                            break;
                        }
                        for &byte in &buffer[..count] {
                            if byte == b'\r' || byte == b'\n' {
                                if !line.is_empty() {
                                    record_line(&line);
                                    line.clear();
                                }
                            } else if line.len() < 8192 {
                                line.push(byte);
                            }
                        }
                    }
                    if !line.is_empty() {
                        record_line(&line);
                    }
                })
            });
            let child = Arc::new(Mutex::new(child));
            *current = Some(child.clone());
            (child, stderr_thread)
        };

        let outcome = loop {
            if self.cancelled.load(Ordering::SeqCst) {
                if let Ok(mut process) = child.lock() {
                    let _ = process.kill();
                    let _ = process.wait();
                }
                break Err("Upscaling cancelled.".into());
            }
            let status = match child.lock() {
                Ok(mut process) => process.try_wait().map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            };
            match status {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(error) => break Err(error),
            }
        };
        *self.child.lock().map_err(|e| e.to_string())? = None;
        if let Some(handle) = stderr_thread {
            let _ = handle.join();
        }
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("Upscaling cancelled.".into());
        }
        let status = outcome?;
        let reported_error = engine_error.lock().ok().and_then(|mut g| g.take());
        if !status.success() {
            if let Some(err) = reported_error {
                return Err(format!("Upscaling failed: {err}"));
            }
            return Err(format!(
                "Upscaling stopped or failed ({status}). Check the log and GPU compatibility."
            ));
        }
        if let Err(verify_err) = verify_image_file(&job.output, &job.format) {
            if let Some(err) = reported_error {
                return Err(format!("Upscaling failed: {err}"));
            }
            return Err(verify_err);
        }
        Ok(())
    }

    pub fn run_job<F>(&self, executable: &Path, job: &UpscaleJob, on_log: F) -> Result<PathBuf, String>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        job.validate()?;
        {
            let _lock = self.child.lock().map_err(|e| e.to_string())?;
            if self
                .busy
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return Err("An upscaling job is already running.".into());
            }
            self.cancelled.store(false, Ordering::SeqCst);
        }
        let _guard = JobGuard(self);
        if !engine_available(executable) {
            return Err("The upscaling engine was not found.".into());
        }
        if job.output.exists() && !job.overwrite {
            return Err(
                "The output already exists. Choose another folder or enable overwrite.".into(),
            );
        }
        let output_dir = job
            .output
            .parent()
            .filter(|p| p.is_dir())
            .ok_or("Output folder does not exist.")?;
        let result_file = tempfile::Builder::new()
            .prefix(".amplifi-")
            .suffix(&format!(".{}", job.format))
            .tempfile_in(output_dir)
            .map_err(|e| format!("Cannot create a file in the output folder: {e}"))?
            .into_temp_path();
        let on_log = Arc::new(on_log);
        let mut pass = job.clone();
        pass.output = result_file.to_path_buf();
        if job.double_upscale {
            let intermediate = tempfile::Builder::new()
                .prefix(".amplifi-pass-")
                .suffix(".png")
                .tempfile_in(output_dir)
                .map_err(|e| format!("Cannot create a file in the output folder: {e}"))?
                .into_temp_path();
            pass.output = intermediate.to_path_buf();
            pass.format = "png".into();
            pass.custom_width = None;
            on_log("Pass 1 of 2".into());
            self.run_pass(executable, &pass, &on_log)?;
            pass.input = intermediate.to_path_buf();
            pass.output = result_file.to_path_buf();
            pass.format = job.format.clone();
            pass.custom_width = job.custom_width;
            on_log("Pass 2 of 2".into());
            self.run_pass(executable, &pass, &on_log)?;
        } else {
            self.run_pass(executable, &pass, &on_log)?;
        }
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("Upscaling cancelled.".into());
        }
        verify_image_file(&result_file, &job.format)?;
        if job.copy_metadata {
            copy_exif_and_icc(&job.input, &result_file)
                .map_err(|e| format!("Could not copy EXIF/ICC metadata: {e}"))?;
        }
        if job.overwrite {
            result_file
                .persist(&job.output)
                .map_err(|e| format!("Could not save the result: {}", e.error))?;
        } else {
            result_file
                .persist_noclobber(&job.output)
                .map_err(|e| format!("Could not save the result: {}", e.error))?;
        }
        Ok(job.output.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_clipboard_image_signatures_and_limits() {
        assert_eq!(clipboard_image_extension(b"\x89PNG\r\n\x1a\n").unwrap(), "png");
        assert_eq!(clipboard_image_extension(b"\xff\xd8\xff").unwrap(), "jpg");
        assert_eq!(clipboard_image_extension(b"RIFF\0\0\0\0WEBP").unwrap(), "webp");
        assert!(clipboard_image_extension(b"not an image").is_err());
        assert!(clipboard_image_extension(&vec![0; 32 * 1024 * 1024 + 1]).is_err());
    }
    #[test]
    fn detects_native_scale() {
        assert_eq!(native_scale("realesr-animevideov3-x2"), 2);
        assert_eq!(native_scale("3x-custom"), 3);
        assert_eq!(native_scale("4x_model"), 4);
        assert_eq!(native_scale("ordinary"), 4);
    }
    #[test]
    fn paths_with_spaces_stay_single_arguments() {
        let job = UpscaleJob {
            input: "C:/Pictures/my image.png".into(),
            output: "C:/Pictures/out.png".into(),
            models_dir: "C:/My models".into(),
            model: "test-4x".into(),
            scale: 4,
            format: "png".into(),
            gpu_id: None,
            custom_width: None,
            compression: 0,
            tile_size: None,
            tta: false,
            double_upscale: false,
            overwrite: false,
            copy_metadata: false,
        };
        let args = job.arguments();
        assert_eq!(args[1], std::ffi::OsString::from("C:/Pictures/my image.png"));
        assert!(!args.contains(&std::ffi::OsString::from("-s")));
    }
    #[test]
    fn parses_progress_lines_and_ignores_log_messages() {
        assert_eq!(parse_progress("0.00%"), Some(0.0));
        assert_eq!(parse_progress("42.5%"), Some(42.5));
        assert_eq!(parse_progress("100.00"), Some(100.0));
        assert_eq!(parse_progress("Pass 1 of 2"), None);
        assert_eq!(parse_progress("[0 NVIDIA GeForce RTX 3050]"), None);
    }
    #[test]
    fn recommends_digital_art_for_drawings_and_standard_for_photos() {
        let models = vec![
            "digital-art-4x".to_string(),
            "high-fidelity-4x".to_string(),
            "amplifi-standard-4x".to_string(),
        ];
        // Synthetic drawing: flat blocks of color with sharp border
        let mut drawing = Vec::with_capacity(16 * 16 * 4);
        for y in 0..16u8 {
            for x in 0..16u8 {
                if x < 8 && y < 8 {
                    drawing.extend_from_slice(&[240, 60, 80, 255]);
                } else {
                    drawing.extend_from_slice(&[30, 40, 200, 255]);
                }
            }
        }
        let rec_draw = recommend_model_from_rgba(16, 16, &drawing, &models).unwrap();
        assert_eq!(rec_draw.category, "drawing");
        assert_eq!(rec_draw.recommended_model, "digital-art-4x");

        // Synthetic photograph: continuous gradients with natural micro-texture
        let mut photo = Vec::with_capacity(16 * 16 * 4);
        for y in 0..16u8 {
            for x in 0..16u8 {
                let r = x.wrapping_mul(11).wrapping_add(y.wrapping_mul(7));
                let g = x.wrapping_mul(13).wrapping_add(y.wrapping_mul(9)).wrapping_add(25);
                let b = x.wrapping_mul(9).wrapping_add(y.wrapping_mul(15)).wrapping_add(50);
                photo.extend_from_slice(&[r, g, b, 255]);
            }
        }
        let rec_photo = recommend_model_from_rgba(16, 16, &photo, &models).unwrap();
        assert_eq!(rec_photo.category, "photo");
        assert_eq!(rec_photo.recommended_model, "amplifi-standard-4x");
    }
}
