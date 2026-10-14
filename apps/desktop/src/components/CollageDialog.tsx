import { useEffect, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { thumbUrl } from "../lib/api";
import { useT } from "../lib/i18n";
import { useFocusTrap } from "../lib/useFocusTrap";
import { useApp } from "../lib/store";

type LayoutKind = "grid" | "strip" | "featured" | "vertical";

const BACKGROUNDS: Array<{ hex: string; key: string }> = [
  { hex: "#ffffff", key: "collage.bgWhite" },
  { hex: "#111114", key: "collage.bgBlack" },
  { hex: "#f5f0e8", key: "collage.bgPaper" },
  { hex: "#e0a458", key: "collage.bgAccent" },
];

const LAST_DIR_KEY = "shutter_hub.exportDir";
const PREVIEW_W = 320;

export function CollageDialog() {
  const t = useT();
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef);
  const collageIds = useApp((s) => s.collageIds);
  const closeCollage = useApp((s) => s.closeCollage);
  const runCollage = useApp((s) => s.runCollage);
  const collageBusy = useApp((s) => s.collageBusy);
  const collageResult = useApp((s) => s.collageResult);
  const photos = useApp((s) => s.photos);

  const [layout, setLayout] = useState<LayoutKind>("grid");
  const [cols, setCols] = useState(2);
  const [gap, setGap] = useState(12);
  const [radius, setRadius] = useState(24);
  const [background, setBackground] = useState("#ffffff");
  const [size, setSize] = useState(2048);
  const [title, setTitle] = useState("");
  const [targetDir, setTargetDir] = useState(
    () => localStorage.getItem(LAST_DIR_KEY) ?? "",
  );
  const [picking, setPicking] = useState(false);

  // Esc 关闭（渲染中除外）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !collageBusy) closeCollage();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [collageBusy, closeCollage]);

  const selected = collageIds
    .map((id) => photos.find((p) => p.id === id))
    .filter((p): p is NonNullable<typeof p> => p != null);

  // 与后端 layout() 等价的预览排布
  const n = Math.max(1, selected.length);
  const effective = layout === "featured" ? Math.min(n, 3) : n;
  const gridCols =
    layout === "grid" ? cols : layout === "strip" ? Math.min(effective, 6) : layout === "vertical" ? 1 : 3;
  const gridRows =
    layout === "strip"
      ? 1
      : layout === "featured"
        ? 2
        : layout === "vertical"
          ? Math.min(effective, 8)
          : Math.ceil(effective / cols);

  // 按输出坐标系计算预览缩放（保持一致的比例观感）
  const cellOut = Math.max(16, (size - gap * (gridCols + 1)) / gridCols);
  const outW = gridCols * cellOut + gap * (gridCols + 1);
  const outH = gridRows * cellOut + gap * (gridRows + 1) + (title.trim() ? size / 12 : 0);
  const previewW = layout === "vertical" ? 150 : PREVIEW_W;
  const scale = previewW / outW;
  const previewH = Math.round(outH * scale);
  const gapPx = Math.max(0, Math.round(gap * scale));
  const radiusPx = Math.round(radius * scale);
  const titleH = title.trim() ? Math.round((size / 12) * scale) : 0;

  const pickFolder = async () => {
    setPicking(true);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ directory: true, title: t("collage.title") });
      if (typeof picked === "string" && picked) {
        setTargetDir(picked);
        localStorage.setItem(LAST_DIR_KEY, picked);
      }
    } finally {
      setPicking(false);
    }
  };

  const generate = () => {
    void runCollage({
      photoIds: collageIds,
      layout,
      cols,
      gap,
      radius,
      background,
      size,
      quality: 92,
      targetDir,
      title,
    });
  };

  return (
    <div className="export-overlay" onClick={collageBusy ? undefined : closeCollage}>
      <div
        ref={dialogRef}
        tabIndex={-1}
        className="export-dialog collage-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={t("collage.title")}
        onClick={(e) => e.stopPropagation()}
      >
        <h2>
          {t("collage.title")} · {collageIds.length}
        </h2>

        {collageResult ? (
          <div className="export-done">
            <div className="export-done-line">✓ {t("collage.done")}</div>
            <div className="collage-result-path" title={collageResult.path}>
              {collageResult.path}
            </div>
            {collageResult.failed.length > 0 && (
              <div className="ai-error">
                {t("collage.failedCells", { n: collageResult.failed.length })}
              </div>
            )}
            <div className="export-actions">
              <button
                className="btn"
                onClick={() => void revealItemInDir(collageResult.path).catch(() => {})}
              >
                {t("export.openFolder")}
              </button>
              <button className="btn btn-primary" onClick={closeCollage}>
                {t("export.close")}
              </button>
            </div>
          </div>
        ) : (
          <>
            <div className="collage-body">
              {/* 实时预览 */}
              <div
                className="collage-preview"
                style={{
                  width: previewW,
                  height: previewH,
                  background,
                  padding: gapPx,
                  gap: gapPx,
                  paddingTop: titleH > 0 ? titleH + gapPx : gapPx,
                  gridTemplateColumns: `repeat(${gridCols}, 1fr)`,
                  gridAutoRows: "1fr",
                }}
              >
                {title.trim() && (
                  <div className="collage-preview-title" style={{ top: 0, height: titleH }}>
                    {title}
                  </div>
                )}
                {selected.slice(0, layout === "featured" ? 3 : 9).map((p, i) => {
                  const url = thumbUrl(p.thumbPath);
                  const span =
                    layout === "featured" && i === 0
                      ? { gridColumn: "span 2", gridRow: "span 2" }
                      : undefined;
                  return (
                    <div
                      key={p.id}
                      className="collage-cell"
                      style={{ borderRadius: radiusPx, ...span }}
                    >
                      {url ? <img src={url} alt={p.filename} draggable={false} /> : null}
                    </div>
                  );
                })}
              </div>

              <div className="collage-options">
                <div className="export-section">
                  <div className="export-label">{t("collage.layout")}</div>
                  <div className="export-presets">
                    {(
                      [
                        ["grid", "collage.layoutGrid"],
                        ["vertical", "collage.layoutVertical"],
                        ["strip", "collage.layoutStrip"],
                        ["featured", "collage.layoutFeatured"],
                      ] as Array<[LayoutKind, string]>
                    ).map(([kind, key]) => (
                      <button
                        key={kind}
                        className={`btn ${layout === kind ? "btn-active" : ""}`}
                        disabled={collageBusy}
                        onClick={() => setLayout(kind)}
                      >
                        {t(key)}
                      </button>
                    ))}
                  </div>
                </div>

                {layout === "grid" && (
                  <div className="export-row">
                    <label>
                      {t("collage.columns")}
                      <select
                        value={cols}
                        disabled={collageBusy}
                        onChange={(e) => setCols(Number(e.target.value))}
                      >
                        {[2, 3, 4].map((c) => (
                          <option key={c} value={c}>
                            {c}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      {t("collage.size")}
                      <select
                        value={size}
                        disabled={collageBusy}
                        onChange={(e) => setSize(Number(e.target.value))}
                      >
                        {[1080, 2048, 3000].map((s) => (
                          <option key={s} value={s}>
                            {s}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                )}

                <div className="export-row">
                  <label>
                    {t("collage.gap")}
                    <input
                      type="range"
                      min={0}
                      max={32}
                      value={gap}
                      disabled={collageBusy}
                      onChange={(e) => setGap(Number(e.target.value))}
                    />
                  </label>
                  <label>
                    {t("collage.radius")}
                    <input
                      type="range"
                      min={0}
                      max={64}
                      value={radius}
                      disabled={collageBusy}
                      onChange={(e) => setRadius(Number(e.target.value))}
                    />
                  </label>
                </div>

                <div className="export-section">
                  <div className="export-label">{t("collage.background")}</div>
                  <div className="collage-bg-row">
                    {BACKGROUNDS.map((b) => (
                      <button
                        key={b.hex}
                        className={`collage-bg ${background === b.hex ? "on" : ""}`}
                        style={{ background: b.hex }}
                        title={t(b.key)}
                        disabled={collageBusy}
                        onClick={() => setBackground(b.hex)}
                      />
                    ))}
                  </div>
                </div>

                <div className="export-section">
                  <div className="export-label">{t("collage.titleLabel")}</div>
                  <input
                    type="text"
                    className="share-watermark-input"
                    value={title}
                    disabled={collageBusy}
                    placeholder={t("collage.titlePlaceholder")}
                    onChange={(e) => setTitle(e.target.value)}
                  />
                </div>

                <div className="export-section">
                  <div className="export-label">{t("export.target")}</div>
                  <div className="export-target">
                    <span className="export-target-path">
                      {targetDir || t("export.noTarget")}
                    </span>
                    <button
                      className="btn"
                      disabled={picking || collageBusy}
                      onClick={() => void pickFolder()}
                    >
                      {t("export.choose")}
                    </button>
                  </div>
                </div>

                {layout === "featured" && selected.length > 3 && (
                  <div className="export-note">{t("collage.featuredHint")}</div>
                )}
              </div>
            </div>

            <div className="export-actions">
              <button className="btn" disabled={collageBusy} onClick={closeCollage}>
                {t("card.close")}
              </button>
              <button
                className="btn btn-primary"
                disabled={collageBusy || !targetDir || selected.length < 2}
                onClick={generate}
              >
                {collageBusy ? t("card.importing") : t("collage.generate")}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
