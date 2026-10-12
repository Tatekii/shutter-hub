import { useEffect, useState } from "react";
import { useApp } from "../lib/store";
import { useT } from "../lib/i18n";
import { viewTitle } from "./Sidebar";
import { FilterPanel } from "./FilterPanel";
import { AiPanel } from "./AiPanel";

export function HeaderBar({ onShowHelp }: { onShowHelp: () => void }) {
  const t = useT();
  const query = useApp((s) => s.query);
  const setQuery = useApp((s) => s.setQuery);
  const stats = useApp((s) => s.stats);
  const view = useApp((s) => s.view);
  const lang = useApp((s) => s.lang);
  const setLang = useApp((s) => s.setLang);
  const addFolder = useApp((s) => s.addFolder);
  const computeHashes = useApp((s) => s.computeHashes);
  const hashBusy = useApp((s) => s.hashBusy);
  const searchMode = useApp((s) => s.searchMode);
  const setSearchMode = useApp((s) => s.setSearchMode);
  const semanticSearch = useApp((s) => s.semanticSearch);
  const clearAiResults = useApp((s) => s.clearAiResults);
  const aiResults = useApp((s) => s.aiResults);
  const [filterOpen, setFilterOpen] = useState(false);
  const [aiOpen, setAiOpen] = useState(false);
  const [search, setSearch] = useState(query.q ?? "");
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (!dirty) setSearch(query.q ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query.q]);

  const activeFilterCount = Object.keys(query).filter((k) => k !== "q").length;

  const runSearch = () => {
    if (searchMode === "ai") {
      void semanticSearch(search);
    } else {
      void setQuery({ q: search || undefined });
    }
  };

  return (
    <>
      <header className="app-header">
        <div className="app-title">
          <span className="app-logo">◉</span>
          <span>Shutter Hub</span>
          <span className="view-title">{viewTitle(view, lang)}</span>
        </div>

        <div className="search-box">
          <button
            className={`search-mode ${searchMode === "ai" ? "on" : ""}`}
            title={searchMode === "ai" ? t("ai.semanticSearch") : t("ai.keywordSearch")}
            onClick={() => {
              setSearchMode(searchMode === "ai" ? "keyword" : "ai");
              setDirty(true);
            }}
          >
            {searchMode === "ai" ? "AI" : "⌕"}
          </button>
          <input
            value={search}
            placeholder={t("header.searchPlaceholder")}
            onChange={(e) => {
              setSearch(e.target.value);
              setDirty(true);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                runSearch();
                setDirty(false);
              }
            }}
          />
          {dirty && (
            <button
              className="icon-btn"
              title={t("header.clearSearch")}
              onClick={() => {
                setSearch("");
                setDirty(false);
                clearAiResults();
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
          title={t("header.filters")}
        >
          {t("header.filterBtn")}
          {activeFilterCount > 0 ? ` (${activeFilterCount})` : ""}
        </button>

        <button
          className={`btn ${aiOpen ? "btn-active" : ""} ${aiResults ? "btn-active" : ""}`}
          onClick={() => setAiOpen(!aiOpen)}
          title={t("ai.title")}
        >
          ✦ {t("header.ai")}
        </button>

        <button className="btn" onClick={() => void computeHashes()} disabled={hashBusy}>
          {hashBusy ? t("header.hashing") : t("header.scanDupes")}
        </button>

        <button className="btn btn-ghost" onClick={() => void addFolder()}>
          {t("header.addFolder")}
        </button>

        <button
          className="btn btn-ghost"
          title={lang === "zh" ? "Switch to English" : "切换为中文"}
          onClick={() => setLang(lang === "zh" ? "en" : "zh")}
        >
          {lang === "zh" ? "EN" : "中"}
        </button>

        <button className="icon-btn" title={t("header.helpTitle")} onClick={onShowHelp}>
          ?
        </button>

        <div className="app-stats">
          {t("header.stats", { n: stats.photoCount, m: stats.folderCount })}
        </div>
      </header>
      {filterOpen && <FilterPanel onClose={() => setFilterOpen(false)} />}
      {aiOpen && <AiPanel onClose={() => setAiOpen(false)} />}
    </>
  );
}
