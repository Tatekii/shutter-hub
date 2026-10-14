import { useEffect, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { formatBytes } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";

type Preset = {
  key: string;
  format: "jpeg" | "png" | "webp" | "tiff";
  quality: number;
  maxDimension: number | null;
};

const PRESETS: Preset[] = [
  { key: "export.presetOriginal", format: "jpeg", quality: 92, maxDimension: null },
  { key: "export.presetWeb", format: "jpeg", quality: 88, maxDimension: 2048 },
  { key: "export.presetSocial", format: "jpeg", quality: 85, maxDimension: 1080 },
  { key: "export.presetPreview", format: "jpeg", quality: 80, maxDimension: 800 },
];

const DIMENSIONS: Array<{ value: number | null; label: string }> = [
  { value: null, label: "export.original" },
  { value: 4096, label: "4096" },
  { value: 2048, label: "2048" },
  { value: 1600, label: "1600" },
  { value: 1080, label: "1080" },
  { value: 800, label: "800" },
];

const LAST_DIR_KEY = "shutter_hub.exportDir";

export function ExportDialog() {
  const t = useT();
  const exportIds = useApp((s) => s.exportIds);
  const closeExport = useApp((s) => s.closeExport);
  const runExport = useApp((s) => s.runExport);
  const exportProgress = useApp((s) => s.exportProgress);
  const exportResult = useApp((s) => s.exportResult);

  const [format, setFormat] = useState<"jpeg" | "png" | "webp" | "tiff">("jpeg");
  const [quality, setQuality] = useState(88);
  const [maxDimension, setMaxDimension] = useState<number | null>(2048);
  const [suffix, setSuffix] = useState("");
  const [watermark, setWatermark] = useState("");
  const [targetDir, setTargetDir] = useState(
    () => localStorage.getItem(LAST_DIR_KEY) ?? "",
  );
  const [picking, setPicking] = useState(false);

  const running = exportProgress != null;

  // Esc 关闭（导出中除外）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !running) closeExport();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [running, closeExport]);

  const pickFolder = async () => {
    setPicking(true);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ directory: true, title: t("export.title") });
      if (typeof picked === "string" && picked) {
        setTargetDir(picked);
        localStorage.setItem(LAST_DIR_KEY, picked);
      }
    } finally {
      setPicking(false);
    }
  };

  const applyPreset = (preset: Preset) => {
    setFormat(preset.format);
    setQuality(preset.quality);
    setMaxDimension(preset.maxDimension);
  };

  const activePreset = PRESETS.find(
    (p) =>
      p.format === format && p.quality === quality && p.maxDimension === maxDimension,
  );

  return (
    <div className="export-overlay" onClick={running ? undefined : closeExport}>
      <div
        className="export-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={t("export.title")}
        onClick={(e) => e.stopPropagation()}
      >
        <h2>
          {t("export.title")} · {exportIds.length}
        </h2>

        {exportResult ? (
          <div className="export-done">
            <div className="export-done-line">
              ✓ {t("export.done", {
                n: exportResult.exported,
                size: formatBytes(exportResult.totalBytes),
              })}
            </div>
            {exportResult.failed.length > 0 && (
              <details>
                <summary>{t("export.failedCount", { n: exportResult.failed.length })}</summary>
                <ul>
                  {exportResult.failed.map((f, i) => (
                    <li key={i}>{f}</li>
                  ))}
                </ul>
              </details>
            )}
            <div className="export-actions">
              <button
                className="btn"
                onClick={() => void revealItemInDir(exportResult.targetDir).catch(() => {})}
              >
                {t("export.openFolder")}
              </button>
              <button className="btn btn-primary" onClick={closeExport}>
                {t("export.close")}
              </button>
            </div>
          </div>
        ) : (
          <>
            <div className="export-section">
              <div className="export-label">{t("export.presets")}</div>
              <div className="export-presets">
                {PRESETS.map((p) => (
                  <button
                    key={p.key}
                    className={`btn ${activePreset?.key === p.key ? "btn-active" : ""}`}
                    disabled={running}
                    onClick={() => applyPreset(p)}
                  >
                    {t(p.key)}
                  </button>
                ))}
              </div>
            </div>

            <div className="export-row">
              <label>
                {t("export.format")}
                <select
                  value={format}
                  disabled={running}
                  onChange={(e) => setFormat(e.target.value as typeof format)}
                >
                  <option value="jpeg">JPEG</option>
                  <option value="png">PNG</option>
                  <option value="webp">WebP（无损）</option>
                  <option value="tiff">TIFF（RAW 后期）</option>
                </select>
              </label>

              <label>
                {t("export.maxDim")}
                <select
                  value={maxDimension ?? ""}
                  disabled={running}
                  onChange={(e) =>
                    setMaxDimension(e.target.value ? Number(e.target.value) : null)
                  }
                >
                  {DIMENSIONS.map((d) => (
                    <option key={d.label} value={d.value ?? ""}>
                      {d.label.startsWith("export.") ? t(d.label) : d.label}
                    </option>
                  ))}
                </select>
              </label>

              {format !== "png" && (
                <label>
                  {t("export.quality")}
                  <input
                    type="range"
                    min={40}
                    max={100}
                    value={quality}
                    disabled={running}
                    onChange={(e) => setQuality(Number(e.target.value))}
                  />
                  <span className="export-quality">{quality}</span>
                </label>
              )}

              <label>
                {t("export.suffix")}
                <input
                  type="text"
                  value={suffix}
                  disabled={running}
                  placeholder={t("export.suffixPlaceholder")}
                  onChange={(e) => setSuffix(e.target.value)}
                />
              </label>

              <label>
                {t("export.watermark")}
                <input
                  type="text"
                  value={watermark}
                  disabled={running}
                  placeholder={t("export.watermarkPlaceholder")}
                  onChange={(e) => setWatermark(e.target.value)}
                />
              </label>
            </div>

            <div className="export-section">
              <div className="export-label">{t("export.target")}</div>
              <div className="export-target">
                <span className="export-target-path">
                  {targetDir || t("export.noTarget")}
                </span>
                <button className="btn" disabled={picking || running} onClick={() => void pickFolder()}>
                  {t("export.choose")}
                </button>
              </div>
            </div>

            {running && exportProgress && (
              <div className="export-progress">
                <div className="ai-progress-track">
                  <div
                    className="ai-progress-fill"
                    style={{
                      width: `${exportProgress.total > 0 ? Math.round((exportProgress.processed / exportProgress.total) * 100) : 0}%`,
                    }}
                  />
                </div>
                <div className="export-progress-text">
                  {t("export.running", {
                    done: exportProgress.processed,
                    total: exportProgress.total,
                    current: exportProgress.current,
                  })}
                </div>
              </div>
            )}

            <div className="export-note">{t("export.videoNote")}</div>

            <div className="export-actions">
              <button className="btn" disabled={running} onClick={closeExport}>
                {t("card.close")}
              </button>
              <button
                className="btn btn-primary"
                disabled={running || !targetDir || exportIds.length === 0}
                onClick={() =>
                  void runExport({
                    photoIds: exportIds,
                    targetDir,
                    format,
                    quality,
                    maxDimension,
                    filenameSuffix: suffix,
                    watermark: watermark.trim() ? watermark.trim() : null,
                  })
                }
              >
                {running ? t("card.importing") : t("export.start", { n: exportIds.length })}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
