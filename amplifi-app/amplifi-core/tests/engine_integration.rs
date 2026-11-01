use amplifi_core::{JobRunner, UpscaleJob};
use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};

#[test]
#[ignore = "requires the downloaded Windows NCNN engine and a compatible GPU"]
fn engine_writes_to_open_temporary_file() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
    let directory = tempfile::tempdir().unwrap();
    let output = tempfile::Builder::new()
        .suffix(".png")
        .tempfile_in(directory.path())
        .unwrap();
    let job = UpscaleJob {
        input: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.png"),
        output: output.path().into(),
        models_dir: root.join("models"),
        model: "amplifi-lite-4x".into(),
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
    job.validate().unwrap();
    let result = Command::new(root.join("bin/amplifi-engine.exe"))
        .args(job.arguments())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(amplifi_core::image_dimensions(output.path()).unwrap(), (128, 128));
}

#[test]
fn temporary_results_never_replace_existing_output_without_permission() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("result.png");
    std::fs::write(&destination, b"old image").unwrap();
    let staging = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
    std::fs::write(staging.path(), b"new image").unwrap();
    assert!(staging.persist_noclobber(&destination).is_err());
    assert_eq!(std::fs::read(&destination).unwrap(), b"old image");
    let staging = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
    std::fs::write(staging.path(), b"new image").unwrap();
    staging.persist(&destination).unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), b"new image");
}

#[test]
#[ignore = "requires the downloaded Windows NCNN engine and a compatible GPU"]
fn two_passes_produce_a_sixteen_fold_image() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("double.png");
    let runner = JobRunner::default();
    let logs = Arc::new(Mutex::new(Vec::new()));
    let logs_clone = logs.clone();
    let job = UpscaleJob {
        input: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.png"),
        output: destination.clone(),
        models_dir: root.join("models"),
        model: "amplifi-lite-4x".into(),
        scale: 4,
        format: "png".into(),
        gpu_id: None,
        custom_width: None,
        compression: 0,
        tile_size: None,
        tta: false,
        double_upscale: true,
        overwrite: false,
        copy_metadata: false,
    };
    let saved = runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, move |line| {
            logs_clone.lock().unwrap().push(line);
        })
        .unwrap();
    assert_eq!(saved, destination);
    assert_eq!(amplifi_core::image_dimensions(&destination).unwrap(), (512, 512));
    let recorded = logs.lock().unwrap();
    assert!(recorded.iter().any(|l| l == "Pass 1 of 2"));
    assert!(recorded.iter().any(|l| l == "Pass 2 of 2"));
    assert!(recorded.iter().any(|l| amplifi_core::parse_progress(l).is_some()));
}

#[test]
fn copies_exif_and_icc_without_reencoding_image_data() {
    use img_parts::{DynImage, ImageEXIF, ImageICC};
    let directory = tempfile::tempdir().unwrap();
    let original =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.png"))
            .unwrap();
    let mut source = DynImage::from_bytes(original.clone().into())
        .unwrap()
        .unwrap();
    let exif = b"II\x2a\0\x08\0\0\0\0\0\0\0\0\0".to_vec();
    let icc = b"test icc profile".to_vec();
    source.set_exif(Some(exif.clone().into()));
    source.set_icc_profile(Some(icc.clone().into()));
    let source_path = directory.path().join("source.png");
    let target_path = directory.path().join("target.png");
    let mut encoded = Vec::new();
    source.encoder().write_to(&mut encoded).unwrap();
    std::fs::write(&source_path, encoded).unwrap();
    std::fs::write(&target_path, original).unwrap();
    amplifi_core::copy_exif_and_icc(&source_path, &target_path).unwrap();
    let output = DynImage::from_bytes(std::fs::read(&target_path).unwrap().into())
        .unwrap()
        .unwrap();
    assert_eq!(output.exif().unwrap().as_ref(), exif.as_slice());
    assert_eq!(output.icc_profile().unwrap().as_ref(), icc.as_slice());
}

