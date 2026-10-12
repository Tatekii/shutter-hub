import { useEffect, useState } from "react";
import { formatBytes, thumbUrl } from "../lib/api";
import { useApp } from "../lib/store";
import type { DupeGroup, SimilarPair } from "../lib/types";
import type { PhotoDto } from "../lib/types";

export function DupeView() {
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
            完全重复（{dupeGroups.length} 组）
          </button>
          <button
            className={`btn ${dupeTab === "similar" ? "btn-active" : ""}`}
            onClick={() => setDupeTab("similar")}
          >
            相似照片（{similarPairs.length} 组）
          </button>
        </div>
        <button className="btn" onClick={() => void computeHashes()} disabled={hashBusy}>
          {hashBusy ? "哈希计算中…" : "重新扫描哈希"}
        </button>
      </div>

      {hashing && (
        <div className="hash-progress">
          正在计算哈希 {hashing.processed}/{hashing.total} · {hashing.current}
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
  if (groups.length === 0) {
    return (
      <div className="dupe-empty">
        没有发现完全重复的照片。点击「重新扫描哈希」计算照片内容指纹。
      </div>
    );
  }
  return (
    <div className="dupe-list">
      {groups.map((g) => (
        <section key={g.hash} className="dupe-group">
          <div className="dupe-group-title">
            {g.photos.length} 张完全相同的照片 · 每张 {formatBytes(g.totalSize / g.photos.length)}
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
  if (pairs.length === 0) {
    return (
      <div className="dupe-empty">
        没有发现相似照片。相似度基于感知哈希（dHash），可用于发现连拍中的近似照片。
      </div>
    );
  }
  return (
    <div className="dupe-list">
      {pairs.map((pair, i) => (
        <section key={i} className="dupe-group">
          <div className="dupe-group-title">相似组 · 汉明距离 {pair.distance}</div>
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
  const url = thumbUrl(photo.thumbPath);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(t);
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
          {copied ? "已复制" : "复制路径"}
        </button>
      </div>
    </div>
  );
}
