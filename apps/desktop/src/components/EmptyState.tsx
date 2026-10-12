import { useT } from "../lib/i18n";

export function EmptyState({ onAdd }: { onAdd: () => void }) {
  const t = useT();
  return (
    <div className="empty-state">
      <svg width="72" height="72" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1">
        <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z" />
        <circle cx="12" cy="13" r="3.5" />
      </svg>
      <h2>{t("empty.title")}</h2>
      <p>{t("empty.body")}</p>
      <button className="btn btn-primary" onClick={onAdd}>
        {t("empty.addFolder")}
      </button>
    </div>
  );
}
