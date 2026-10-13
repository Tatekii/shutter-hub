import { useRef, useState } from "react";

type Rect = { left: number; top: number; w: number; h: number };
type Corner = "tl" | "tr" | "bl" | "br";

/// 裁剪取景框：叠加在（未裁切的）画面上，四角拖拽；可选比例锁定。
/// crop 为 [top, left, bottom, right]（0..1），坐标为旋转后空间。
export function CropOverlay({
  crop,
  aspect,
  imgRect,
  imageAspect,
  onCommit,
}: {
  crop: [number, number, number, number];
  aspect: number | null;
  imgRect: Rect;
  /** 旋转后图像的 宽/高（像素比），比例锁定时换算用 */
  imageAspect: number;
  onCommit: (crop: [number, number, number, number]) => void;
}) {
  const drag = useRef<{
    corner: Corner;
    start: [number, number];
    startCrop: [number, number, number, number];
  } | null>(null);
  const [live, setLive] = useState<[number, number, number, number] | null>(null);

  const current = live ?? crop;
  const x = imgRect.left + current[1] * imgRect.w;
  const y = imgRect.top + current[0] * imgRect.h;
  const w = (current[3] - current[1]) * imgRect.w;
  const h = (current[2] - current[0]) * imgRect.h;

  const onPointerDown = (corner: Corner) => (e: React.PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { corner, start: [e.clientX, e.clientY], startCrop: current };
    setLive(current);
  };

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    const dx = (e.clientX - drag.current.start[0]) / Math.max(1, imgRect.w);
    const dy = (e.clientY - drag.current.start[1]) / Math.max(1, imgRect.h);
    const [t0, l0, b0, r0] = drag.current.startCrop;
    let top = t0;
    let left = l0;
    let bottom = b0;
    let right = r0;
    const { corner } = drag.current;

    if (corner === "tl" || corner === "tr") top = t0 + dy;
    if (corner === "bl" || corner === "br") bottom = b0 + dy;
    if (corner === "tl" || corner === "bl") left = l0 + dx;
    if (corner === "tr" || corner === "br") right = r0 + dx;

    // 最小尺寸 5%
    top = Math.min(Math.max(top, 0), bottom - 0.05);
    bottom = Math.max(Math.min(bottom, 1), top + 0.05);
    left = Math.min(Math.max(left, 0), right - 0.05);
    right = Math.max(Math.min(right, 1), left + 0.05);

    // 比例锁定：以宽度为准推导高度（或反之），锚定对角
    if (aspect != null && aspect > 0) {
      const cw = right - left;
      const desiredCh = (cw * imageAspect) / aspect / 1; // ch = cw * W/(R*H)
      const ch = Math.min(desiredCh, 1);
      if (corner === "tl" || corner === "tr") {
        top = Math.max(0, bottom - ch);
      } else {
        bottom = Math.min(1, top + ch);
      }
    }

    setLive([top, left, bottom, right]);
  };

  const onPointerUp = () => {
    if (!drag.current) return;
    drag.current = null;
    if (live) onCommit([round4(live[0]), round4(live[1]), round4(live[2]), round4(live[3])]);
    setLive(null);
  };

  return (
    <div className="crop-layer" style={{ left: imgRect.left, top: imgRect.top, width: imgRect.w, height: imgRect.h }}>
      {/* 变暗遮罩（四块） */}
      <div className="crop-shade" style={{ left: 0, top: 0, width: imgRect.w, height: y - imgRect.top }} />
      <div className="crop-shade" style={{ left: 0, top: y - imgRect.top + h, width: imgRect.w, height: Math.max(0, imgRect.h - (y - imgRect.top) - h) }} />
      <div className="crop-shade" style={{ left: 0, top: y - imgRect.top, width: x - imgRect.left, height: h }} />
      <div className="crop-shade" style={{ left: x - imgRect.left + w, top: y - imgRect.top, width: Math.max(0, imgRect.w - (x - imgRect.left) - w), height: h }} />
      {/* 取景框 */}
      <div className="crop-frame" style={{ left: x - imgRect.left, top: y - imgRect.top, width: w, height: h }}>
        {(["tl", "tr", "bl", "br"] as Corner[]).map((corner) => (
          <div
            key={corner}
            className={`crop-handle crop-${corner}`}
            onPointerDown={onPointerDown(corner)}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
          />
        ))}
      </div>
    </div>
  );
}

function round4(v: number): number {
  return Math.round(v * 10000) / 10000;
}
