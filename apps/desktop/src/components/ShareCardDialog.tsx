import { useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { thumbUrl } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";

type Aspect = "1:1" | "4:5" | "3:4";
type StylePreset = "classic" | "plog" | "polaroid" | "plain";

const BACKGROUNDS: Array<{ hex: string; key: string }> = [
  { hex: "#ffffff", key: "collage.bgWhite" },
  { hex: "#111114", key: "collage.bgBlack" },
  { hex: "#f5f0e8", key: "collage.bgPaper" },
  { hex: "#e0a458", key: "collage.bgAccent" },
];

const LAST_DIR_KEY = "shutter_hub.exportDir";
const PREVIEW_W = 250;

function textColorFor(background: string): string {
  const hex = background.replace("#", "");
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  const lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return lum > 0.6 ? "#1e1e1e" : "#f5f5f5";
}

export function ShareCardDialog() {
  const t = useT();
  const shareIds = useApp((s) => s.shareIds);
  const closeShare = useApp((s) => s.closeShare);
  const runShare = useApp((s) => s.runShare);
  const shareBusy = useApp((s) => s.shareBusy);
  const shareResult = useApp((s) => s.shareResult);
  const photos = useApp((s) => s.photos);

  const [aspect, setAspect] = useState<Aspect>("4:5");
  const [stylePreset, setStylePreset] = useState<StylePreset>("classic");
  const [background, setBackground] = useState("#ffffff");
  const [showInfo, setShowInfo] = useState(true);
  const [title, setTitle] = useState("");
  const [caption, setCaption] = useState("");
  const [watermark, setWatermark] = useState("");
  const polaroid = stylePreset === "polaroid";
  const [targetDir, setTargetDir] = useState(
    () => localStorage.getItem(LAST_DIR_KEY) ?? "",
  );
  const [picking, setPicking] = useState(false);

  const selected = shareIds
    .map((id) => photos.find((p) => p.id === id))
    .filter((p): p is NonNullable<typeof p> => p != null);
  const sample = selected[0];
  const aspectRatio = aspect === "1:1" ? 1 : aspect === "3:4" ? 4 / 3 : 5 / 4;
  const textColor = textColorFor(background);

  const infoLines = (p: NonNullable<typeof sample>) => {
    const lines: string[] = [];
    if (title.trim()) lines.push(title.trim());
    if (caption.trim()) lines.push(caption.trim());
    const camera = [p.cameraMake, p.cameraModel].filter(Boolean).join(" ") || p.filename;
    lines.push(camera);
    const params: string[] = [];
    if (p.focalLength != null) params.push(`${p.focalLength}mm`);
    if (p.aperture != null) params.push(`f/${p.aperture}`);
    if (p.shutterSpeed) params.push(p.shutterSpeed);
    if (p.iso != null) params.push(`ISO ${p.iso}`);
    if (params.length) lines.push(params.join("  ·  "));
    if (p.takenAt) lines.push(p.takenAt.slice(0, 10));
    return lines;
  };

  const applyPreset = (preset: StylePreset) => {
    setStylePreset(preset);
    if (preset === "classic") {
      setBackground("#ffffff");
      setShowInfo(true);
    } else if (preset === "plog") {
      setBackground("#f5f0e8");
      setShowInfo(true);
    } else if (preset === "polaroid") {
      setBackground("#ffffff");
      setShowInfo(false);
    } else {
      setShowInfo(false);
    }
  };

  const pickFolder = async () => {
    setPicking(true);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ directory: true, title: t("share.title") });
      if (typeof picked === "string" && picked) {
        setTargetDir(picked);
        localStorage.setItem(LAST_DIR_KEY, picked);
      }
    } finally {
      setPicking(false);
    }
  };

  const generate = () => {
    void runShare({
      photoIds: shareIds,
      aspect,
      background,
      textColor,
      showInfo,
      watermark,
      targetDir,
      title: title.trim(),
      caption: caption.trim(),
      polaroid,
    });
  };

  return (
    <div className="export-overlay" onClick={shareBusy ? undefined : closeShare}>
      <div
        className="export-dialog collage-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={t("share.title")}
        onClick={(e) => e.stopPropagation()}
      >
        <h2>
          {t("share.title")} · {shareIds.length}
        </h2>

        {shareResult ? (
          <div className="export-done">
            <div className="export-done-line">
              ✓ {t("share.done", { n: shareResult.paths.length })}
            </div>
            {shareResult.failed.length > 0 && (
              <div className="ai-error">{t("share.failed", { n: shareResult.failed.length })}</div>
            )}
            <div className="export-actions">
              {sample && (
                <button
                  className="btn"
                  title={t("share.copyCaptionTitle")}
                  onClick={() => {
                    const p = sample;
                    const parts = [
                      [p.cameraMake, p.cameraModel].filter(Boolean).join(" "),
                      p.focalLength != null ? `${p.focalLength}mm` : null,
                      p.aperture != null ? `f/${p.aperture}` : null,
                      p.shutterSpeed,
                      p.iso != null ? `ISO ${p.iso}` : null,
                    ].filter(Boolean);
                    void navigator.clipboard.writeText(
                      `${parts.join(" · ")}\n#ShutterHub #摄影`,
                    );
                  }}
                >
                  {t("share.copyCaption")}
                </button>
              )}
              <button
                className="btn"
                onClick={() => void revealItemInDir(shareResult.paths[0] ?? targetDir)}
              >
                {t("export.openFolder")}
              </button>
              <button className="btn btn-primary" onClick={closeShare}>
                {t("export.close")}
              </button>
            </div>
          </div>
        ) : (
          <>
            <div className="collage-body">
              {/* 预览：与后端版式一致（照片 + 信息条 / 拍立得书写区 + 水印） */}
              <div
                className={`share-preview ${polaroid ? "share-preview-polaroid" : ""}`}
                style={{
                  width: PREVIEW_W,
                  height: Math.round(PREVIEW_W * aspectRatio),
                  background,
                  ...(polaroid
                    ? {
                        paddingTop: PREVIEW_W / 16,
                        paddingLeft: PREVIEW_W / 16,
                        paddingRight: PREVIEW_W / 16,
                        paddingBottom: PREVIEW_W / 8,
                      }
                    : { padding: PREVIEW_W / 20 }),
                }}
              >
                {sample && (
                  <>
                    <div className="share-preview-photo">
                      {thumbUrl(sample.thumbPath) ? (
                        <img src={thumbUrl(sample.thumbPath)!} alt="" draggable={false} />
                      ) : null}
                      {watermark && (
                        <span className="share-preview-watermark">{watermark}</span>
                      )}
                    </div>
                    {polaroid ? (
                      <div className="share-preview-polaroid-caption" style={{ color: textColor }}>
                        {title.trim() && (
                          <span className="share-preview-polaroid-title">{title}</span>
                        )}
                        {caption.trim() && <span>{caption}</span>}
                      </div>
                    ) : (
                      showInfo && (
                        <div className="share-preview-info" style={{ color: textColor }}>
                          {infoLines(sample).map((line, i) => (
                            <span key={i}>{line}</span>
                          ))}
                        </div>
                      )
                    )}
                  </>
                )}
              </div>

              <div className="collage-options">
                <div className="export-section">
                  <div className="export-label">{t("share.style")}</div>
                  <div className="export-presets">
                    {(
                      [
                        ["classic", "share.styleClassic"],
                        ["plog", "share.stylePlog"],
                        ["polaroid", "share.stylePolaroid"],
                        ["plain", "share.stylePlain"],
                      ] as Array<[StylePreset, string]>
                    ).map(([preset, key]) => (
                      <button
                        key={preset}
                        className={`btn ${stylePreset === preset ? "btn-active" : ""}`}
                        disabled={shareBusy}
                        onClick={() => applyPreset(preset)}
                      >
                        {t(key)}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="export-section">
                  <div className="export-label">{t("share.aspect")}</div>
                  <div className="export-presets">
                    {(["1:1", "4:5", "3:4"] as Aspect[]).map((a) => (
                      <button
                        key={a}
                        className={`btn ${aspect === a ? "btn-active" : ""}`}
                        disabled={shareBusy}
                        onClick={() => setAspect(a)}
                      >
                        {a}
                      </button>
                    ))}
                  </div>
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
                        disabled={shareBusy}
                        onClick={() => setBackground(b.hex)}
                      />
                    ))}
                  </div>
                </div>

                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={showInfo}
                    disabled={shareBusy}
                    onChange={(e) => setShowInfo(e.target.checked)}
                  />
                  {t("share.showInfo")}
                </label>

                <div className="export-section">
                  <div className="export-label">{t("share.titleLabel")}</div>
                  <input
                    type="text"
                    className="share-watermark-input"
                    aria-label={t("share.titleLabel")}
                    value={title}
                    disabled={shareBusy}
                    placeholder={t("share.titlePlaceholder")}
                    onChange={(e) => setTitle(e.target.value)}
                  />
                </div>

                <div className="export-section">
                  <div className="export-label">{t("share.captionLabel")}</div>
                  <input
                    type="text"
                    className="share-watermark-input"
                    aria-label={t("share.captionLabel")}
                    value={caption}
                    disabled={shareBusy}
                    placeholder={t("share.captionPlaceholder")}
                    onChange={(e) => setCaption(e.target.value)}
                  />
                </div>

                <div className="export-section">
                  <div className="export-label">{t("share.watermark")}</div>
                  <input
                    type="text"
                    className="share-watermark-input"
                    aria-label={t("share.watermark")}
                    value={watermark}
                    disabled={shareBusy}
                    placeholder={t("share.watermarkPlaceholder")}
                    onChange={(e) => setWatermark(e.target.value)}
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
                      disabled={picking || shareBusy}
                      onClick={() => void pickFolder()}
                    >
                      {t("export.choose")}
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div className="export-actions">
              <button className="btn" disabled={shareBusy} onClick={closeShare}>
                {t("card.close")}
              </button>
              <button
                className="btn btn-primary"
                disabled={shareBusy || !targetDir || selected.length === 0}
                onClick={generate}
              >
                {shareBusy ? t("card.importing") : t("share.generate")}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
