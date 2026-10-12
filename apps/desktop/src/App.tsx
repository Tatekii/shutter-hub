import { useCallback, useEffect, useState } from "react";
import "./App.css";
import { CardBrowser } from "./components/CardBrowser";
import { DupeView } from "./components/DupeView";
import { EmptyState } from "./components/EmptyState";
import { HeaderBar } from "./components/HeaderBar";
import { HelpOverlay } from "./components/HelpOverlay";
import { Inspector } from "./components/Inspector";
import { PhotoGrid } from "./components/PhotoGrid";
import { ScanBar } from "./components/ScanBar";
import { SelectionBar } from "./components/SelectionBar";
import { Sidebar } from "./components/Sidebar";
import { Viewer } from "./components/Viewer";
import * as api from "./lib/api";
import { useT } from "./lib/i18n";
import { useApp } from "./lib/store";

function App() {
  const t = useT();
  const init = useApp((s) => s.init);
  const loading = useApp((s) => s.loading);
  const error = useApp((s) => s.error);
  const setError = useApp((s) => s.setError);
  const photos = useApp((s) => s.photos);
  const view = useApp((s) => s.view);
  const scanning = useApp((s) => s.scanning);
  const setScanning = useApp((s) => s.setScanning);
  const hashing = useApp((s) => s.hashing);
  const setHashing = useApp((s) => s.setHashing);
  const setIngestProgress = useApp((s) => s.setIngestProgress);
  const volumes = useApp((s) => s.volumes);
  const cardVolume = useApp((s) => s.cardVolume);
  const openVolume = useApp((s) => s.openVolume);
  const refreshVolumes = useApp((s) => s.refreshVolumes);
  const addFolder = useApp((s) => s.addFolder);
  const [showHelp, setShowHelp] = useState(false);
  const [showVolumePicker, setShowVolumePicker] = useState(false);

  useEffect(() => {
    void init();
  }, [init]);

  useEffect(() => {
    const unsubscribers = [
      api.onScanProgress(setScanning),
      api.onScanDone(() => {
        setScanning(null);
        void useApp.getState().refresh();
      }),
      api.onScanError((msg) => {
        setScanning(null);
        setError(msg);
      }),
      api.onHashProgress(setHashing),
      api.onHashDone(() => setHashing(null)),
      api.onIngestProgress(setIngestProgress),
      api.onIngestDone(() => {
        useApp.getState().setIngestProgress(null);
      }),
    ];
    const all = Promise.all(unsubscribers);
    return () => {
      void all.then((fns) => fns.forEach((fn) => fn()));
    };
  }, [setScanning, setError, setHashing, setIngestProgress]);

  useEffect(() => {
    const timer = setInterval(() => void refreshVolumes(), 15000);
    return () => clearInterval(timer);
  }, [refreshVolumes]);

  const onKey = useCallback((e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    if (
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT"
    ) {
      if (e.key === "Escape") (target as HTMLInputElement).blur();
      return;
    }
    const s = useApp.getState();
    // 看图模式打开时按键全部交给 Viewer 处理
    if (s.viewerId != null) return;
    if (e.key === "?") {
      setShowHelp((v) => !v);
      return;
    }
    if (e.key === "Escape") {
      setShowHelp(false);
      if (s.cardVolume) s.closeVolume();
      else s.clearSelection();
      return;
    }
    if (e.key === "Enter" && s.inspectorId != null) {
      s.openViewer(s.inspectorId);
      return;
    }
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      s.selectAll();
      return;
    }
    if (e.key === "ArrowLeft") {
      s.moveSelection(-1);
      return;
    }
    if (e.key === "ArrowRight") {
      s.moveSelection(1);
      return;
    }
    const n = parseInt(e.key, 10);
    if (n >= 1 && n <= 5) {
      void s.rateSelected(n);
      return;
    }
    if (e.key === "0") {
      void s.rateSelected(0);
      return;
    }
    if (e.key === "6") {
      void s.colorSelected("red");
      return;
    }
    if (e.key === "7") {
      void s.colorSelected("yellow");
      return;
    }
    if (e.key === "8") {
      void s.colorSelected("green");
      return;
    }
    if (e.key === "9") {
      void s.colorSelected("blue");
      return;
    }
    if (e.key.toLowerCase() === "p") {
      void s.flagSelected("pick");
      return;
    }
    if (e.key.toLowerCase() === "x") {
      void s.flagSelected("reject");
      return;
    }
    if (e.key.toLowerCase() === "u") {
      void s.flagSelected("");
      return;
    }
  }, []);

  useEffect(() => {
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onKey]);

  const mainContent = () => {
    if (loading) return null;
    if (view.kind === "dupes") return <DupeView />;
    if (photos.length === 0) {
      return <EmptyState onAdd={() => void addFolder()} />;
    }
    return <PhotoGrid photos={photos} />;
  };

  return (
    <div className="app">
      <HeaderBar onShowHelp={() => setShowHelp(true)} />

      <div className="app-body">
        <Sidebar />
        <main className="app-main">
          {error && (
            <div className="error-banner">
              <span>{error}</span>
              <button className="chip-action" onClick={() => setError(null)}>
                ×
              </button>
            </div>
          )}
          {mainContent()}
        </main>
        <Inspector />
      </div>

      <SelectionBar />

      {volumes.length > 0 && !cardVolume && (
        <div className="volume-banner">
          <button
            className="btn btn-primary"
            onClick={() => setShowVolumePicker(!showVolumePicker)}
          >
            {t("app.volumeDetected", { n: volumes.length })}
          </button>
          {showVolumePicker && (
            <div className="volume-picker">
              {volumes.map((v) => (
                <button
                  key={v.path}
                  className="volume-item"
                  onClick={() => {
                    setShowVolumePicker(false);
                    void openVolume(v);
                  }}
                >
                  <span className="volume-name">{v.name}</span>
                  <span className="volume-path">{v.path}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {scanning && (
        <footer className="app-footer">
          <ScanBar progress={scanning} />
        </footer>
      )}

      {hashing && (
        <footer className="app-footer">
          <div className="scan-bar">
            <div className="scan-bar-track">
              <div
                className="scan-bar-fill"
                style={{
                  width: `${hashing.total > 0 ? (hashing.processed / hashing.total) * 100 : 0}%`,
                }}
              />
            </div>
            <span className="scan-bar-text">
              {t("dupe.hashProgress", {
                done: hashing.processed,
                total: hashing.total,
                current: hashing.current ?? "",
              })}
            </span>
          </div>
        </footer>
      )}

      {cardVolume && <CardBrowser />}
      {showHelp && <HelpOverlay onClose={() => setShowHelp(false)} />}
      <Viewer />
    </div>
  );
}

export default App;
