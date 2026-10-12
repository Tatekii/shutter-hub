import { formatBytes } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";

export function AiPanel({ onClose }: { onClose: () => void }) {
  const t = useT();
  const aiStatus = useApp((s) => s.aiStatus);
  const aiDownload = useApp((s) => s.aiDownload);
  const aiProgress = useApp((s) => s.aiProgress);
  const downloadModels = useApp((s) => s.downloadModels);
  const computeEmbeddings = useApp((s) => s.computeEmbeddings);
  const detectFaces = useApp((s) => s.detectFaces);
  const busy = aiStatus?.busy ?? false;
  const modelsReady = (aiStatus?.clipReady ?? false) && (aiStatus?.faceReady ?? false);

  const downloadLabel = () => {
    if (!aiDownload || aiDownload.done) return null;
    const total = aiDownload.total != null ? formatBytes(aiDownload.total) : "?";
    return t("ai.downloading", {
      file: aiDownload.file,
      received: formatBytes(aiDownload.received),
      total,
    });
  };

  const progressBar = (done: number, total: number) => (
    <div className="ai-progress-track">
      <div
        className="ai-progress-fill"
        style={{ width: `${total > 0 ? Math.round((done / total) * 100) : 0}%` }}
      />
    </div>
  );

  return (
    <div className="ai-panel">
      <div className="ai-panel-header">
        <span>{t("ai.title")}</span>
        <button className="icon-btn" title={t("filter.collapse")} onClick={onClose}>
          ×
        </button>
      </div>

      {!modelsReady ? (
        <div className="ai-section">
          <div className="ai-hint">{t("ai.notDownloaded")}</div>
          <button
            className="btn btn-primary"
            disabled={busy}
            onClick={() => void downloadModels()}
          >
            {t("ai.download")}
          </button>
          {aiDownload && !aiDownload.done && (
            <div className="ai-downloading">
              {downloadLabel()}
              {progressBar(aiDownload.received, aiDownload.total ?? aiDownload.received + 1)}
            </div>
          )}
          {aiDownload?.done && aiDownload.error && (
            <div className="ai-error">{aiDownload.error}</div>
          )}
        </div>
      ) : (
        <div className="ai-section">
          <div className="ai-ready">✓ {t("ai.ready")}</div>

          <div className="ai-row">
            <div className="ai-row-label">
              {t("ai.embeddingsStatus", {
                done: aiStatus?.embeddingsDone ?? 0,
                total: aiStatus?.embeddingsTotal ?? 0,
              })}
            </div>
            {progressBar(aiStatus?.embeddingsDone ?? 0, aiStatus?.embeddingsTotal ?? 0)}
            <button
              className="btn"
              disabled={busy}
              onClick={() => void computeEmbeddings()}
            >
              {t("ai.computeEmbeddings")}
            </button>
          </div>

          <div className="ai-row">
            <div className="ai-row-label">
              {t("ai.facesStatus", {
                done: aiStatus?.facesDone ?? 0,
                total: aiStatus?.facesTotal ?? 0,
              })}
            </div>
            {progressBar(aiStatus?.facesDone ?? 0, aiStatus?.facesTotal ?? 0)}
            <button className="btn" disabled={busy} onClick={() => void detectFaces()}>
              {t("ai.detectFaces")}
            </button>
          </div>

          {aiProgress && (
            <div className="ai-progress-text">
              {t("ai.progress", {
                label:
                  aiProgress.kind === "embeddings"
                    ? t("ai.labelEmbeddings")
                    : t("ai.labelFaces"),
                done: aiProgress.processed,
                total: aiProgress.total,
                current: aiProgress.current,
              })}
            </div>
          )}
          {busy && !aiProgress && <div className="ai-progress-text">{t("ai.busy")}</div>}
        </div>
      )}
    </div>
  );
}
