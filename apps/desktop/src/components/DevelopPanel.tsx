import { useRef, useState } from "react";
import { useT } from "../lib/i18n";
import type { Adjustments } from "../lib/types";

export type DevelopTab = "basic" | "curve" | "hsl" | "crop";

const HSL_BAND_HUES = [0, 30, 60, 120, 180, 240, 280, 320];
const HSL_BAND_KEYS = [
  "hsl.red",
  "hsl.orange",
  "hsl.yellow",
  "hsl.green",
  "hsl.aqua",
  "hsl.blue",
  "hsl.purple",
  "hsl.magenta",
];

const CURVE_XS = [0, 0.25, 0.5, 0.75, 1];

/// 曲线点：空曲线 → 恒等 5 点。
export function curvePoints(adj: Adjustments): [number, number][] {
  if (adj.curve.length >= 2) {
    return adj.curve.map(([x, y]) => [x, y]);
  }
  return CURVE_XS.map((x) => [x, x]);
}

export function DevelopPanel({
  adj,
  tab,
  setTab,
  savedTick,
  cropAspect,
  onCropAspect,
  onPatch,
  onReset,
  onClose,
}: {
  adj: Adjustments;
  tab: DevelopTab;
  setTab: (t: DevelopTab) => void;
  savedTick: boolean;
  cropAspect: number | null;
  onCropAspect: (a: number | null) => void;
  onPatch: (patch: Partial<Adjustments>) => void;
  onReset: () => void;
  onClose: () => void;
}) {
  const t = useT();
  const [hslBand, setHslBand] = useState(0);

  const tabs: Array<[DevelopTab, string]> = [
    ["basic", t("develop.tabBasic")],
    ["curve", t("develop.tabCurve")],
    ["hsl", t("develop.tabHsl")],
    ["crop", t("develop.tabCrop")],
  ];

  return (
    <div className="develop-panel">
      <div className="develop-header">
        <span className="develop-title">🎚 {t("develop.title")}</span>
        <span className={`develop-saved ${savedTick ? "on" : ""}`}>
          {savedTick ? t("develop.saved") : ""}
        </span>
        <button className="icon-btn" aria-label={t("develop.title")} onClick={onClose}>
          ×
        </button>
      </div>

      <div className="develop-tabs">
        {tabs.map(([key, label]) => (
          <button
            key={key}
            className={`develop-tab ${tab === key ? "on" : ""}`}
            onClick={() => setTab(key)}
          >
            {label}
          </button>
        ))}
      </div>

      {tab === "basic" && (
        <>
          <DevelopSlider
            label={t("develop.exposure")}
            value={adj.exposure}
            min={-5}
            max={5}
            step={0.05}
            onChange={(v) => onPatch({ exposure: v })}
          />
          <DevelopSlider
            label={t("develop.contrast")}
            value={adj.contrast}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => onPatch({ contrast: v })}
          />
          <DevelopSlider
            label={t("develop.highlights")}
            value={adj.highlights}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => onPatch({ highlights: v })}
          />
          <DevelopSlider
            label={t("develop.shadows")}
            value={adj.shadows}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => onPatch({ shadows: v })}
          />
          <DevelopSlider
            label={t("develop.temperature")}
            value={adj.temperature}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => onPatch({ temperature: v })}
          />
          <DevelopSlider
            label={t("develop.tint")}
            value={adj.tint}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => onPatch({ tint: v })}
          />
          <DevelopSlider
            label={t("develop.saturation")}
            value={adj.saturation}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => onPatch({ saturation: v })}
          />
        </>
      )}

      {tab === "curve" && (
        <CurveEditor
          points={curvePoints(adj)}
          onChange={(p) => onPatch({ curve: p })}
        />
      )}

      {tab === "hsl" && (
        <>
          <div className="hsl-bands">
            {HSL_BAND_HUES.map((h, i) => (
              <button
                key={h}
                className={`hsl-band ${hslBand === i ? "on" : ""}`}
                style={{ background: `hsl(${h}, 70%, 50%)` }}
                title={t(HSL_BAND_KEYS[i])}
                onClick={() => setHslBand(i)}
              />
            ))}
          </div>
          <DevelopSlider
            label={t("hsl.saturation")}
            value={adj.hslSat[hslBand] ?? 0}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => {
              const next = [...adj.hslSat];
              next[hslBand] = v;
              onPatch({ hslSat: next });
            }}
          />
          <DevelopSlider
            label={t("hsl.luminance")}
            value={adj.hslLum[hslBand] ?? 0}
            min={-100}
            max={100}
            step={1}
            onChange={(v) => {
              const next = [...adj.hslLum];
              next[hslBand] = v;
              onPatch({ hslLum: next });
            }}
          />
          <div className="develop-hint">{t("hsl.hint")}</div>
        </>
      )}

      {tab === "crop" && (
        <>
          <div className="export-presets">
            <button
              className={`btn ${cropAspect === null && isFullCrop(adj) ? "btn-active" : ""}`}
              onClick={() => {
                onCropAspect(null);
                onPatch({ crop: [0, 0, 1, 1] });
              }}
            >
              {t("crop.original")}
            </button>
            <button
              className={`btn ${cropAspect === -1 ? "btn-active" : ""}`}
              onClick={() => onCropAspect(-1)}
            >
              {t("crop.free")}
            </button>
            {([["1:1", 1], ["4:5", 4 / 5], ["3:4", 3 / 4], ["16:9", 16 / 9]] as Array<[string, number]>).map(
              ([label, ratio]) => (
                <button
                  key={label}
                  className={`btn ${cropAspect === ratio ? "btn-active" : ""}`}
                  onClick={() => onCropAspect(ratio)}
                >
                  {label}
                </button>
              ),
            )}
          </div>
          <div className="export-presets">
            <button
              className="btn"
              onClick={() => {
                const rotation = (adj.rotation + 90) % 360;
                const [top, left, bottom, right] = adj.crop;
                // 顺时针 90°：新裁剪在旋转后空间
                onPatch({
                  rotation,
                  crop: [left, 1 - bottom, right, 1 - top],
                });
              }}
            >
              {t("crop.rotate")}
            </button>
            <button className="btn" onClick={() => onPatch({ crop: [0, 0, 1, 1] })}>
              {t("crop.resetRect")}
            </button>
          </div>
          <div className="develop-hint">{t("crop.hint")}</div>
        </>
      )}

      <div className="develop-actions">
        <button className="btn" onClick={onReset}>
          {t("develop.reset")}
        </button>
      </div>
      <div className="develop-hint">{t("develop.hint")}</div>
    </div>
  );
}