#[test]
fn copies_exif_and_icc_across_supported_output_formats() {
    use img_parts::{DynImage, ImageEXIF, ImageICC};
    let dir = tempfile::tempdir().unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let bytes = std::fs::read(fixtures.join("sample.png")).unwrap();
    let mut src = DynImage::from_bytes(bytes.into()).unwrap().unwrap();
    let exif = b"II\x2a\0\x08\0\0\0\0\0\0\0\0\0".to_vec();
    let icc = b"test icc profile".to_vec();
    src.set_exif(Some(exif.clone().into()));
    src.set_icc_profile(Some(icc.clone().into()));
    let source = dir.path().join("input.png");
    let mut data = Vec::new();
    src.encoder().write_to(&mut data).unwrap();
    std::fs::write(&source, data).unwrap();
    for ext in ["png", "jpg", "webp"] {
        let output = dir.path().join(format!("out.{ext}"));
        std::fs::copy(fixtures.join(format!("sample.{ext}")), &output).unwrap();
        amplifi_core::copy_exif_and_icc(&source, &output).unwrap();
        let result = DynImage::from_bytes(std::fs::read(&output).unwrap().into())
            .unwrap()
            .unwrap();
        assert_eq!(result.exif().unwrap().as_ref(), exif.as_slice(), "{ext}");
        assert_eq!(
            result.icc_profile().unwrap().as_ref(),
            icc.as_slice(),
            "{ext}"
        );
        assert_eq!(amplifi_core::image_dimensions(&output).unwrap(), (32, 32), "{ext}");
    }
}

#[test]
fn batch_scan_does_not_reprocess_prior_output_folder() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("photo.png"), b"input").unwrap();
    std::fs::write(root.path().join("photo.jpg"), b"input").unwrap();
    let output = amplifi_core::batch_output_folder(root.path()).unwrap();
    std::fs::write(output.join("photo.png_amplifi_4x_model.png"), b"previous output").unwrap();
    let first = amplifi_core::list_images(root.path()).unwrap();
    let second = amplifi_core::list_images(root.path()).unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first, second);
    assert!(first.iter().all(|p| p.parent() == Some(root.path())));
}

#[test]
fn inspect_drop_and_custom_model_list_work_for_valid_and_invalid_inputs() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    assert_eq!(amplifi_core::inspect_drop(&fixtures).unwrap(), "folder");
    assert_eq!(
        amplifi_core::inspect_drop(&fixtures.join("sample.png")).unwrap(),
        "image"
    );
    assert_eq!(
        amplifi_core::inspect_drop(&fixtures.join("sample.jpg")).unwrap(),
        "image"
    );
    assert_eq!(
        amplifi_core::inspect_drop(&fixtures.join("sample.webp")).unwrap(),
        "image"
    );

    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("notes.txt");
    std::fs::write(&txt, b"hello").unwrap();
    assert!(amplifi_core::inspect_drop(&txt).is_err());
    assert!(amplifi_core::inspect_drop(&dir.path().join("missing.png")).is_err());

    std::fs::write(dir.path().join("alpha-4x.param"), b"p").unwrap();
    std::fs::write(dir.path().join("alpha-4x.bin"), b"b").unwrap();
    std::fs::write(dir.path().join("orphan.param"), b"p").unwrap();
    std::fs::write(dir.path().join("only-bin.bin"), b"b").unwrap();
    assert_eq!(
        amplifi_core::model_list(dir.path()).unwrap(),
        vec!["alpha-4x".to_string()]
    );
}

#[test]
#[ignore = "requires the downloaded Windows NCNN engine and a compatible GPU"]
fn clipboard_staging_and_overwrite_semantics_work_end_to_end() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let png_bytes = std::fs::read(fixtures.join("sample.png")).unwrap();
    let pasted = amplifi_core::stage_clipboard_image(&cache, &png_bytes).unwrap();
    assert!(pasted.extension().is_some_and(|e| e == "png"));

    let output = dir.path().join("pasted_out.png");
    let runner = JobRunner::default();
    let mut job = UpscaleJob {
        input: pasted.to_path_buf(),
        output: output.clone(),
        models_dir: root.join("models"),
        model: "amplifi-lite-4x".into(),
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
    runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, |_| {})
        .unwrap();
    assert_eq!(amplifi_core::image_dimensions(&output).unwrap(), (128, 128));

    // Without overwrite enabled, existing output is protected.
    let err = runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, |_| {})
        .unwrap_err();
    assert!(err.contains("already exists"), "{err}");

    // With overwrite enabled and scale 2, existing output is replaced with 64x64.
    job.overwrite = true;
    job.scale = 2;
    runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, |_| {})
        .unwrap();
    assert_eq!(amplifi_core::image_dimensions(&output).unwrap(), (64, 64));
}

