import { useApp } from "../lib/store";
import { COLOR_HEX } from "../lib/api";

export function FilterPanel({ onClose }: { onClose: () => void }) {
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
          相机
          <select
            value={query.camera ?? ""}
            onChange={(e) => set("camera", e.target.value || undefined)}
          >
            <option value="">全部</option>
            {facets?.cameraModels.map((f) => (
              <option key={f.value} value={f.value}>
                {f.value} ({f.count})
              </option>
            ))}
          </select>
        </label>

        <label>
          镜头
          <select
            value={query.lens ?? ""}
            onChange={(e) => set("lens", e.target.value || undefined)}
          >
            <option value="">全部</option>
            {facets?.lensModels.map((f) => (
              <option key={f.value} value={f.value}>
                {f.value} ({f.count})
              </option>
            ))}
          </select>
        </label>

        <label>
          ISO 不低于
          <input
            type="number"
            placeholder="如 800"
            value={query.isoMin ?? ""}
            onChange={(e) => set("isoMin", e.target.value ? Number(e.target.value) : undefined)}
          />
        </label>

        <label>
          最小焦距 (mm)
          <input
            type="number"
            placeholder="如 35"
            value={query.focalMin ?? ""}
            onChange={(e) =>
              set("focalMin", e.target.value ? Number(e.target.value) : undefined)
            }
          />
        </label>
      </div>

      <div className="filter-row">
        <label>
          拍摄日期从
          <input
            type="date"
            value={query.dateFrom ?? ""}
            onChange={(e) => set("dateFrom", e.target.value || undefined)}
          />
        </label>

        <label>
          到
          <input
            type="date"
            value={query.dateTo ?? ""}
            onChange={(e) => set("dateTo", e.target.value || undefined)}
          />
        </label>

        <label>
          最低评分
          <select
            value={query.minRating ?? 0}
            onChange={(e) => set("minRating", Number(e.target.value) || undefined)}
          >
            <option value={0}>不限</option>
            {[1, 2, 3, 4, 5].map((r) => (
              <option key={r} value={r}>
                {"★".repeat(r)}
              </option>
            ))}
          </select>
        </label>

        <label>
          色标
          <select
            value={query.colorLabel ?? ""}
            onChange={(e) => set("colorLabel", e.target.value || undefined)}
          >
            <option value="">不限</option>
            {Object.entries(COLOR_HEX).map(([name, hex]) => (
              <option key={name} value={name}>
                {name} ({hex})
              </option>
            ))}
          </select>
        </label>

        <label>
          旗标
          <select
            value={query.flag ?? ""}
            onChange={(e) => set("flag", e.target.value || undefined)}
          >
            <option value="">不限</option>
            <option value="pick">✓ 留存</option>
            <option value="reject">✕ 淘汰</option>
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
            清除全部筛选
          </button>
        )}
        <button className="btn" onClick={onClose}>
          收起
        </button>
      </div>
    </div>
  );
}
