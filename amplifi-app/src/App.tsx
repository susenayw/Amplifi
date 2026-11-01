import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { basename, dirname, join, resourceDir } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import appIcon from "../app-icon.svg";
import {
  comparisonClipPath,
  outputFilename,
  parseProgressLine,
  targetDimensions,
} from "./naming";
import "./App.css";

const modelLabels: Record<string, string> = {
  "amplifi-standard-4x": "Amplify Better",
  "amplifi-lite-4x": "Amplify",
  "upscayl-standard-4x": "Amplify Better",
  "upscayl-lite-4x": "Amplify",
  "high-fidelity-4x": "High Fidelity",
  "digital-art-4x": "Digital Art",
  "remacri-4x": "Remacri (non-commercial)",
  "ultramix-balanced-4x": "Ultramix (non-commercial)",
  "ultrasharp-4x": "Ultrasharp (non-commercial)",
};
const nonCommercialModels = new Set([
  "remacri-4x",
  "ultramix-balanced-4x",
  "ultrasharp-4x",
]);

type Stats = {
  total: number;
  batch: number;
  double: number;
  lastDurationMs: number;
};

type Settings = {
  model?: string;
  scale?: number;
  format?: string;
  gpu?: string;
  width?: string;
  tile?: string;
  tta?: boolean;
  compression?: number;
  doubleUpscale?: boolean;
  overwrite?: boolean;
  copyMetadata?: boolean;
  rememberOutputFolder?: boolean;
  theme?: "dark" | "light";
  viewMode?: "slider" | "lens";
  lensSize?: number;
  outputDir?: string;
  modelsDir?: string;
  stats?: Stats;
};

function readSettings(): Settings {
  try {
    return JSON.parse(localStorage.getItem("amplifi.settings") || "{}");
  } catch {
    return {};
  }
}
const savedSettings = readSettings();

type Job = {
  input: string;
  output: string;
  models_dir: string;
  model: string;
  scale: number;
  format: string;
  gpu_id: string | null;
  custom_width: number | null;
  compression: number;
  tile_size: number | null;
  tta: boolean;
  double_upscale: boolean;
  overwrite: boolean;
  copy_metadata: boolean;
};

type Recommendation = {
  category: string;
  recommended_model: string;
  reason: string;
};

