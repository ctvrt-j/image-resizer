import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke, Channel } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import "./App.css";

type ImageInfo = {
  path: string;
  name: string;
  size: number;
  width: number;
  height: number;
  format: string;
};

type ResizeSettings = {
  width: number;
  height: number;
};

type ResizeProgress = {
  current: number;
  total: number;
  info: ImageInfo;
};

const IMAGE_EXTENSIONS = [
  "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp",
  "ico", "pnm", "pbm", "pgm", "ppm", "pam", "tga", "dds", "ff", "qoi",
];

function CheckCircle() {
  return (
    <svg className="check-circle" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="8" />
      <path d="M4.5 8.5l2.2 2.2 4.8-5" />
    </svg>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function App() {
  const [images, setImages] = useState<ImageInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [width, setWidth] = useState("");
  const [height, setHeight] = useState("");
  const [resizing, setResizing] = useState(false);
  const [progress, setProgress] = useState<{ current: number; total: number; name: string } | null>(null);
  const [donePaths, setDonePaths] = useState<Set<string>>(new Set());
  const [finished, setFinished] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const settingsLoaded = useRef(false);
  const busyRef = useRef(false);
  busyRef.current = loading || resizing;

  useEffect(() => {
    invoke<ResizeSettings | null>("load_resize_settings")
      .then((s) => {
        if (s) {
          setWidth(String(s.width));
          setHeight(String(s.height));
        }
      })
      .catch(() => {})
      .finally(() => {
        settingsLoaded.current = true;
      });
  }, []);

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") return;
      if (event.payload.type === "drop") {
        setDragOver(false);
        loadDroppedPaths(event.payload.paths);
      } else {
        // "enter" | "leave" — no highlight while loading or resizing, drops are ignored then anyway
        setDragOver(event.payload.type === "enter" && !busyRef.current);
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  async function loadDroppedPaths(paths: string[]) {
    if (busyRef.current || paths.length === 0) return;

    setLoading(true);
    setError(null);
    setFinished(false);
    setDonePaths(new Set());
    try {
      const infos = await invoke<ImageInfo[]>("load_dropped_paths", { paths });
      setImages(infos);
    } catch (e) {
      setError(String(e));
      setImages([]);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!settingsLoaded.current) return;
    const w = parseInt(width, 10);
    const h = parseInt(height, 10);
    if (!(w > 0) || !(h > 0)) return;
    invoke("save_resize_settings", {
      settings: { width: w, height: h },
    }).catch(() => {});
  }, [width, height]);

  async function handleSelectFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }],
    });
    if (!selected || typeof selected !== "string") return;

    setLoading(true);
    setError(null);
    setFinished(false);
    setDonePaths(new Set());
    try {
      const info = await invoke<ImageInfo>("load_image_file", { path: selected });
      setImages([info]);
    } catch (e) {
      setError(String(e));
      setImages([]);
    } finally {
      setLoading(false);
    }
  }

  async function handleSelectFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (!selected || typeof selected !== "string") return;

    setLoading(true);
    setError(null);
    setFinished(false);
    setDonePaths(new Set());
    try {
      const infos = await invoke<ImageInfo[]>("load_images_from_folder", { folderPath: selected });
      setImages(infos);
    } catch (e) {
      setError(String(e));
      setImages([]);
    } finally {
      setLoading(false);
    }
  }

  async function handleResize() {
    const w = parseInt(width, 10);
    const h = parseInt(height, 10);
    if (!(w > 0) || !(h > 0)) {
      setError("Enter a valid width and height before resizing");
      return;
    }

    setResizing(true);
    setFinished(false);
    setError(null);
    setDonePaths(new Set());
    setProgress({ current: 0, total: images.length, name: "" });

    const onProgress = new Channel<ResizeProgress>();
    onProgress.onmessage = (p) => {
      setProgress({ current: p.current, total: p.total, name: p.info.name });
      setImages((imgs) =>
        imgs.map((img) => (img.path === p.info.path ? p.info : img))
      );
      setDonePaths((prev) => new Set(prev).add(p.info.path));
    };

    try {
      await invoke("resize_images", {
        paths: images.map((img) => img.path),
        width: w,
        height: h,
        onProgress,
      });
      setFinished(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setResizing(false);
      setProgress(null);
    }
  }

  async function removeImage(image: ImageInfo) {
    const filteredImages = images.filter((im) => im !== image)
    setImages(filteredImages)
  }

  const sizeValid = parseInt(width, 10) > 0 && parseInt(height, 10) > 0;

  function handleReset() {
    setImages([]);
    setError(null);
    setFinished(false);
    setProgress(null);
    setDonePaths(new Set());
  }

  return (
    <main className="app">
      <h1 className="app__title">Image Resizer</h1>

      <div className="top-row">
        <div className="resize-controls">
          <label className="size-field">
            <span className="size-field__label">Width</span>
            <input
              type="number"
              min="1"
              value={width}
              disabled={resizing}
              onChange={(e) => setWidth(e.target.value)}
              placeholder="px"
            />
          </label>
          <label className="size-field">
            <span className="size-field__label">Height</span>
            <input
              type="number"
              min="1"
              value={height}
              disabled={resizing}
              onChange={(e) => setHeight(e.target.value)}
              placeholder="px"
            />
          </label>
          <div className="resize-controls__actions">
            <button
              className="resize-button"
              onClick={handleResize}
              disabled={images.length === 0 || resizing || loading || !sizeValid}
            >
              {resizing && <span className="spinner" />}
              {!resizing && "Resize"}
            </button>
            <button
              className="reset-button"
              title="Discard loaded images"
              onClick={handleReset}
              disabled={resizing || images.length === 0}
            >
              ✕
            </button>
          </div>
        </div>

        <div className={`drop-zone${dragOver ? " drop-zone--active" : ""}`}>
          <p className="drop-zone__hint">
            {loading
              ? "Loading…"
              : dragOver
                ? "Release to load"
                : "Drop images or a folder here, or select below"}
          </p>
          <div className="drop-zone__actions">
            <button onClick={handleSelectFile} disabled={loading || resizing}>
              Open Image
            </button>
            <button onClick={handleSelectFolder} disabled={loading || resizing}>
              Open Folder
            </button>
          </div>
        </div>
      </div>

      {progress && (
        <div className="progress">
          <div className="progress__track">
            <div
              className="progress__fill"
              style={{ width: `${(progress.current / progress.total) * 100}%` }}
            />
          </div>
          <p className="progress__label">
            {progress.current} / {progress.total}
            {progress.name && <> &middot; {progress.name}</>}
          </p>
        </div>
      )}

      {finished && (
        <div className="finished-card">
          <CheckCircle />
          <span>Finished</span>
        </div>
      )}

      {error && <p className="error">{error}</p>}

      {images.length > 0 && (
        <section className="image-list">
          <p className="image-list__count">
            {images.length} image{images.length !== 1 ? "s" : ""} loaded
          </p>
          <ul>
            {images.map((img) => (
              <li key={img.path} className="image-item">
                <span className="image-item__name">
                  {donePaths.has(img.path) && <CheckCircle />}
                  <span className="image-item__name-text">{img.name}</span>
                </span>
                <span className="image-item__meta">
                  {img.format} &middot; {img.width}&times;{img.height} &middot; {formatBytes(img.size)}
                  <button
                    className="reset-button"
                    title="Remove Image"
                    onClick={() => removeImage(img)}
                    disabled={resizing}
                  >
                    ✕
                  </button>
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </main>
  );
}

export default App;
