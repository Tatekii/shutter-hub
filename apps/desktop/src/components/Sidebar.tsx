import { useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { useApp } from "../lib/store";
import type { View } from "../lib/store";
import { translate, useT } from "../lib/i18n";
import { COLOR_HEX } from "../lib/api";

export function Sidebar() {
  const t = useT();
  const folders = useApp((s) => s.folders);
  const albums = useApp((s) => s.albums);
  const tags = useApp((s) => s.tags);
  const view = useApp((s) => s.view);
  const setView = useApp((s) => s.setView);
  const rescanFolder = useApp((s) => s.rescanFolder);
  const removeFolder = useApp((s) => s.removeFolder);
  const createAlbum = useApp((s) => s.createAlbum);
  const deleteAlbum = useApp((s) => s.deleteAlbum);
  const deleteTag = useApp((s) => s.deleteTag);
  const addFolder = useApp((s) => s.addFolder);

  const [albumInput, setAlbumInput] = useState(false);
  const [albumName, setAlbumName] = useState("");
  const [updateMsg, setUpdateMsg] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);

  async function checkUpdates() {
    setChecking(true);
    setUpdateMsg(null);
    try {
      const update = await check();
      if (!update) {
        setUpdateMsg(t("sidebar.upToDate"));
        return;
      }
      setUpdateMsg(t("sidebar.foundUpdate", { v: update.version }));
      await update.downloadAndInstall();
      setUpdateMsg(t("sidebar.installedUpdate", { v: update.version }));
    } catch (e) {
      setUpdateMsg(t("sidebar.updateFailed", { e: String(e) }));
    } finally {
      setChecking(false);
    }
  }

  return (
    <aside className="sidebar">
      <section className="sidebar-section">
        <div className="sidebar-title">
          <span>{t("sidebar.folders")}</span>
          <button className="icon-btn" title={t("sidebar.addFolder")} onClick={() => void addFolder()}>
            +
          </button>
        </div>
        {folders.length === 0 && (
          <div className="sidebar-hint">{t("sidebar.noFolders")}</div>
        )}
        {folders.map((f) => (
          <div
            key={f.id}
            className={`sidebar-item ${view.kind === "library" ? "active" : ""}`}
            onClick={() => setView({ kind: "library" })}
            title={f.path}
          >
            <span className="sidebar-item-name">{folderName(f.path)}</span>
            <span className="sidebar-item-count">{f.photoCount}</span>
            <span className="sidebar-item-actions">
              <button
                className="icon-btn tiny"
                title={t("sidebar.rescan")}
                onClick={(e) => {
                  e.stopPropagation();
                  void rescanFolder(f.id);
                }}
              >
                ↻
              </button>
              <button
                className="icon-btn tiny danger"
                title={t("sidebar.removeFolder")}
                onClick={(e) => {
                  e.stopPropagation();
                  void removeFolder(f.id);
                }}
              >
                ×
              </button>
            </span>
          </div>
        ))}
      </section>

      <section className="sidebar-section">
        <div className="sidebar-title">
          <span>{t("sidebar.albums")}</span>
          <button
            className="icon-btn"
            title={t("sidebar.newAlbum")}
            onClick={() => setAlbumInput(true)}
          >
            +
          </button>
        </div>
        {albumInput && (
          <form
            className="sidebar-inline-form"
            onSubmit={(e) => {
              e.preventDefault();
              if (albumName.trim()) void createAlbum(albumName.trim());
              setAlbumName("");
              setAlbumInput(false);
            }}
          >
            <input
              autoFocus
              value={albumName}
              onChange={(e) => setAlbumName(e.target.value)}
              placeholder={t("sidebar.albumName")}
            />
          </form>
        )}
        {albums.map((a) => (
          <div
            key={a.id}
            className={`sidebar-item ${
              view.kind === "album" && view.id === a.id ? "active" : ""
            }`}
            onClick={() => setView({ kind: "album", id: a.id, name: a.name })}
            title={a.name}
          >
            <span className="sidebar-item-name">▣ {a.name}</span>
            <span className="sidebar-item-count">{a.photoCount}</span>
            <span className="sidebar-item-actions">
              <button
                className="icon-btn tiny danger"
                title={t("sidebar.deleteAlbum")}
                onClick={(e) => {
                  e.stopPropagation();
                  void deleteAlbum(a.id);
                }}
              >
                ×
              </button>
            </span>
          </div>
        ))}
      </section>

      <section className="sidebar-section">
        <div className="sidebar-title">
          <span>{t("sidebar.tags")}</span>
        </div>
        {tags.map((tag) => (
          <div
            key={tag.id}
            className={`sidebar-item ${
              view.kind === "tag" && view.id === tag.id ? "active" : ""
            }`}
            onClick={() => setView({ kind: "tag", id: tag.id, name: tag.name })}
            title={tag.name}
          >
            <span className="sidebar-item-name"># {tag.name}</span>
            <span className="sidebar-item-count">{tag.photoCount}</span>
            <span className="sidebar-item-actions">
              <button
                className="icon-btn tiny danger"
                title={t("sidebar.deleteTag")}
                onClick={(e) => {
                  e.stopPropagation();
                  void deleteTag(tag.id);
                }}
              >
                ×
              </button>
            </span>
          </div>
        ))}
      </section>

      <div className="sidebar-footer">
        <button
          className="btn btn-ghost sidebar-dupe-btn"
          onClick={() => useApp.getState().setView({ kind: "dupes" })}
        >
          {t("sidebar.findDupes")}
        </button>
        <button
          className="btn sidebar-dupe-btn"
          onClick={() => void checkUpdates()}
          disabled={checking}
        >
          {checking ? t("sidebar.checking") : t("sidebar.checkUpdates")}
        </button>
        {updateMsg && <div className="sidebar-update-msg">{updateMsg}</div>}
        <div className="color-key" title={t("sidebar.colorKeyTitle")}>
          {Object.entries(COLOR_HEX).map(([name, hex]) => (
            <span key={name} className="color-dot" style={{ background: hex }} />
          ))}
        </div>
      </div>
    </aside>
  );
}

function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function viewTitle(view: View, lang: "zh" | "en"): string {
  switch (view.kind) {
    case "library":
      return translate(lang, "sidebar.viewLibrary");
    case "album":
      return translate(lang, "sidebar.viewAlbum", { name: view.name });
    case "tag":
      return translate(lang, "sidebar.viewTag", { name: view.name });
    case "dupes":
      return translate(lang, "sidebar.viewDupes");
  }
}
