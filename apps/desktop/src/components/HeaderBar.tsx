import { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { viewTitle } from "./Sidebar";
import { FilterPanel } from "./FilterPanel";

export function HeaderBar({ onShowHelp }: { onShowHelp: () => void }) {
  const query = useApp((s) => s.query);
  const setQuery = useApp((s) => s.setQuery);
  const stats = useApp((s) => s.stats);
  const view = useApp((s) => s.view);
  const addFolder = useApp((s) => s.addFolder);
  const computeHashes = useApp((s) => s.computeHashes);
  const hashBusy = useApp((s) => s.hashBusy);
  const [filterOpen, setFilterOpen] = useState(false);
  const [search, setSearch] = useState(query.q ?? "");
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (!dirty) setSearch(query.q ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query.q]);

  const activeFilterCount = Object.keys(query).filter((k) => k !== "q").length;

  return (
    <>
      <header className="app-header">
        <div className="app-title">
          <span className="app-logo">◉</span>
          <span>Shutter Hub</span>
          <span className="view-title">{viewTitle(view)}</span>
        </div>

        <div className="search-box">
          <input
            value={search}
            placeholder="搜索文件名、路径、相机、镜头…"
            onChange={(e) => {
              setSearch(e.target.value);
              setDirty(true);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                void setQuery({ q: search || undefined });
                setDirty(false);
              }
            }}
          />
          {dirty && (
            <button
              className="icon-btn"
              title="清空搜索"
              onClick={() => {
                setSearch("");
                setDirty(false);
                void setQuery({ q: undefined });
              }}
            >
              ×
            </button>
          )}
        </div>

        <button
          className={`btn ${activeFilterCount > 0 ? "btn-active" : ""}`}
          onClick={() => setFilterOpen(!filterOpen)}
          title="筛选条件"
        >
          ⚙ 筛选{activeFilterCount > 0 ? ` (${activeFilterCount})` : ""}
        </button>

        <button className="btn" onClick={() => void computeHashes()} disabled={hashBusy}>
          {hashBusy ? "哈希计算中…" : "扫描重复"}
        </button>

        <button className="btn btn-ghost" onClick={() => void addFolder()}>
          + 添加文件夹
        </button>

        <button className="icon-btn" title="键盘快捷键（?）" onClick={onShowHelp}>
          ?
        </button>

        <div className="app-stats">
          {stats.photoCount} 张 · {stats.folderCount} 个文件夹
        </div>
      </header>
      {filterOpen && <FilterPanel onClose={() => setFilterOpen(false)} />}
    </>
  );
}
