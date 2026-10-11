import { useCallback, useEffect, useState } from "react";
import "./App.css";
import { EmptyState } from "./components/EmptyState";
import { PhotoGrid } from "./components/PhotoGrid";
import { ScanBar } from "./components/ScanBar";
import {
  addFolder,
  getStats,
  listFolders,
  listPhotos,
  onScanDone,
  onScanError,
  onScanProgress,
  removeFolder,
  rescanFolder,
} from "./lib/api";
import type {
  FolderDto,
  LibraryStats,
  PhotoDto,
  ScanProgress,
} from "./lib/types";

function App() {
  const [folders, setFolders] = useState<FolderDto[]>([]);
  const [photos, setPhotos] = useState<PhotoDto[]>([]);
  const [stats, setStats] = useState<LibraryStats>({ photoCount: 0, folderCount: 0 });
  const [scanning, setScanning] = useState<ScanProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const [f, p, s] = await Promise.all([listFolders(), listPhotos(), getStats()]);
      setFolders(f);
      setPhotos(p);
      setStats(s);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [
      onScanProgress((p) => setScanning(p)),
      onScanDone((s) => {
        setScanning(null);
        console.log(
          `扫描完成：新增 ${s.added}，更新 ${s.updated}，跳过 ${s.skipped}`,
        );
        void refresh();
      }),
      onScanError((msg) => {
        setScanning(null);
        setError(msg);
      }),
    ];
    return () => {
      for (const u of unlisteners) {
        void u.then((fn) => fn());
      }
    };
  }, [refresh]);

  const handleAddFolder = useCallback(async () => {
    setError(null);
    try {
      await addFolder();
      void refresh();
    } catch (e) {
      setError(String(e));
    }
  }, [refresh]);

  const handleRescan = useCallback(
    async (id: number) => {
      setError(null);
      try {
        await rescanFolder(id);
        void refresh();
      } catch (e) {
        setError(String(e));
      }
    },
    [refresh],
  );

  const handleRemove = useCallback(
    async (id: number) => {
      setError(null);
      try {
        await removeFolder(id);
        void refresh();
      } catch (e) {
        setError(String(e));
      }
    },
    [refresh],
  );

  return (
    <div className="app">
      <header className="app-header">
        <div className="app-title">
          <span className="app-logo">◉</span>
          <span>Shutter Hub</span>
        </div>
        <div className="folder-chips">
          {folders.map((f) => (
            <span key={f.id} className="folder-chip" title={f.path}>
              {folderName(f.path)}
              <span className="folder-chip-count">{f.photoCount}</span>
              <button
                className="chip-action"
                title="重新扫描"
                onClick={() => void handleRescan(f.id)}
              >
                ↻
              </button>
              <button
                className="chip-action chip-action-danger"
                title="移除（不删除文件）"
                onClick={() => void handleRemove(f.id)}
              >
                ×
              </button>
            </span>
          ))}
          <button className="btn btn-ghost btn-add" onClick={() => void handleAddFolder()}>
            + 添加文件夹
          </button>
        </div>
        <div className="app-stats">
          {stats.photoCount} 张照片 · {stats.folderCount} 个文件夹
        </div>
      </header>

      <main className="app-main">
        {error && (
          <div className="error-banner">
            <span>{error}</span>
            <button className="chip-action" onClick={() => setError(null)}>
              ×
            </button>
          </div>
        )}
        {loading ? null : photos.length === 0 ? (
          <EmptyState onAdd={() => void handleAddFolder()} />
        ) : (
          <PhotoGrid photos={photos} />
        )}
      </main>

      {scanning && (
        <footer className="app-footer">
          <ScanBar progress={scanning} />
        </footer>
      )}
    </div>
  );
}

function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export default App;
