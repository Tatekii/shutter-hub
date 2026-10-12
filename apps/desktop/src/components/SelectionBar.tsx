import { useApp } from "../lib/store";
import { useT } from "../lib/i18n";
import { useState } from "react";

export function SelectionBar() {
  const t = useT();
  const selectedIds = useApp((s) => s.selectedIds);
  const albums = useApp((s) => s.albums);
  const clearSelection = useApp((s) => s.clearSelection);
  const addSelectedToAlbum = useApp((s) => s.addSelectedToAlbum);
  const addTagsToSelected = useApp((s) => s.addTagsToSelected);
  const [tagInput, setTagInput] = useState("");

  if (selectedIds.size === 0) return null;

  return (
    <div className="selection-bar">
      <span className="selection-count">{t("select.count", { n: selectedIds.size })}</span>
      <select
        value=""
        onChange={(e) => {
          const id = Number(e.target.value);
          if (id) void addSelectedToAlbum(id);
        }}
        title={t("select.addToAlbumTitle")}
      >
        <option value="">{t("select.addToAlbum")}</option>
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
            .map((tag) => tag.trim())
            .filter(Boolean);
          if (tags.length > 0) void addTagsToSelected(tags);
          setTagInput("");
        }}
      >
        <input
          value={tagInput}
          onChange={(e) => setTagInput(e.target.value)}
          placeholder={t("select.tagPlaceholder")}
        />
      </form>
      <button className="btn" onClick={clearSelection}>
        {t("select.cancel")}
      </button>
    </div>
  );
}
