import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { COLOR_HEX, formatBytes, getViewerPreview } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";

const MIN_ZOOM = 0.1;
const MAX_ZOOM = 8;
const WHEEL_STEP = 1.15;

export function Viewer() {
  const t = useT();
  const viewerId = useApp((s) => s.viewerId);
  const photos = useApp((s) => s.photos);
  const closeViewer = useApp((s) => s.closeViewer);
  const viewerMove = useApp((s) => s.viewerMove);
  const rateSelected = useApp((s) => s.rateSelected);
  const colorSelected = useApp((s) => s.colorSelected);
  const flagSelected = useApp((s) => s.flagSelected);

  const photo = photos.find((p) => p.id === viewerId) ?? null;
  const [url, setUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [mode, setMode] = useState<"fit" | "zoom">("fit");
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [autoAdvance, setAutoAdvance] = useState(true);
  const [natural, setNatural] = useState<{ w: number; h: number } | null>(null);
  const [canvas, setCanvas] = useState({ w: 0, h: 0 });
  const [drag, setDrag] = useState<{
    startX: number;
    startY: number;
    baseX: number;
    baseY: number;
  } | null>(null);
  const canvasRef = useRef<HTMLDivElement>(null);

  // 当前照片被筛选掉（如旗标筛选 + 看图内淘汰）且无邻居时自动退出
  useEffect(() => {
    if (viewerId != null && !photo) closeViewer();
  }, [viewerId, photo, closeViewer]);

  // 切换照片时重置显示状态并懒加载全尺寸预览
  useEffect(() => {
    if (!photo) return;
    let cancelled = false;
    setUrl(null);
    setFailed(false);
    setNatural(null);
    setMode("fit");
    setZoom(1);
    setPan({ x: 0, y: 0 });
    void getViewerPreview(photo.id)
      .then((path) => {
        if (cancelled) return;
        if (path) setUrl(convertFileSrc(path));
        else setFailed(true);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [photo?.id]);

  useEffect(() => {
    const el = canvasRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setCanvas({ w: entry.contentRect.width, h: entry.contentRect.height });
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const toggleMode = () => {
    setMode((m) => (m === "zoom" ? "fit" : "zoom"));
    setZoom(1);
    setPan({ x: 0, y: 0 });
  };

  const doCull = async (action: () => Promise<void>) => {
    await action();
    if (autoAdvance) viewerMove(1);
  };

  useEffect(() => {
    if (!photo) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT"
      ) {
        return;
      }
      if (e.key === "Escape") {
        closeViewer();
        return;
      }
      if (e.key === "ArrowLeft") {
        viewerMove(-1);
        return;
      }
      if (e.key === "ArrowRight") {
        viewerMove(1);
        return;
      }
      if (e.key.toLowerCase() === "z") {
        toggleMode();
        return;
      }
      const n = parseInt(e.key, 10);
      if (n >= 1 && n <= 5) {
        void doCull(() => rateSelected(n));
        return;
      }
      if (e.key === "0") {
        void doCull(() => rateSelected(0));
        return;
      }
      if (e.key === "6") {
        void doCull(() => colorSelected("red"));
        return;
      }
      if (e.key === "7") {
        void doCull(() => colorSelected("yellow"));
        return;
      }
      if (e.key === "8") {
        void doCull(() => colorSelected("green"));
        return;
      }
      if (e.key === "9") {
        void doCull(() => colorSelected("blue"));
        return;
      }
      if (e.key.toLowerCase() === "p") {
        void doCull(() => flagSelected("pick"));
        return;
      }
      if (e.key.toLowerCase() === "x") {
        void doCull(() => flagSelected("reject"));
        return;
      }
      if (e.key.toLowerCase() === "u") {
        void doCull(() => flagSelected(""));
        return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [photo?.id, autoAdvance]);

  if (viewerId == null || !photo) return null;

  const index = photos.findIndex((p) => p.id === photo.id);

  const exif: string[] = [];
  if (photo.cameraModel || photo.cameraMake) {
    exif.push([photo.cameraMake, photo.cameraModel].filter(Boolean).join(" "));
  }
  if (photo.focalLength != null) exif.push(`${photo.focalLength}mm`);
  if (photo.aperture != null) exif.push(`f/${photo.aperture}`);
  if (photo.shutterSpeed) exif.push(photo.shutterSpeed);
  if (photo.iso != null) exif.push(`ISO ${photo.iso}`);
  if (photo.width && photo.height) exif.push(`${photo.width}×${photo.height}`);
  exif.push(formatBytes(photo.size));

  const dispW = mode === "zoom" && natural ? natural.w * zoom : 0;
  const dispH = mode === "zoom" && natural ? natural.h * zoom : 0;
  const maxX = Math.max(0, (dispW - canvas.w) / 2);
  const maxY = Math.max(0, (dispH - canvas.h) / 2);
  const cx = Math.min(Math.max(pan.x, -maxX), maxX);
  const cy = Math.min(Math.max(pan.y, -maxY), maxY);

  return (
    <div className="viewer">
      <div
        ref={canvasRef}
        className={`viewer-canvas ${mode === "zoom" ? "zooming" : ""} ${drag ? "dragging" : ""}`}
        onWheel={(e) => {
          const base = mode === "zoom" ? zoom : 1;
          const next = Math.min(
            MAX_ZOOM,
            Math.max(MIN_ZOOM, base * (e.deltaY < 0 ? WHEEL_STEP : 1 / WHEEL_STEP)),
          );
          setMode("zoom");
          setZoom(next);
        }}
        onDoubleClick={toggleMode}
        onPointerDown={(e) => {
          if (mode !== "zoom") return;
          e.currentTarget.setPointerCapture(e.pointerId);
          setDrag({ startX: e.clientX, startY: e.clientY, baseX: pan.x, baseY: pan.y });
        }}
        onPointerMove={(e) => {
          if (!drag) return;
          setPan({
            x: drag.baseX + e.clientX - drag.startX,
            y: drag.baseY + e.clientY - drag.startY,
          });
        }}
        onPointerUp={() => setDrag(null)}
        onPointerCancel={() => setDrag(null)}
      >
        {url && !failed && (
          <img
            className={`viewer-img ${mode === "zoom" ? "viewer-zoom" : ""}`}
            src={url}
            alt={photo.filename}
            draggable={false}
            style={
              mode === "zoom" && natural
                ? {
                    left: canvas.w / 2 + cx,
                    top: canvas.h / 2 + cy,
                    width: dispW,
                    height: dispH,
                  }
                : undefined
            }
            onLoad={(e) => {
              const img = e.currentTarget;
              setNatural({ w: img.naturalWidth, h: img.naturalHeight });
            }}
          />
        )}
        {!url && !failed && <div className="viewer-status">{t("viewer.loading")}</div>}
        {failed && <div className="viewer-status">{t("viewer.unsupported")}</div>}
      </div>

      <div className="viewer-bar">
        <span className="viewer-counter">
          {index + 1} / {photos.length}
        </span>
        <span className="viewer-filename" title={photo.path}>
          {photo.filename}
        </span>
        <span className="viewer-exif" title={exif.join(" · ")}>
          {exif.join(" · ")}
        </span>
        <span className="viewer-marks">
          {photo.rating > 0 && (
            <span className="viewer-stars" title={t("viewer.rate", { n: photo.rating })}>
              {"★".repeat(photo.rating)}
            </span>
          )}
          {photo.colorLabel && (
            <span
              className="viewer-color"
              style={{ background: COLOR_HEX[photo.colorLabel] ?? "#888" }}
            />
          )}
          {photo.flag === "pick" && <span className="viewer-flag flag-pick">{t("viewer.pick")}</span>}
          {photo.flag === "reject" && (
            <span className="viewer-flag flag-reject">{t("viewer.reject")}</span>
          )}
        </span>
        <button
          className={`btn ${autoAdvance ? "btn-active" : ""}`}
          title={t("viewer.autoAdvanceTitle")}
          onClick={() => setAutoAdvance((v) => !v)}
        >
          {t("viewer.autoAdvance")} {autoAdvance ? t("viewer.autoAdvanceOn") : t("viewer.autoAdvanceOff")}
        </button>
        <button className="btn" onClick={closeViewer} title={t("viewer.closeTitle")}>
          {t("viewer.close")}
        </button>
      </div>
    </div>
  );
}
