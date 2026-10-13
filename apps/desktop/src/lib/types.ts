export interface FolderDto {
  id: number;
  path: string;
  photoCount: number;
}

export interface PhotoDto {
  id: number;
  folderId: number;
  path: string;
  filename: string;
  size: number;
  width: number | null;
  height: number | null;
  cameraMake: string | null;
  cameraModel: string | null;
  lensModel: string | null;
  focalLength: number | null;
  aperture: number | null;
  shutterSpeed: string | null;
  iso: number | null;
  takenAt: string | null;
  hasThumb: boolean;
  thumbPath: string | null;
  rating: number;
  colorLabel: string;
  flag: string;
  mediaType: "photo" | "video";
  durationMs: number | null;
  liveMotion: string | null;
  hasEdit: boolean;
}

export interface PhotoQuery {
  q?: string;
  camera?: string;
  lens?: string;
  isoMin?: number;
  isoMax?: number;
  focalMin?: number;
  focalMax?: number;
  apertureMin?: number;
  apertureMax?: number;
  dateFrom?: string;
  dateTo?: string;
  minRating?: number;
  colorLabel?: string;
  flag?: string;
  mediaType?: "photo" | "video";
  hasFace?: boolean;
  albumId?: number;
  tagId?: number;
}

export interface ScanProgress {
  folderId: number;
  processed: number;
  total: number;
  current: string;
}

export interface ScanStats {
  folderId: number;
  added: number;
  updated: number;
  skipped: number;
  total: number;
}

export interface LibraryStats {
  photoCount: number;
  folderCount: number;
}

export interface AlbumDto {
  id: number;
  name: string;
  photoCount: number;
}

export interface TagDto {
  id: number;
  name: string;
  photoCount: number;
}

export interface VolumeDto {
  name: string;
  path: string;
  totalBytes: number;
  availableBytes: number;
}

export interface CardPhotoDto {
  path: string;
  filename: string;
  size: number;
  width: number | null;
  height: number | null;
  cameraModel: string | null;
  takenAt: string | null;
  thumbPath: string | null;
  mediaType: "photo" | "video";
}

export interface IngestProgress {
  processed: number;
  total: number;
  current: string;
}

export interface IngestResult {
  copied: number;
  failed: string[];
  totalBytes: number;
}

export interface DupeGroup {
  hash: string;
  totalSize: number;
  photos: PhotoDto[];
}

export interface SimilarPair {
  distance: number;
  photos: PhotoDto[];
}

export interface HashProgress {
  processed: number;
  total: number;
  current: string;
}

export interface FacetValue {
  value: string;
  count: number;
}

export interface FacetDto {
  cameraModels: FacetValue[];
  lensModels: FacetValue[];
  isoRange: [number | null, number | null];
  focalRange: [number | null, number | null];
  dateRange: [string | null, string | null];
}

export interface AiStatus {
  clipReady: boolean;
  faceReady: boolean;
  embeddingsDone: number;
  embeddingsTotal: number;
  facesDone: number;
  facesTotal: number;
  busy: boolean;
}

export interface AiDownloadProgress {
  file: string;
  received: number;
  total: number | null;
  done: boolean;
  error: string | null;
}

export interface AiJobProgress {
  kind: "embeddings" | "faces";
  processed: number;
  total: number;
  current: string;
}

export interface AiJobDone {
  kind: string;
  ok: boolean;
  error: string | null;
}

export interface FaceDto {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Adjustments {
  exposure: number;
  contrast: number;
  highlights: number;
  shadows: number;
  temperature: number;
  tint: number;
  saturation: number;
}

export interface ExportRequest {
  photoIds: number[];
  targetDir: string;
  format: "jpeg" | "png" | "webp" | "tiff";
  quality: number;
  maxDimension: number | null;
  filenameSuffix: string;
  watermark?: string | null;
}

export interface ExportProgress {
  processed: number;
  total: number;
  current: string;
}

export interface ExportResult {
  exported: number;
  failed: string[];
  totalBytes: number;
  targetDir: string;
}

export interface CollageRequest {
  photoIds: number[];
  layout: "grid" | "strip" | "featured" | "vertical";
  cols: number;
  gap: number;
  radius: number;
  background: string;
  size: number;
  quality: number;
  targetDir: string;
  title?: string;
}

export interface CollageResult {
  path: string;
  width: number;
  height: number;
  failed: string[];
}

export interface ShareCardRequest {
  photoIds: number[];
  aspect: "1:1" | "4:5" | "3:4";
  background: string;
  textColor: string;
  showInfo: boolean;
  watermark: string;
  targetDir: string;
  title?: string;
  caption?: string;
  polaroid?: boolean;
}

export interface ShareCardResult {
  paths: string[];
  failed: string[];
}
