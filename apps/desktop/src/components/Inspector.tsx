import { useState } from "react";
import { COLOR_HEX, COLOR_LABELS, formatBytes, thumbUrl } from "../lib/api";
import { useApp } from "../lib/store";

export function Inspector() {
  const inspectorId = useApp((s) => s.inspectorId);
  const photos = useApp((s) => s.photos);
  const setInspector = useApp((s) => s.setInspector);
  const rateSelected = useApp((s) => s.rateSelected);
  const colorSelected = useApp((s) => s.colorSelected);
  const flagSelected = useApp((s) => s.flagSelected);
  const addTagsToSelected = useApp((s) => s.addTagsToSelected);
  const [tagInput, setTagInput] = useState("");

  const photo = photos.find((p) => p.id === inspectorId);
  if (!photo) return null;

  const url = thumbUrl(photo.thumbPath);

  const rows: Array<[string, string]> = [
    ["文件名", photo.filename],
    ["拍摄时间", photo.takenAt ?? "未知"],
    ["尺寸", photo.width ? `${photo.width} × ${photo.height}` : "未知"],
    ["文件大小", formatBytes(photo.size)],
    ["相机", [photo.cameraMake, photo.cameraModel].filter(Boolean).join(" ") || "未知"],
    ["镜头", photo.lensModel ?? "未知"],
    ["焦距", photo.focalLength != null ? `${photo.focalLength}mm` : "未知"],
    ["光圈", photo.aperture != null ? `f/${photo.aperture}` : "未知"],
    ["快门", photo.shutterSpeed ?? "未知"],
    ["ISO", photo.iso?.toString() ?? "未知"],
    ["路径", photo.path],
  ];

  return (
    <aside className="inspector">
      <div className="inspector-header">
        <span>照片详情</span>
        <button className="icon-btn" title="关闭 (Esc)" onClick={() => setInspector(null)}>
          ×
        </button>
      </div>

      <div className="inspector-preview">
        {url ? (
          <img src={url} alt={photo.filename} draggable={false} />
        ) : (
          <div className="photo-card-placeholder">无预览</div>
        )}
      </div>

      <div className="inspector-ratings">
        <div className="star-row">
          {[1, 2, 3, 4, 5].map((r) => (
            <button
              key={r}
              className={`star ${photo.rating >= r ? "on" : ""}`}
              title={`评分 ${r}`}
              onClick={() => void rateSelected(photo.rating === r ? 0 : r)}
            >
              ★
            </button>
          ))}
        </div>
        <div className="label-row">
          {Object.entries(COLOR_HEX).map(([name, hex]) => (
            <button
              key={name}
              className={`color-pick ${photo.colorLabel === name ? "on" : ""}`}
              style={{ background: hex }}
              title={`色标 ${COLOR_LABELS[name]}（再次点击取消）`}
              onClick={() => void colorSelected(photo.colorLabel === name ? "" : name)}
            />
          ))}
          <button
            className={`flag-pick ${photo.flag === "pick" ? "on-pick" : ""}`}
            title="留存 (P)"
            onClick={() => void flagSelected(photo.flag === "pick" ? "" : "pick")}
          >
            ✓ 留存
          </button>
          <button
            className={`flag-pick ${photo.flag === "reject" ? "on-reject" : ""}`}
            title="淘汰 (X)"
            onClick={() => void flagSelected(photo.flag === "reject" ? "" : "reject")}
          >
            ✕ 淘汰
          </button>
        </div>
      </div>

      <form
        className="inspector-tags-form"
        onSubmit={(e) => {
          e.preventDefault();
          const tags = tagInput
            .split(/[,，\s]+/)
            .map((t) => t.trim())
            .filter(Boolean);
          if (tags.length > 0) void addTagsToSelected(tags);
          setTagInput("");
        }}
      >
        <input
          value={tagInput}
          onChange={(e) => setTagInput(e.target.value)}
          placeholder="添加标签（逗号分隔）"
        />
      </form>

      <table className="exif-table">
        <tbody>
          {rows.map(([k, v]) => (
            <tr key={k}>
              <td className="exif-key">{k}</td>
              <td className="exif-value" title={v}>
                {v}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </aside>
  );
}
