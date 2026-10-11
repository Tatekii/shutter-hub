import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  FolderDto,
  LibraryStats,
  PhotoDto,
  ScanProgress,
  ScanStats,
} from "./types";

export const addFolder = () => invoke<ScanStats | null>("add_folder");

export const rescanFolder = (folderId: number) =>
  invoke<ScanStats>("rescan_folder", { folderId });

export const removeFolder = (folderId: number) =>
  invoke<void>("remove_folder", { folderId });

export const listFolders = () => invoke<FolderDto[]>("list_folders");

export const listPhotos = () => invoke<PhotoDto[]>("list_photos");

export const getStats = () => invoke<LibraryStats>("get_stats");

export const thumbUrl = (photo: PhotoDto) =>
  photo.thumbPath ? convertFileSrc(photo.thumbPath) : null;

export const onScanProgress = (
  handler: (p: ScanProgress) => void,
): Promise<UnlistenFn> => listen<ScanProgress>("scan://progress", (e) => handler(e.payload));

export const onScanDone = (handler: (s: ScanStats) => void): Promise<UnlistenFn> =>
  listen<ScanStats>("scan://done", (e) => handler(e.payload));

export const onScanError = (handler: (msg: string) => void): Promise<UnlistenFn> =>
  listen<string>("scan://error", (e) => handler(e.payload));

export function formatFocal(f: number | null): string {
  return f != null ? `${f}mm` : "";
}

export function formatAperture(a: number | null): string {
  return a != null ? `f/${a}` : "";
}
