import { useApp } from "../lib/store";
import { useT } from "../lib/i18n";
import { COLOR_HEX } from "../lib/api";

export function FilterPanel({ onClose }: { onClose: () => void }) {
  const t = useT();
  const query = useApp((s) => s.query);
  const setQuery = useApp((s) => s.setQuery);
  const facets = useApp((s) => s.facets);
  const clearSelection = useApp((s) => s.clearSelection);

  const hasActive = Object.keys(query).length > 0;

  const set = (key: string, value: unknown) => {
    const next = { ...query };
    if (value === "" || value === undefined || value === null) {
      delete (next as Record<string, unknown>)[key];
    } else {
      (next as Record<string, unknown>)[key] = value;
    }
    void setQuery(next);
    clearSelection();
  };

  return (
    <div className="filter-panel">
      <div className="filter-row">
        <label>
          {t("filter.camera")}
          <select
            value={query.camera ?? ""}
            onChange={(e) => set("camera", e.target.value || undefined)}
          >
            <option value="">{t("filter.all")}</option>
            {facets?.cameraModels.map((f) => (
              <option key={f.value} value={f.value}>
                {f.value} ({f.count})
              </option>
            ))}
          </select>
        </label>

        <label>
          {t("filter.lens")}
          <select
            value={query.lens ?? ""}
            onChange={(e) => set("lens", e.target.value || undefined)}
          >
            <option value="">{t("filter.all")}</option>
            {facets?.lensModels.map((f) => (
              <option key={f.value} value={f.value}>
                {f.value} ({f.count})
              </option>
            ))}
          </select>
        </label>

        <label>
          {t("filter.isoMin")}
          <input
            type="number"
            placeholder={t("filter.isoPlaceholder")}
            value={query.isoMin ?? ""}
            onChange={(e) => set("isoMin", e.target.value ? Number(e.target.value) : undefined)}
          />
        </label>

        <label>
          {t("filter.focalMin")}
          <input
            type="number"
            placeholder={t("filter.focalPlaceholder")}
            value={query.focalMin ?? ""}
            onChange={(e) =>
              set("focalMin", e.target.value ? Number(e.target.value) : undefined)
            }
          />
        </label>
      </div>

      <div className="filter-row">
        <label>
          {t("filter.dateFrom")}
          <input
            type="date"
            value={query.dateFrom ?? ""}
            onChange={(e) => set("dateFrom", e.target.value || undefined)}
          />
        </label>

        <label>
          {t("filter.dateTo")}
          <input
            type="date"
            value={query.dateTo ?? ""}
            onChange={(e) => set("dateTo", e.target.value || undefined)}
          />
        </label>

        <label>
          {t("filter.minRating")}
          <select
            value={query.minRating ?? 0}
            onChange={(e) => set("minRating", Number(e.target.value) || undefined)}
          >
            <option value={0}>{t("filter.any")}</option>
            {[1, 2, 3, 4, 5].map((r) => (
              <option key={r} value={r}>
                {"★".repeat(r)}
              </option>
            ))}
          </select>
        </label>

        <label>
          {t("filter.colorLabel")}
          <select
            value={query.colorLabel ?? ""}
            onChange={(e) => set("colorLabel", e.target.value || undefined)}
          >
            <option value="">{t("filter.any")}</option>
            {Object.entries(COLOR_HEX).map(([name, hex]) => (
              <option key={name} value={name}>
                {name} ({hex})
              </option>
            ))}
          </select>
        </label>

        <label>
          {t("filter.flag")}
          <select
            value={query.flag ?? ""}
            onChange={(e) => set("flag", e.target.value || undefined)}
          >
            <option value="">{t("filter.any")}</option>
            <option value="pick">{t("filter.pick")}</option>
            <option value="reject">{t("filter.reject")}</option>
          </select>
        </label>

        <label>
          {t("filter.mediaType")}
          <select
            value={query.mediaType ?? ""}
            onChange={(e) => set("mediaType", e.target.value || undefined)}
          >
            <option value="">{t("filter.any")}</option>
            <option value="photo">{t("filter.photo")}</option>
            <option value="video">{t("filter.video")}</option>
          </select>
        </label>
      </div>

      <div className="filter-actions">
        {hasActive && (
          <button
            className="btn"
            onClick={() => {
              void setQuery({});
              clearSelection();
            }}
          >
            {t("filter.clearAll")}
          </button>
        )}
        <button className="btn" onClick={onClose}>
          {t("filter.collapse")}
        </button>
      </div>
    </div>
  );
}
