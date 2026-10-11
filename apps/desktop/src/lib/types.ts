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
