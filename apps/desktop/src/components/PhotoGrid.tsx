import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef, useState } from "react";
import { formatDuration, thumbUrl, COLOR_HEX } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";
import type { PhotoDto } from "../lib/types";

const CELL_WIDTH = 208;
const CELL_HEIGHT = 212;
const GAP = 12;

export function PhotoGrid({ photos }: { photos: PhotoDto[] }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const columns = Math.max(1, Math.floor((containerWidth + GAP) / (CELL_WIDTH + GAP)));
  const rowCount = Math.ceil(photos.length / columns);

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => containerRef.current,
    estimateSize: () => CELL_HEIGHT + GAP,
    overscan: 4,
  });

  const rows = virtualizer.getVirtualItems();
  const cellCount = Math.max(1, columns);

  return (
    <div ref={containerRef} className="photo-grid">
      <div
        className="photo-grid-inner"
        style={{
          height: virtualizer.getTotalSize(),
          gridTemplateColumns: `repeat(${cellCount}, minmax(0, 1fr))`,
        }}
      >
        {rows.map((row) => {
          const start = row.index * columns;
          const rowPhotos = photos.slice(start, start + columns);
          return (
            <div
              key={row.key}
              className="photo-grid-row"
              style={{
                transform: `translateY(${row.start}px)`,
                gridTemplateColumns: `repeat(${cellCount}, minmax(0, 1fr))`,
              }}
            >
              {rowPhotos.map((photo) => (
                <PhotoCard key={photo.id} photo={photo} />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function PhotoCard({ photo }: { photo: PhotoDto }) {
  const t = useT();
  const url = thumbUrl(photo.thumbPath);
  const selected = useApp((s) => s.selectedIds.has(photo.id));
  const inspectorId = useApp((s) => s.inspectorId);
  const select = useApp((s) => s.select);
  const openViewer = useApp((s) => s.openViewer);

  const isInspector = inspectorId === photo.id;

  return (
    <div
      className={`photo-card ${selected ? "selected" : ""} ${isInspector ? "inspector" : ""}`}
      title={photo.filename}
      onClick={(e) => select(photo.id, e.metaKey || e.ctrlKey)}
      onDoubleClick={() => openViewer(photo.id)}
    >
      <div className="photo-card-thumb">
        {url ? (
          <img src={url} alt={photo.filename} loading="lazy" draggable={false} />
        ) : (
          <div className="photo-card-placeholder">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
              <rect x="3" y="5" width="18" height="14" rx="2" />
              <circle cx="8.5" cy="10" r="1.5" />
              <path d="m3 17 5-5 4 4 3-3 6 6" />
            </svg>
          </div>
        )}
        {photo.mediaType === "video" && (
          <span className="video-badge">
            ▶ {photo.durationMs != null ? formatDuration(photo.durationMs) : t("filter.video")}
          </span>
        )}
        {photo.liveMotion && <span className="live-badge">LIVE</span>}
        {photo.flag && (
          <span className={`flag-badge flag-${photo.flag}`}>
            {photo.flag === "pick" ? "✓" : "✕"}
          </span>
        )}
        {(photo.rating > 0 || photo.hasEdit) && (
          <span className="card-badge-left">
            {photo.rating > 0 && (
              <span className="rating-badge-inline">{"★".repeat(photo.rating)}</span>
            )}
            {photo.hasEdit && <span className="edit-badge-inline">✎</span>}
          </span>
        )}
        {photo.colorLabel && (
          <span
            className="color-badge"
            style={{ background: COLOR_HEX[photo.colorLabel] ?? "#888" }}
          />
        )}
      </div>
      <div className="photo-card-caption">
        <span className="photo-card-name">{photo.filename}</span>
        {photo.takenAt && (
          <span className="photo-card-date">{photo.takenAt.slice(0, 10)}</span>
        )}
      </div>
    </div>
  );
}
