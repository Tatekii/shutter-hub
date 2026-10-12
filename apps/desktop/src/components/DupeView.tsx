import { useEffect, useState } from "react";
import { formatBytes, thumbUrl } from "../lib/api";
import { useT } from "../lib/i18n";
import { useApp } from "../lib/store";
import type { DupeGroup, SimilarPair } from "../lib/types";
import type { PhotoDto } from "../lib/types";

export function DupeView() {
  const t = useT();
  const dupeGroups = useApp((s) => s.dupeGroups);
  const similarPairs = useApp((s) => s.similarPairs);
  const dupeTab = useApp((s) => s.dupeTab);
  const setDupeTab = useApp((s) => s.setDupeTab);
  const computeHashes = useApp((s) => s.computeHashes);
  const hashBusy = useApp((s) => s.hashBusy);
  const hashing = useApp((s) => s.hashing);

  return (
    <div className="dupe-view">
      <div className="dupe-header">
        <div className="dupe-tabs">
          <button
            className={`btn ${dupeTab === "exact" ? "btn-active" : ""}`}
            onClick={() => setDupeTab("exact")}
          >
            {t("dupe.exactTab", { n: dupeGroups.length })}
          </button>
          <button
            className={`btn ${dupeTab === "similar" ? "btn-active" : ""}`}
            onClick={() => setDupeTab("similar")}
          >
            {t("dupe.similarTab", { n: similarPairs.length })}
          </button>
        </div>
        <button className="btn" onClick={() => void computeHashes()} disabled={hashBusy}>
          {hashBusy ? t("dupe.hashing") : t("dupe.rescan")}
        </button>
      </div>

      {hashing && (
        <div className="hash-progress">
          {t("dupe.hashProgress", {
            done: hashing.processed,
            total: hashing.total,
            current: hashing.current,
          })}
        </div>
      )}

      {dupeTab === "exact" ? (
        <ExactGroups groups={dupeGroups} />
      ) : (
        <SimilarList pairs={similarPairs} />
      )}
    </div>
  );
}

function ExactGroups({ groups }: { groups: DupeGroup[] }) {
  const t = useT();
  if (groups.length === 0) {
    return <div className="dupe-empty">{t("dupe.exactEmpty")}</div>;
  }
  return (
    <div className="dupe-list">
      {groups.map((g) => (
        <section key={g.hash} className="dupe-group">
          <div className="dupe-group-title">
            {t("dupe.exactGroupTitle", {
              n: g.photos.length,
              size: formatBytes(g.totalSize / g.photos.length),
            })}
          </div>
          <div className="dupe-photos">
            {g.photos.map((p) => (
              <PhotoTile key={p.id} photo={p} />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

function SimilarList({ pairs }: { pairs: SimilarPair[] }) {
  const t = useT();
  if (pairs.length === 0) {
    return <div className="dupe-empty">{t("dupe.similarEmpty")}</div>;
  }
  return (
    <div className="dupe-list">
      {pairs.map((pair, i) => (
        <section key={i} className="dupe-group">
          <div className="dupe-group-title">
            {t("dupe.similarGroupTitle", { d: pair.distance })}
          </div>
          <div className="dupe-photos">
            {pair.photos.map((p) => (
              <PhotoTile key={p.id} photo={p} />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

function PhotoTile({ photo }: { photo: PhotoDto }) {
  const t = useT();
  const url = thumbUrl(photo.thumbPath);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(timer);
  }, [copied]);

  return (
    <div className="dupe-photo" title={photo.path}>
      {url ? (
        <img src={url} alt={photo.filename} loading="lazy" draggable={false} />
      ) : (
        <div className="photo-card-placeholder" />
      )}
      <div className="dupe-photo-meta">
        <span className="dupe-photo-name">{photo.filename}</span>
        <button
          className="btn tiny-btn"
          onClick={() => {
            void navigator.clipboard.writeText(photo.path);
            setCopied(true);
          }}
        >
          {copied ? t("dupe.copied") : t("dupe.copyPath")}
        </button>
      </div>
    </div>
  );
}
