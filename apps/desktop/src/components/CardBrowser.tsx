import { useEffect, useRef, useState } from "react";
import { formatBytes, thumbUrl } from "../lib/api";
import { useApp } from "../lib/store";

export function CardBrowser() {
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
              {cardVolume.path} · 可用 {formatBytes(cardVolume.availableBytes)} /{" "}
              {formatBytes(cardVolume.totalBytes)}
            </div>
          </div>
          <button className="icon-btn" title="关闭" onClick={closeVolume}>
            ×
          </button>
        </header>

        {cardScanning ? (
          <div className="card-body">正在扫描存储卡…</div>
        ) : (
          <>
            <div className="card-toolbar">
              <span>
                已选 {selectedCount}/{cardPhotos.length} 张（{formatBytes(totalSelectedBytes)}）
              </span>
              <button className="btn" onClick={() => markAllCard(cardMarks.size < cardPhotos.length)}>
                全选 / 全不选
              </button>
              <button
                className="btn btn-primary"
                disabled={selectedCount === 0 || ingesting}
                onClick={() => void startIngest(target, organize)}
              >
                {ingesting ? "导入中…" : `导入 ${selectedCount} 张`}
              </button>
              <button
                className="btn"
                onClick={() => {
                  if (confirm(`安全弹出「${cardVolume.name}」？`)) void eject(cardVolume.path);
                }}
              >
                安全弹出
              </button>
            </div>

            <div className="card-options">
              <label className="card-target">
                导入到：
                <span className="card-target-path">{target || "未选择目标文件夹"}</span>
                <button
                  className="btn"
                  ref={targetRef}
                  onClick={() => setPicking(true)}
                >
                  选择…
                </button>
              </label>
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={organize}
                  onChange={(e) => setOrganize(e.target.checked)}
                />
                按拍摄日期归档（YYYY/YYYY-MM-DD）
              </label>
            </div>

            {ingesting && ingestProgress && (
              <div className="card-progress">
                导入中 {ingestProgress.processed}/{ingestProgress.total} ·{" "}
                {ingestProgress.current}
              </div>
            )}

            {ingestResult && !ingesting && (
              <div className="card-result">
                ✓ 导入完成：{ingestResult.copied} 张，共{" "}
                {formatBytes(ingestResult.totalBytes)}
                {ingestResult.failed.length > 0 && (
                  <details>
                    <summary>失败 {ingestResult.failed.length} 张</summary>
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
