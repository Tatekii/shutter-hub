import { useState } from "react";
import { COLOR_HEX, formatBytes, formatDuration, thumbUrl } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";

export function Inspector() {
  const t = useT();
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
  const unknown = t("inspector.unknown");

  const rows: Array<[string, string]> = [];
  if (photo.mediaType === "video") {
    rows.push([t("inspector.rowType"), t("filter.video")]);
  }
  rows.push(
    [t("inspector.rowFilename"), photo.filename],
    [t("inspector.rowTaken"), photo.takenAt ?? unknown],
    [t("inspector.rowDims"), photo.width ? `${photo.width} × ${photo.height}` : unknown],
  );
  if (photo.mediaType === "video" && photo.durationMs != null) {
    rows.push([t("inspector.rowDuration"), formatDuration(photo.durationMs)]);
  }
  if (photo.liveMotion) {
    rows.push([t("inspector.rowLive"), photo.liveMotion]);
  }
  rows.push(
    [t("inspector.rowSize"), formatBytes(photo.size)],
    [
      t("inspector.rowCamera"),
      [photo.cameraMake, photo.cameraModel].filter(Boolean).join(" ") || unknown,
    ],
    [t("inspector.rowLens"), photo.lensModel ?? unknown],
    [t("inspector.rowFocal"), photo.focalLength != null ? `${photo.focalLength}mm` : unknown],
    [t("inspector.rowAperture"), photo.aperture != null ? `f/${photo.aperture}` : unknown],
    [t("inspector.rowShutter"), photo.shutterSpeed ?? unknown],
    [t("inspector.rowIso"), photo.iso?.toString() ?? unknown],
    [t("inspector.rowPath"), photo.path],
  );

  return (
    <aside className="inspector">
      <div className="inspector-header">
        <span>{t("inspector.title")}</span>
        <button className="icon-btn" title={t("inspector.close")} onClick={() => setInspector(null)}>
          ×
        </button>
      </div>

      <div className="inspector-preview">
        {url ? (
          <img src={url} alt={photo.filename} draggable={false} />
        ) : (
          <div className="photo-card-placeholder">{t("inspector.noPreview")}</div>
        )}
      </div>

      <div className="inspector-ratings">
        <div className="star-row">
          {[1, 2, 3, 4, 5].map((r) => (
            <button
              key={r}
              className={`star ${photo.rating >= r ? "on" : ""}`}
              title={t("inspector.rate", { n: r })}
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
              title={t("inspector.colorToggle", { name: t(`color.${name}`) })}
              onClick={() => void colorSelected(photo.colorLabel === name ? "" : name)}
            />
          ))}
          <button
            className={`flag-pick ${photo.flag === "pick" ? "on-pick" : ""}`}
            title={t("inspector.pick")}
            onClick={() => void flagSelected(photo.flag === "pick" ? "" : "pick")}
          >
            ✓ {t("inspector.pickLabel")}
          </button>
          <button
            className={`flag-pick ${photo.flag === "reject" ? "on-reject" : ""}`}
            title={t("inspector.reject")}
            onClick={() => void flagSelected(photo.flag === "reject" ? "" : "reject")}
          >
            ✕ {t("inspector.rejectLabel")}
          </button>
        </div>
      </div>

      <form
        className="inspector-tags-form"
        onSubmit={(e) => {
          e.preventDefault();
          const tags = tagInput
            .split(/[,，\s]+/)
            .map((tag) => tag.trim())
            .filter(Boolean);
          if (tags.length > 0) void addTagsToSelected(tags);
          setTagInput("");
        }}
      >
        <input
          value={tagInput}
          onChange={(e) => setTagInput(e.target.value)}
          placeholder={t("inspector.tagPlaceholder")}
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
