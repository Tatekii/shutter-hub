import { useEffect, useRef, useState } from "react";
import { formatBytes, thumbUrl } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";

export function CardBrowser() {
  const t = useT();
  const cardVolume = useApp((s) => s.cardVolume);
  const cardPhotos = useApp((s) => s.cardPhotos);
  const cardMarks = useApp((s) => s.cardMarks);
  const cardScanning = useApp((s) => s.cardScanning);
  const closeVolume = useApp((s) => s.closeVolume);
  const toggleCardMark = useApp((s) => s.toggleCardMark);
  const markAllCard = useApp((s) => s.markAllCard);
  const startIngest = useApp((s) => s.startIngest);
  const ingesting = useApp((s) => s.ingesting);
  const ingestProgress = useApp((s) => s.ingestProgress);
  const ingestResult = useApp((s) => s.ingestResult);
  const eject = useApp((s) => s.eject);

  const [target, setTarget] = useState("");
  const [organize, setOrganize] = useState(true);
  const [picking, setPicking] = useState(false);
  const targetRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!picking) return;
    void import("@tauri-apps/plugin-dialog")
      .then(({ open }) => open({ directory: true }))
      .then((picked) => {
        if (typeof picked === "string" && picked) setTarget(picked);
        setPicking(false);
      });
  }, [picking]);

  if (!cardVolume) return null;

  const selectedCount = cardMarks.size;
  const totalSelectedBytes = cardPhotos
    .filter((p) => cardMarks.has(p.path))
    .reduce((sum, p) => sum + p.size, 0);

  return (
    <div className="card-overlay">
      <div className="card-dialog">
        <header className="card-header">
          <div>
            <div className="card-title">💾 {cardVolume.name}</div>
            <div className="card-subtitle">
              {cardVolume.path} ·{" "}
              {t("card.space", {
                free: formatBytes(cardVolume.availableBytes),
                total: formatBytes(cardVolume.totalBytes),
              })}
            </div>
          </div>
          <button className="icon-btn" title={t("card.close")} onClick={closeVolume}>
            ×
          </button>
        </header>

        {cardScanning ? (
          <div className="card-body">{t("card.scanning")}</div>
        ) : (
          <>
            <div className="card-toolbar">
              <span>
                {t("card.selected", {
                  n: selectedCount,
                  total: cardPhotos.length,
                  size: formatBytes(totalSelectedBytes),
                })}
              </span>
              <button
                className="btn"
                onClick={() => markAllCard(cardMarks.size < cardPhotos.length)}
              >
                {t("card.toggleAll")}
              </button>
              <button
                className="btn btn-primary"
                disabled={selectedCount === 0 || ingesting}
                onClick={() => void startIngest(target, organize)}
              >
                {ingesting ? t("card.importing") : t("card.import", { n: selectedCount })}
              </button>
              <button
                className="btn"
                onClick={() => {
                  if (confirm(t("card.ejectConfirm", { name: cardVolume.name }))) {
                    void eject(cardVolume.path);
                  }
                }}
              >
                {t("card.eject")}
              </button>
            </div>

            <div className="card-options">
              <label className="card-target">
                {t("card.importTo")}
                <span className="card-target-path">{target || t("card.noTarget")}</span>
                <button
                  className="btn"
                  ref={targetRef}
                  onClick={() => setPicking(true)}
                >
                  {t("card.choose")}
                </button>
              </label>
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={organize}
                  onChange={(e) => setOrganize(e.target.checked)}
                />
                {t("card.organizeByDate")}
              </label>
            </div>

            {ingesting && ingestProgress && (
              <div className="card-progress">
                {t("card.progress", {
                  done: ingestProgress.processed,
                  total: ingestProgress.total,
                  current: ingestProgress.current,
                })}
              </div>
            )}

            {ingestResult && !ingesting && (
              <div className="card-result">
                {t("card.done", {
                  n: ingestResult.copied,
                  size: formatBytes(ingestResult.totalBytes),
                })}
                {ingestResult.failed.length > 0 && (
                  <details>
                    <summary>{t("card.failed", { n: ingestResult.failed.length })}</summary>
                    <ul>
                      {ingestResult.failed.map((f, i) => (
                        <li key={i}>{f}</li>
                      ))}
                    </ul>
                  </details>
                )}
              </div>
            )}

            <div className="card-grid">
              {cardPhotos.map((p) => {
                const marked = cardMarks.has(p.path);
                return (
                  <div
                    key={p.path}
                    className={`card-cell ${marked ? "marked" : ""}`}
                    onClick={() => toggleCardMark(p.path)}
                    title={p.filename}
                  >
                    <div className="card-cell-thumb">
                      {thumbUrl(p.thumbPath) ? (
                        <img
                          src={thumbUrl(p.thumbPath)!}
                          alt={p.filename}
                          loading="lazy"
                          draggable={false}
                        />
                      ) : (
                        <div className="photo-card-placeholder" />
                      )}
                      {p.mediaType === "video" && <span className="video-badge">▶</span>}
                      <span className={`card-check ${marked ? "on" : ""}`}>
                        {marked ? "✓" : ""}
                      </span>
                    </div>
                    <span className="card-cell-name">{p.filename}</span>
                  </div>
                );
              })}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
