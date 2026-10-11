export function EmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="empty-state">
      <svg width="72" height="72" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1">
        <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z" />
        <circle cx="12" cy="13" r="3.5" />
      </svg>
      <h2>开始整理你的素材</h2>
      <p>添加一个存放照片的文件夹，Shutter Hub 会自动扫描并建立索引。</p>
      <button className="btn btn-primary" onClick={onAdd}>
        添加文件夹
      </button>
    </div>
  );
}