function isFullCrop(adj: Adjustments): boolean {
  const [t0, l, b, r] = adj.crop;
  return t0 === 0 && l === 0 && b === 1 && r === 1;
}

/// 曲线编辑器：5 个固定 x 控制点，拖拽 y；SVG 坐标 y 翻转。
function CurveEditor({
  points,
  onChange,
}: {
  points: [number, number][];
  onChange: (p: [number, number][]) => void;
}) {
  const ref = useRef<SVGSVGElement>(null);
  const [dragIndex, setDragIndex] = useState<number | null>(null);

  const toSvg = (x: number, y: number) => [x, 1 - y] as const;

  const onMove = (e: React.PointerEvent<SVGSVGElement>) => {
    if (dragIndex == null || !ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    const y = 1 - (e.clientY - rect.top) / rect.height;
    const next = points.map((p, i) =>
      i === dragIndex ? ([p[0], Math.min(1, Math.max(0, y))] as [number, number]) : p,
    );
    onChange(next);
  };

  const path = points
    .map(([x, y], i) => {
      const [sx, sy] = toSvg(x, y);
      return `${i === 0 ? "M" : "L"} ${sx.toFixed(4)} ${sy.toFixed(4)}`;
    })
    .join(" ");

  return (
    <div className="curve-editor">
      <svg
        ref={ref}
        viewBox="0 0 1 1"
        preserveAspectRatio="none"
        onPointerMove={onMove}
        onPointerUp={() => setDragIndex(null)}
        onPointerLeave={() => setDragIndex(null)}
      >
        <rect x="0" y="0" width="1" height="1" fill="rgba(127,127,127,0.12)" />
        <line x1="0" y1="1" x2="1" y2="0" stroke="rgba(127,127,127,0.5)" strokeWidth="0.006" />
        <path d={path} fill="none" stroke="var(--accent)" strokeWidth="0.012" />
        {points.map(([x, y], i) => {
          const [sx, sy] = toSvg(x, y);
          return (
            <circle
              key={i}
              cx={sx}
              cy={sy}
              r={dragIndex === i ? 0.045 : 0.035}
              fill="var(--accent)"
              stroke="#fff"
              strokeWidth="0.008"
              style={{ cursor: "ns-resize" }}
              onPointerDown={(e) => {
                e.currentTarget.setPointerCapture(e.pointerId);
                setDragIndex(i);
              }}
            />
          );
        })}
      </svg>
    </div>
  );
}

function DevelopSlider({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
}) {
  const decimals = step >= 1 ? 0 : 2;
  return (
    <label className="develop-slider" title="双击归零">
      <span className="develop-slider-label">{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        onDoubleClick={() => onChange(0)}
      />
      <span className="develop-slider-value">{value.toFixed(decimals)}</span>
    </label>
  );
}
