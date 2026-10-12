import { useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { useApp } from "../lib/store";
import type { View } from "../lib/store";
import { COLOR_HEX } from "../lib/api";

export function Sidebar() {
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
        setUpdateMsg("当前已是最新版本");
        return;
      }
      setUpdateMsg(`发现新版本 ${update.version}，正在下载…`);
      await update.downloadAndInstall();
      setUpdateMsg(`已安装 ${update.version}，重启应用后生效`);
    } catch (e) {
      setUpdateMsg(`检查更新失败：${String(e)}`);
    } finally {
      setChecking(false);
    }
  }

  return (
    <aside className="sidebar">
      <section className="sidebar-section">
        <div className="sidebar-title">
          <span>文件夹</span>
          <button className="icon-btn" title="添加文件夹" onClick={() => void addFolder()}>
            +
          </button>
        </div>
        {folders.length === 0 && <div className="sidebar-hint">还没有文件夹</div>}
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
                title="重新扫描"
                onClick={(e) => {
                  e.stopPropagation();
                  void rescanFolder(f.id);
                }}
              >
                ↻
              </button>
              <button
                className="icon-btn tiny danger"
                title="移除（不删除文件）"
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
          <span>相册</span>
          <button
            className="icon-btn"
            title="新建相册"
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
              placeholder="相册名称"
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
                title="删除相册（不删除照片）"
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
          <span>标签</span>
        </div>
        {tags.map((t) => (
          <div
            key={t.id}
            className={`sidebar-item ${
              view.kind === "tag" && view.id === t.id ? "active" : ""
            }`}
            onClick={() => setView({ kind: "tag", id: t.id, name: t.name })}
            title={t.name}
          >
            <span className="sidebar-item-name"># {t.name}</span>
            <span className="sidebar-item-count">{t.photoCount}</span>
            <span className="sidebar-item-actions">
              <button
                className="icon-btn tiny danger"
                title="删除标签"
                onClick={(e) => {
                  e.stopPropagation();
                  void deleteTag(t.id);
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
          查找重复照片
        </button>
        <button
          className="btn sidebar-dupe-btn"
          onClick={() => void checkUpdates()}
          disabled={checking}
        >
          {checking ? "检查中…" : "检查更新"}
        </button>
        {updateMsg && <div className="sidebar-update-msg">{updateMsg}</div>}
        <div className="color-key" title="色标快捷键 6-9">
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

export function viewTitle(view: View): string {
  switch (view.kind) {
    case "library":
      return "资料库";
    case "album":
      return `相册 · ${view.name}`;
    case "tag":
      return `标签 · #${view.name}`;
    case "dupes":
      return "重复照片";
  }
}
