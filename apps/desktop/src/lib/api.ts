import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Adjustments,
  AiDownloadProgress,
  AiJobDone,
  AiJobProgress,
  AiStatus,
  AlbumDto,
  CardPhotoDto,
  CollageRequest,
  CollageResult,
  DupeGroup,
  ExportProgress,
  ExportRequest,
  ExportResult,
  FacetDto,
  FaceDto,
  FolderDto,
  HashProgress,
  IngestProgress,
  IngestResult,
  LibraryStats,
  PhotoDto,
  PhotoQuery,
  ScanProgress,
  ScanStats,
  SimilarPair,
  TagDto,
  VolumeDto,
} from "./types";

export const addFolder = () => invoke<ScanStats | null>("add_folder");
export const rescanFolder = (folderId: number) =>
  invoke<ScanStats>("rescan_folder", { folderId });
export const removeFolder = (folderId: number) =>
  invoke<void>("remove_folder", { folderId });
export const listFolders = () => invoke<FolderDto[]>("list_folders");
export const listPhotos = (query: PhotoQuery) =>
  invoke<PhotoDto[]>("list_photos", { query });
export const getPhoto = (id: number) => invoke<PhotoDto>("get_photo", { id });
export const getViewerPreview = (id: number) =>
  invoke<string | null>("get_viewer_preview", { id });
export const getStats = () => invoke<LibraryStats>("get_stats");
export const listFacets = () => invoke<FacetDto>("list_facets");

export const setRating = (id: number, rating: number) =>
  invoke<void>("set_rating", { id, rating });
export const setColorLabel = (id: number, label: string) =>
  invoke<void>("set_color_label", { id, label });
export const setFlag = (id: number, flag: string) =>
  invoke<void>("set_flag", { id, flag });

export const listAlbums = () => invoke<AlbumDto[]>("list_albums");
export const createAlbum = (name: string) => invoke<AlbumDto>("create_album", { name });
export const deleteAlbum = (albumId: number) =>
  invoke<void>("delete_album", { albumId });
export const addPhotosToAlbum = (albumId: number, photoIds: number[]) =>
  invoke<void>("add_photos_to_album", { albumId, photoIds });
export const removePhotosFromAlbum = (albumId: number, photoIds: number[]) =>
  invoke<void>("remove_photos_from_album", { albumId, photoIds });

export const listTags = () => invoke<TagDto[]>("list_tags");
export const addTags = (photoIds: number[], tags: string[]) =>
  invoke<void>("add_tags", { photoIds, tags });
export const removeTagFromPhotos = (tagId: number, photoIds: number[]) =>
  invoke<void>("remove_tag_from_photos", { tagId, photoIds });
export const deleteTag = (tagId: number) => invoke<void>("delete_tag", { tagId });

export const listVolumes = () => invoke<VolumeDto[]>("list_volumes");
export const browseVolume = (path: string) =>
  invoke<CardPhotoDto[]>("browse_volume", { path });
export const ingestFiles = (files: string[], target: string, organizeByDate: boolean) =>
  invoke<IngestResult>("ingest_files", {
    req: { files, target, organizeByDate },
  });
export const ejectVolume = (path: string) => invoke<void>("eject_volume", { path });

export const computeHashes = () => invoke<void>("compute_hashes");
export const listDupes = () => invoke<DupeGroup[]>("list_dupes");
export const listSimilar = () => invoke<SimilarPair[]>("list_similar");

export const aiStatus = () => invoke<AiStatus>("ai_status");
export const aiDownloadModels = () => invoke<void>("ai_download_models");
export const aiComputeEmbeddings = () => invoke<void>("ai_compute_embeddings");
export const aiDetectFaces = () => invoke<void>("ai_detect_faces");
export const aiSearch = (query: string, limit?: number) =>
  invoke<PhotoDto[]>("ai_search", { query, limit });
export const listFaces = (photoId: number) => invoke<FaceDto[]>("list_faces", { photoId });

export const exportPhotos = (req: ExportRequest) =>
  invoke<ExportResult>("export_photos", { req });
export const renderCollage = (req: CollageRequest) =>
  invoke<CollageResult>("render_collage", { req });

export const getEdit = (id: number) => invoke<Adjustments>("get_edit", { id });
export const setEdit = (id: number, adj: Adjustments) =>
  invoke<void>("set_edit", { id, adj });
export const exportLiveMotion = (photoId: number, targetDir: string, format: "gif" | "copy") =>
  invoke<string>("export_live_motion", { photoId, targetDir, format });

export const thumbUrl = (thumbPath: string | null) =>
  thumbPath ? convertFileSrc(thumbPath) : null;

export function listenEvent<T>(
  name: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(name, (e) => handler(e.payload));
}

export const onScanProgress = (h: (p: ScanProgress) => void) =>
  listenEvent("scan://progress", h);
export const onScanDone = (h: (s: ScanStats) => void) => listenEvent("scan://done", h);
export const onScanError = (h: (msg: string) => void) => listenEvent("scan://error", h);

export const onHashProgress = (h: (p: HashProgress) => void) =>
  listenEvent("hash://progress", h);
export const onHashDone = (h: () => void) => listenEvent("hash://done", h);

export const onIngestProgress = (h: (p: IngestProgress) => void) =>
  listenEvent("ingest://progress", h);
export const onIngestDone = (h: (r: IngestResult) => void) =>
  listenEvent("ingest://done", h);

export const onAiDownload = (h: (p: AiDownloadProgress) => void) =>
  listenEvent("ai://download", h);
export const onAiProgress = (h: (p: AiJobProgress) => void) => listenEvent("ai://progress", h);
export const onAiDone = (h: (p: AiJobDone) => void) => listenEvent("ai://done", h);

export const onExportProgress = (h: (p: ExportProgress) => void) =>
  listenEvent("export://progress", h);

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function formatDuration(ms: number): string {
  const totalSec = Math.round(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

export const COLOR_LABELS: Record<string, string> = {
  red: "红",
  yellow: "黄",
  green: "绿",
  blue: "蓝",
  purple: "紫",
};

export const COLOR_HEX: Record<string, string> = {
  red: "#e06666",
  yellow: "#e0c458",
  green: "#66c166",
  blue: "#5a8de0",
  purple: "#b066d0",
};