#[test]
#[ignore = "requires the downloaded Windows NCNN engine and a compatible GPU"]
fn custom_model_scale_width_formats_tile_tta_and_compression_end_to_end() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let custom_models = dir.path().join("custom_models");
    std::fs::create_dir_all(&custom_models).unwrap();
    std::fs::copy(
        root.join("models/amplifi-lite-4x.param"),
        custom_models.join("custom-lite-4x.param"),
    )
    .unwrap();
    std::fs::copy(
        root.join("models/amplifi-lite-4x.bin"),
        custom_models.join("custom-lite-4x.bin"),
    )
    .unwrap();
    let listed = amplifi_core::model_list(&custom_models).unwrap();
    assert_eq!(listed, vec!["custom-lite-4x".to_string()]);

    let runner = JobRunner::default();
    let jpg_out = dir.path().join("scaled.jpg");
    let job_jpg = UpscaleJob {
        input: fixtures.join("sample.png"),
        output: jpg_out.clone(),
        models_dir: custom_models.clone(),
        model: "custom-lite-4x".into(),
        scale: 3,
        format: "jpg".into(),
        gpu_id: Some("0".into()),
        custom_width: None,
        compression: 40,
        tile_size: Some(64),
        tta: false,
        double_upscale: false,
        overwrite: false,
        copy_metadata: false,
    };
    runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job_jpg, |_| {})
        .unwrap();
    assert_eq!(amplifi_core::image_dimensions(&jpg_out).unwrap(), (96, 96));

    let webp_out = dir.path().join("custom_width.webp");
    let job_webp = UpscaleJob {
        input: fixtures.join("sample.jpg"),
        output: webp_out.clone(),
        models_dir: custom_models,
        model: "custom-lite-4x".into(),
        scale: 4,
        format: "webp".into(),
        gpu_id: Some("0".into()),
        custom_width: Some(80),
        compression: 25,
        tile_size: Some(64),
        tta: true,
        double_upscale: false,
        overwrite: false,
        copy_metadata: false,
    };
    runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job_webp, |_| {})
        .unwrap();
    assert_eq!(amplifi_core::image_dimensions(&webp_out).unwrap(), (80, 80));
}

#[test]
#[ignore = "requires the downloaded Windows NCNN engine and a compatible GPU"]
fn invalid_gpu_malformed_image_and_missing_output_dir_report_clear_errors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let runner = JobRunner::default();

    // 1. Non-image file rejected before engine launch
    let fake_png = dir.path().join("fake.png");
    std::fs::write(&fake_png, b"not an image").unwrap();
    let mut job = UpscaleJob {
        input: fake_png,
        output: dir.path().join("out1.png"),
        models_dir: root.join("models"),
        model: "amplifi-lite-4x".into(),
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
    let err = runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, |_| {})
        .unwrap_err();
    assert!(err.contains("not a valid PNG, JPEG, or WebP"), "{err}");
    assert!(!job.output.exists());

    // 2. Truncated/corrupted PNG with valid magic header caught by engine stderr + output verification
    let corrupt_png = dir.path().join("corrupt.png");
    std::fs::write(&corrupt_png, b"\x89PNG\r\n\x1a\ncorrupt_body").unwrap();
    job.input = corrupt_png;
    job.output = dir.path().join("out2.png");
    let err = runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, |_| {})
        .unwrap_err();
    assert!(
        err.contains("Couldn't read the image") || err.contains("did not produce a valid image"),
        "{err}"
    );
    assert!(!job.output.exists());

    // 3. Invalid GPU ID reports GPU error and leaves no output file
    job.input = fixtures.join("sample.png");
    job.output = dir.path().join("out3.png");
    job.gpu_id = Some("99".into());
    let err = runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, |_| {})
        .unwrap_err();
    assert!(err.contains("Invalid GPU"), "{err}");
    assert!(!job.output.exists());

    // 4. Missing output folder fails before engine launch
    job.gpu_id = None;
    job.output = dir.path().join("missing_folder/out4.png");
    let err = runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, |_| {})
        .unwrap_err();
    assert!(err.contains("Output folder does not exist"), "{err}");
}

#[test]
#[ignore = "requires the downloaded Windows NCNN engine and a compatible GPU"]
fn cancellation_stops_active_engine_job_and_cleans_up_staging() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cancelled.png");
    let runner = Arc::new(JobRunner::default());
    let runner_for_log = runner.clone();
    let job = UpscaleJob {
        input: fixtures.join("sample.png"),
        output: output.clone(),
        models_dir: root.join("models"),
        model: "amplifi-standard-4x".into(),
        scale: 4,
        format: "png".into(),
        gpu_id: None,
        custom_width: None,
        compression: 0,
        tile_size: None,
        tta: true,
        double_upscale: true,
        overwrite: false,
        copy_metadata: false,
    };
    let err = runner
        .run_job(&root.join("bin/amplifi-engine.exe"), &job, move |line| {
            if line != "Pass 1 of 2" {
                let _ = runner_for_log.cancel();
            }
        })
        .unwrap_err();
    assert_eq!(err, "Upscaling cancelled.");
    assert!(!output.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
