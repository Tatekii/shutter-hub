import { useT } from "../lib/i18n";
import type { ScanProgress } from "../lib/types";

export function ScanBar({ progress }: { progress: ScanProgress }) {
  const t = useT();
  const percent =
    progress.total > 0 ? Math.round((progress.processed / progress.total) * 100) : 0;

  return (
    <div className="scan-bar">
      <div className="scan-bar-track">
        <div className="scan-bar-fill" style={{ width: `${percent}%` }} />
      </div>
      <span className="scan-bar-text">
        {t("scan.progress", { done: progress.processed, total: progress.total })}
        {progress.current ? ` · ${progress.current}` : ""}
      </span>
    </div>
  );
}
