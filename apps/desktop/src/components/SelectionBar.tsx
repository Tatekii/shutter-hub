import { useApp } from "../lib/store";
import { useState } from "react";

export function SelectionBar() {
  const selectedIds = useApp((s) => s.selectedIds);
  const albums = useApp((s) => s.albums);
  const clearSelection = useApp((s) => s.clearSelection);
  const addSelectedToAlbum = useApp((s) => s.addSelectedToAlbum);
  const addTagsToSelected = useApp((s) => s.addTagsToSelected);
  const [tagInput, setTagInput] = useState("");

  if (selectedIds.size === 0) return null;

  return (
    <div className="selection-bar">
      <span className="selection-count">已选 {selectedIds.size} 张</span>
      <select
        value=""
        onChange={(e) => {
          const id = Number(e.target.value);
          if (id) void addSelectedToAlbum(id);
        }}
        title="加入相册"
      >
        <option value="">加入相册…</option>
        {albums.map((a) => (
          <option key={a.id} value={a.id}>
            {a.name}
          </option>
        ))}
      </select>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const tags = tagInput
            .split(/[,，\s]+/)
            .map((t) => t.trim())
            .filter(Boolean);
          if (tags.length > 0) void addTagsToSelected(tags);
          setTagInput("");
        }}
      >
        <input
          value={tagInput}
          onChange={(e) => setTagInput(e.target.value)}
          placeholder="添加标签（逗号分隔）"
        />
      </form>
      <button className="btn" onClick={clearSelection}>
        取消选择
      </button>
    </div>
  );
}
