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