function App() {
  const [input, setInput] = useState("");
  const [batch, setBatch] = useState(false);
  const cancelRequested = useRef(false);
  const [rememberOutputFolder, setRememberOutputFolder] = useState(
    savedSettings.rememberOutputFolder || false,
  );
  const [outputDir, setOutputDir] = useState(
    savedSettings.rememberOutputFolder ? savedSettings.outputDir || "" : "",
  );
  const [modelsDir, setModelsDir] = useState(savedSettings.modelsDir || "");
  const [models, setModels] = useState<string[]>([]);
  const [model, setModel] = useState(savedSettings.model || "amplifi-standard-4x");
  const [scale, setScale] = useState(savedSettings.scale || 4);
  const [format, setFormat] = useState(savedSettings.format || "png");
  const [gpu, setGpu] = useState(savedSettings.gpu || "");
  const [width, setWidth] = useState(savedSettings.width || "");
  const [tile, setTile] = useState(savedSettings.tile || "");
  const [copyMetadata, setCopyMetadata] = useState(
    savedSettings.copyMetadata || false,
  );
  const [overwrite, setOverwrite] = useState(savedSettings.overwrite || false);
  const [doubleUpscale, setDoubleUpscale] = useState(
    savedSettings.doubleUpscale || false,
  );
  const [tta, setTta] = useState(savedSettings.tta || false);
  const [compression, setCompression] = useState(
    savedSettings.compression || 0,
  );
  const [result, setResult] = useState("");
  const [resultVersion, setResultVersion] = useState(0);
  const [lastSaved, setLastSaved] = useState("");
  const [position, setPosition] = useState(50);
  const [theme, setTheme] = useState<"dark" | "light">(
    savedSettings.theme || "dark",
  );
  const [viewMode, setViewMode] = useState<"slider" | "lens">(
    savedSettings.viewMode || "slider",
  );
  const [lensSize, setLensSize] = useState(savedSettings.lensSize || 120);
  const [lensPoint, setLensPoint] = useState<{ x: number; y: number } | null>(
    null,
  );
  const [zoom, setZoom] = useState(100);
  const [inputDims, setInputDims] = useState<{
    width: number;
    height: number;
  } | null>(null);
  const [outputSize, setOutputSize] = useState("");
  const [recommendation, setRecommendation] = useState<Recommendation | null>(
    null,
  );
  const [showSidebar, setShowSidebar] = useState(true);
  const [stats, setStats] = useState<Stats>(
    savedSettings.stats || { total: 0, batch: 0, double: 0, lastDurationMs: 0 },
  );
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [log, setLog] = useState("");
  const [progress, setProgress] = useState<number | null>(null);

  const expectedDims = useMemo(
    () =>
      inputDims
        ? targetDimensions(
            inputDims.width,
            inputDims.height,
            scale,
            doubleUpscale,
            width,
          )
        : null,
    [inputDims, scale, doubleUpscale, width],
  );

  useEffect(() => {
    let active = true;
    void (async () => {
      try {
        const root = await resourceDir();
        const bundledModels = await join(root, "resources", "models");
        const candidates = [bundledModels, "src-tauri/resources/models"];
        const savedModels = savedSettings.modelsDir;
        if (savedModels) {
          try {
            const found = await invoke<string[]>("model_list", { folder: savedModels });
            if (found.length) {
              if (active) setModelsDir(savedModels);
              return;
            }
          } catch {
            // A stale saved custom folder should not prevent built-in models loading.
          }
        }
        for (const candidate of candidates) {
          try {
            const found = await invoke<string[]>("model_list", { folder: candidate });
            if (found.length) {
              if (active) setModelsDir(candidate);
              return;
            }
          } catch {
            // Continue through the development and packaged resource locations.
          }
        }
        if (active) setMessage("Built-in model files could not be found.");
      } catch (error) {
        if (active) setMessage(`Could not locate built-in models: ${String(error)}`);
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    localStorage.setItem(
      "amplifi.settings",
      JSON.stringify({
        model,
        scale,
        format,
        gpu,
        width,
        tile,
        tta,
        doubleUpscale,
        overwrite,
        copyMetadata,
        rememberOutputFolder,
        theme,
        viewMode,
        lensSize,
        compression,
        outputDir: rememberOutputFolder ? outputDir : "",
        modelsDir,
        stats,
      }),
    );
  }, [
    model,
    scale,
    format,
    gpu,
    width,
    tile,
    tta,
    doubleUpscale,
    overwrite,
    copyMetadata,
    rememberOutputFolder,
    theme,
    viewMode,
    lensSize,
    compression,
    outputDir,
    modelsDir,
    stats,
  ]);

  useEffect(() => {
    let active = true;
    setModels([]);
    if (modelsDir)
      invoke<string[]>("model_list", { folder: modelsDir })
        .then((items) => {
          if (!active) return;
          setModels(items);
          setModel((current) =>
            items.includes(current) ? current : items[0] || "",
          );
          if (!items.length)
            setMessage(
              "No matching .param and .bin model pairs were found in this folder.",
            );
        })
        .catch((e) => {
          if (active) setMessage(`Could not read models from ${modelsDir}: ${String(e)}`);
        });
    return () => {
      active = false;
    };
  }, [modelsDir]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("upscale-log", (event) => {
      const line = event.payload.trim();
      const parsed = parseProgressLine(line);
      if (parsed !== null) setProgress(parsed);
      if (line) setLog((prev) => (prev + line + "\n").slice(-10000));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  function clearSelection() {
    setInput("");
    setResult("");
    setLastSaved("");
    setInputDims(null);
    setOutputSize("");
    setRecommendation(null);
    setMessage("");
    setLog("");
    setProgress(null);
  }

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop" || busy) return;
        const paths = event.payload.paths;
        if (paths.length !== 1) {
          setMessage("Drop one image or one folder at a time.");
          return;
        }
        void (async () => {
          const path = paths[0];
          try {
            const kind = await invoke<string>("inspect_drop", { path });
            if (kind === "image") await invoke("allow_preview", { path });
            setBatch(kind === "folder");
            setInput(path);
            setResult("");
            setLastSaved("");
            setInputDims(null);
            setOutputSize("");
            setRecommendation(null);
            setMessage("");
            setLog("");
            if (!rememberOutputFolder || !outputDir) {
              setOutputDir(kind === "folder" ? path : await dirname(path));
            }
          } catch (error) {
            setMessage(`Could not process dropped item: ${String(error)}`);
          }
        })();
      })
      .then((fn) => {
        if (active) unlisten = fn;
        else fn();
      })
      .catch((e) => {
        if (active) setMessage(`Drag-and-drop setup unavailable: ${String(e)}`);
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [busy, outputDir, rememberOutputFolder]);

  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      if (
        busy ||
        event.target instanceof HTMLInputElement ||
        event.target instanceof HTMLTextAreaElement
      )
        return;
      const image = Array.from(event.clipboardData?.files || []).find((file) =>
        ["image/png", "image/jpeg", "image/webp"].includes(file.type),
      );
      if (!image) return;
      event.preventDefault();
      if (image.size > 32 * 1024 * 1024) {
        setMessage("Clipboard image must be 32 MiB or smaller.");
        return;
      }
      void image
        .arrayBuffer()
        .then((buffer) => invoke<string>("paste_image", buffer))
        .then((path) => {
          setBatch(false);
          setInput(path);
          setResult("");
          setLastSaved("");
          setInputDims(null);
          setOutputSize("");
          setRecommendation(null);
          setLog("");
          setMessage(
            outputDir
              ? "Clipboard image loaded."
              : "Clipboard image loaded. Choose an output folder.",
          );
        })
        .catch((error) => setMessage(String(error)));
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [busy, outputDir]);

  function analyzePreviewImage(img: HTMLImageElement) {
    if (!models.length || img.naturalWidth === 0 || img.naturalHeight === 0)
      return;
    try {
      const sampleW = Math.min(64, img.naturalWidth);
      const sampleH = Math.min(64, img.naturalHeight);
      const canvas = document.createElement("canvas");
      canvas.width = sampleW;
      canvas.height = sampleH;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      ctx.drawImage(img, 0, 0, sampleW, sampleH);
      const rgba = Array.from(ctx.getImageData(0, 0, sampleW, sampleH).data);
      void invoke<Recommendation>("recommend_model", {
        width: sampleW,
        height: sampleH,
        rgba,
        models,
      })
        .then((rec) => setRecommendation(rec))
        .catch(() => setRecommendation(null));
    } catch {
      setRecommendation(null);
    }
  }

  async function resetSettings() {
    setModel("amplifi-standard-4x");
    setScale(4);
    setFormat("png");
    setGpu("");
    setWidth("");
    setTile("");
    setTta(false);
    setCompression(0);
    setOverwrite(false);
    setDoubleUpscale(false);
    setCopyMetadata(false);
    setRememberOutputFolder(false);
    setTheme("dark");
    setViewMode("slider");
    setLensSize(120);
    setOutputDir("");
    try {
      setModelsDir(await join(await resourceDir(), "resources", "models"));
      setMessage("Settings reset.");
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function chooseImage() {
    const path = await open({
      multiple: false,
      filters: [
        { name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] },
      ],
    });
    if (path) {
      await invoke("allow_preview", { path });
      setInput(path);
      setResult("");
      setLastSaved("");
      setInputDims(null);
      setOutputSize("");
      setRecommendation(null);
      setMessage("");
      if (!rememberOutputFolder || !outputDir)
        setOutputDir(await dirname(path));
    }
  }

  async function chooseInputFolder() {
    const folder = await open({ directory: true, multiple: false });
    if (folder) {
      setInput(folder);
      setResult("");
      setLastSaved("");
      setInputDims(null);
      setOutputSize("");
      setRecommendation(null);
      setMessage("");
      if (!rememberOutputFolder || !outputDir) setOutputDir(folder);
    }
  }

  async function chooseFolder(setter: (path: string) => void) {
    const path = await open({ directory: true, multiple: false });
    if (path) setter(path);
  }

  async function run() {
    if (!input || !outputDir || !model) {
      setMessage("Choose an image, model and output folder first.");
      return;
    }
    const startedAt = Date.now();
    setBusy(true);
    cancelRequested.current = false;
    setMessage("");
    setLog("");
    setResult("");
    setLastSaved("");
    setOutputSize("");
    setProgress(null);
    try {
      const inputs = batch
        ? await invoke<string[]>("list_images", { folder: input })
        : [input];
      if (!inputs.length)
        throw new Error("No supported images were found in that folder.");
      const destination = batch
        ? await invoke<string>("batch_output_folder", { folder: outputDir })
        : outputDir;
      let completed = 0;
      let failed = 0;
      for (const image of inputs) {
        if (cancelRequested.current) break;
        const filename = await basename(image);
        const out = await join(
          destination,
          outputFilename(
            filename,
            batch,
            width,
            scale,
            doubleUpscale,
            model,
            format,
          ),
        );
        const job: Job = {
          input: image,
          output: out,
          models_dir: modelsDir,
          model,
          scale,
          format,
          gpu_id: gpu || null,
          custom_width: width ? Number(width) : null,
          compression,
          tile_size: tile ? Number(tile) : null,
          tta,
          double_upscale: doubleUpscale,
          overwrite,
          copy_metadata: copyMetadata,
        };
        setProgress(null);
        setMessage(`Processing ${completed + 1} of ${inputs.length}…`);
        try {
          const saved = await invoke<string>("upscale", { job });
          completed++;
          setLastSaved(saved);
          if (!batch) {
            await invoke("allow_preview", { path: saved });
            setResultVersion((v) => v + 1);
            setResult(saved);
          }
        } catch (error) {
          if (cancelRequested.current) break;
          if (!batch) throw error;
          failed++;
          setLog((prev) => `${prev}\n${image}: ${String(error)}\n`);
        }
      }
      if (completed > 0) {
        const elapsed = Date.now() - startedAt;
        setStats((prev) => ({
          total: prev.total + completed,
          batch: prev.batch + (batch ? 1 : 0),
          double: prev.double + (doubleUpscale ? completed : 0),
          lastDurationMs: elapsed,
        }));
      }
      setMessage(
        cancelRequested.current
          ? `Stopped after ${completed} of ${inputs.length} images.`
          : `Saved ${completed} image${completed === 1 ? "" : "s"}${batch ? " in the Amplifi outputs folder" : ""}${failed ? `; ${failed} failed. See the processing log.` : "."}`,
      );
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className={`app ${showSidebar ? "" : "sidebar-collapsed"}`}
      data-theme={theme}
    >
      {showSidebar && (
        <aside>
          <div className="side-content">
            <header>
              <img className="brand-icon" src={appIcon} alt="" />
              <div>
                <h1>Amplifi</h1>
                <p>AI image upscaler</p>
              </div>
            </header>
            <section>
              <h2>Image</h2>
              <label className="check">
                <input
                  type="checkbox"
                  checked={batch}
                  onChange={(e) => {
                    setBatch(e.target.checked);
                    clearSelection();
                  }}
                  disabled={busy}
                />{" "}
                Batch folder
              </label>
              <div className="button-row">
                <button
                  onClick={batch ? chooseInputFolder : chooseImage}
                  disabled={busy}
                >
                  {batch ? "Choose folder" : "Choose image"}
                </button>
                {input && (
                  <button
                    className="secondary"
                    onClick={clearSelection}
                    disabled={busy}
                  >
                    Clear
                  </button>
                )}
              </div>
              <p className="path">
                {input || (batch ? "No folder selected" : "No image selected")}
              </p>
            </section>
            <section>
              <h2>Upscaling</h2>
              <label>
                Model
                <select
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  disabled={busy || !models.length}
                >
                  {models.map((m) => (
                    <option key={m} value={m}>
                      {modelLabels[m] || m}
                    </option>
                  ))}
                </select>
              </label>
              {recommendation && !batch && (
                <div className="recommendation">
                  <p className="path">
                    <strong>
                      Suggested for {recommendation.category}:{" "}
                      {modelLabels[recommendation.recommended_model] ||
                        recommendation.recommended_model}
                    </strong>{" "}
                    — {recommendation.reason}
                  </p>
                  {model !== recommendation.recommended_model &&
                    models.includes(recommendation.recommended_model) && (
                      <button
                        className="secondary small-btn"
                        onClick={() =>
                          setModel(recommendation.recommended_model)
                        }
                        disabled={busy}
                      >
                        Use{" "}
                        {modelLabels[recommendation.recommended_model] ||
                          recommendation.recommended_model}
                      </button>
                    )}
                </div>
              )}
              {nonCommercialModels.has(model) && (
                <p className="path">
                  This model is for non-commercial use. See attribution and
                  license information before use.
                </p>
              )}
              <label>
                Scale
                <select
                  value={scale}
                  onChange={(e) => setScale(Number(e.target.value))}
                  disabled={busy}
                >
                  {Array.from({ length: 16 }, (_, i) => i + 1).map((n) => (
                    <option key={n} value={n}>
                      {n}×
                    </option>
                  ))}
                </select>
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={doubleUpscale}
                  onChange={(e) => setDoubleUpscale(e.target.checked)}
                  disabled={busy}
                />{" "}
                Double upscale
              </label>
              {scale > 4 && (
                <p className="path">
                  Values above 4× resize the AI output; they do not add more AI
                  detail.
                </p>
              )}
              <label>
                Custom width (optional)
                <input
                  type="number"
                  min="1"
                  value={width}
                  onChange={(e) => setWidth(e.target.value)}
                  disabled={busy}
                />
              </label>
            </section>
            <section>
              <h2>Output</h2>
              <button
                className="secondary"
                onClick={() => chooseFolder(setOutputDir)}
                disabled={busy}
              >
                Choose folder
              </button>
              <p className="path">{outputDir || "No output folder selected"}</p>
              <label className="check">
                <input
                  type="checkbox"
                  checked={rememberOutputFolder}
                  onChange={(e) => setRememberOutputFolder(e.target.checked)}
                  disabled={busy}
                />{" "}
                Remember output folder
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={overwrite}
                  onChange={(e) => setOverwrite(e.target.checked)}
                  disabled={busy}
                />{" "}
                Replace existing output
              </label>
              <label>
                Format
                <select
                  value={format}
                  onChange={(e) => setFormat(e.target.value)}
                  disabled={busy}
                >
                  <option value="png">PNG</option>
                  <option value="jpg">JPEG</option>
                  <option value="webp">WebP</option>
                </select>
              </label>
              {format === "jpg" && (
                <p className="path">JPEG does not preserve transparency.</p>
              )}
            </section>
            <details>
              <summary>Advanced settings</summary>
              <label>
                Appearance
                <select
                  value={theme}
                  onChange={(e) =>
                    setTheme(e.target.value as "dark" | "light")
                  }
                >
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                </select>
              </label>
              <label>
                Custom model folder
                <button
                  className="secondary"
                  onClick={() => chooseFolder(setModelsDir)}
                  disabled={busy}
                >
                  Choose model folder
                </button>
              </label>
              <button
                className="secondary"
                onClick={async () =>
                  setModelsDir(
                    await join(await resourceDir(), "resources", "models"),
                  )
                }
                disabled={busy}
              >
                Use built-in models
              </button>
              <p className="path">{modelsDir}</p>
              <label>
                GPU ID
                <input
                  value={gpu}
                  onChange={(e) => setGpu(e.target.value)}
                  disabled={busy}
                  placeholder="Automatic"
                />
              </label>
              <label>
                Tile size
                <input
                  type="number"
                  min="32"
                  value={tile}
                  onChange={(e) => setTile(e.target.value)}
                  disabled={busy}
                  placeholder="Automatic"
                />
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={copyMetadata}
                  onChange={(e) => setCopyMetadata(e.target.checked)}
                  disabled={busy}
                />{" "}
                Copy EXIF and ICC metadata
              </label>
              {copyMetadata && (
                <p className="path">
                  Some EXIF fields may still describe the original image; other
                  metadata is not copied.
                </p>
              )}
              <label>
                Compression
                <input
                  type="number"
                  min="0"
                  max="100"
                  value={compression}
                  onChange={(e) => setCompression(Number(e.target.value))}
                  disabled={busy}
                />
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={tta}
                  onChange={(e) => setTta(e.target.checked)}
                  disabled={busy}
                />{" "}
                Test-time augmentation
              </label>
              {stats.total > 0 && (
                <p className="path">
                  Local stats: {stats.total} upscaled ({stats.batch} batch,{" "}
                  {stats.double} double; last{" "}
                  {(stats.lastDurationMs / 1000).toFixed(1)}s)
                </p>
              )}
              <button
                className="secondary"
                onClick={resetSettings}
                disabled={busy}
              >
                Reset settings
              </button>
            </details>
          </div>
          <div className="controls">
            {busy ? (
              <button
                className="action"
                onClick={() => {
                  cancelRequested.current = true;
                  invoke("cancel").catch((e) => setMessage(String(e)));
                }}
              >
                Cancel
              </button>
            ) : (
              <button
                className="action"
                onClick={run}
                disabled={!input || !models.length}
              >
                {batch ? "Upscale folder" : "Upscale image"}
              </button>
            )}
            {busy && (
              <progress
                max={100}
                value={progress ?? undefined}
                aria-label="Current image progress"
              />
            )}
            {message && (
              <p role="status" className="message">
                {message}
              </p>
            )}
            {lastSaved && !busy && (
              <button
                className="secondary"
                onClick={() =>
                  revealItemInDir(lastSaved).catch((e) => setMessage(String(e)))
                }
              >
                Show output in folder
              </button>
            )}
          </div>
        </aside>
      )}
      <main>
        <div className="top-bar">
          <button
            className="secondary small-btn"
            onClick={() => setShowSidebar((prev) => !prev)}
          >
            {showSidebar ? "Hide sidebar" : "Show sidebar"}
          </button>
        </div>
        <div
          className="preview"
          onPointerMove={(e) => {
            if (viewMode === "lens") {
              const box = e.currentTarget.getBoundingClientRect();
              setLensPoint({
                x: e.clientX - box.left,
                y: e.clientY - box.top,
              });
            }
          }}
          onPointerLeave={() => setLensPoint(null)}
        >
          {input && !batch ? (
            <>
              <img
                crossOrigin="anonymous"
                style={{ transform: `scale(${zoom / 100})` }}
                src={
                  result
                    ? `${convertFileSrc(result)}?v=${resultVersion}`
                    : convertFileSrc(input)
                }
                alt={result ? "Upscaled image" : "Original image"}
                onLoad={(e) => {
                  const img = e.currentTarget;
                  if (result) {
                    setOutputSize(`${img.naturalWidth} × ${img.naturalHeight}`);
                  } else {
                    setInputDims({
                      width: img.naturalWidth,
                      height: img.naturalHeight,
                    });
                    analyzePreviewImage(img);
                  }
                }}
              />
              {result && (
                <div
                  className="original"
                  style={{
                    clipPath: comparisonClipPath(
                      viewMode,
                      position,
                      lensSize,
                      lensPoint,
                    ),
                  }}
                >
                  <img
                    crossOrigin="anonymous"
                    style={{ transform: `scale(${zoom / 100})` }}
                    src={convertFileSrc(input)}
                    alt="Original image for comparison"
                    onLoad={(e) =>
                      setInputDims({
                        width: e.currentTarget.naturalWidth,
                        height: e.currentTarget.naturalHeight,
                      })
                    }
                  />
                </div>
              )}
            </>
          ) : (
            <div className="empty">
              <h2>
                {batch && input ? "Batch ready" : "Make more of your images"}
              </h2>
              <p>
                {batch
                  ? input
                    ? "Start upscaling to process the images in this folder."
                    : "Choose a folder of images to get started."
                  : "Choose, drop, or paste an image to get started."}
              </p>
            </div>
          )}
        </div>
        {input && !batch && (
          <div className="preview-tools">
            <span>
              {inputDims &&
                `Original ${inputDims.width} × ${inputDims.height}`}
              {outputSize
                ? `  →  Output ${outputSize}`
                : expectedDims
                  ? `  →  Target ${expectedDims.width} × ${expectedDims.height}`
                  : ""}
            </span>
            <label>
              Zoom{" "}
              <input
                type="range"
                min="50"
                max="400"
                step="25"
                value={zoom}
                onChange={(e) => setZoom(Number(e.target.value))}
                aria-label="Preview zoom"
              />{" "}
              {zoom}%
            </label>
          </div>
        )}
        {result && (
          <>
            <div className="view-settings">
              <label>
                Compare{" "}
                <select
                  value={viewMode}
                  onChange={(e) =>
                    setViewMode(e.target.value as "slider" | "lens")
                  }
                >
                  <option value="slider">Slider</option>
                  <option value="lens">Lens</option>
                </select>
              </label>
              {viewMode === "lens" && (
                <label>
                  Lens size{" "}
                  <input
                    type="range"
                    min="40"
                    max="250"
                    value={lensSize}
                    onChange={(e) => setLensSize(Number(e.target.value))}
                  />
                </label>
              )}
            </div>
            {viewMode === "slider" ? (
              <label className="compare">
                Original{" "}
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={position}
                  onChange={(e) => setPosition(Number(e.target.value))}
                  aria-label="Compare original and upscaled images"
                />{" "}
                Upscaled
              </label>
            ) : (
              <p className="lens-hint">
                Move the pointer over the image to reveal the original. Use
                Slider for keyboard comparison.
              </p>
            )}
          </>
        )}
        {log && (
          <details className="log">
            <summary>Processing log</summary>
            <button
              className="secondary small-btn"
              onClick={() => void navigator.clipboard.writeText(log)}
            >
              Copy log
            </button>
            <pre>{log}</pre>
          </details>
        )}
      </main>
    </div>
  );
}
export default App;
