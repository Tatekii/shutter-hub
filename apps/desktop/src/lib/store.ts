import { create } from "zustand";
import * as api from "./api";
import type { Lang } from "./i18n";
import type {
  AlbumDto,
  CardPhotoDto,
  DupeGroup,
  FacetDto,
  FolderDto,
  HashProgress,
  IngestProgress,
  IngestResult,
  LibraryStats,
  PhotoDto,
  PhotoQuery,
  ScanProgress,
  SimilarPair,
  TagDto,
  VolumeDto,
} from "./types";

export type View =
  | { kind: "library" }
  | { kind: "album"; id: number; name: string }
  | { kind: "tag"; id: number; name: string }
  | { kind: "dupes" };

interface AppState {
  folders: FolderDto[];
  albums: AlbumDto[];
  tags: TagDto[];
  stats: LibraryStats;
  facets: FacetDto | null;
  photos: PhotoDto[];
  query: PhotoQuery;
  view: View;
  selectedIds: Set<number>;
  inspectorId: number | null;
  viewerId: number | null;
  loading: boolean;
  error: string | null;
  scanning: ScanProgress | null;
  hashing: HashProgress | null;
  hashBusy: boolean;
  dupeGroups: DupeGroup[];
  similarPairs: SimilarPair[];
  dupeTab: "exact" | "similar";
  volumes: VolumeDto[];
  cardVolume: VolumeDto | null;
  cardPhotos: CardPhotoDto[];
  cardMarks: Set<string>;
  cardScanning: boolean;
  ingesting: boolean;
  ingestProgress: IngestProgress | null;
  ingestResult: IngestResult | null;
  lang: Lang;

  init: () => Promise<void>;
  refresh: () => Promise<void>;
  refreshPhotos: () => Promise<void>;
  setQuery: (q: Partial<PhotoQuery>) => Promise<void>;
  setView: (v: View) => void;
  select: (id: number, additive: boolean) => void;
  selectAll: () => void;
  clearSelection: () => void;
  setInspector: (id: number | null) => void;
  moveSelection: (dir: 1 | -1) => void;
  openViewer: (id: number) => void;
  closeViewer: () => void;
  viewerMove: (dir: 1 | -1) => void;
  rateSelected: (rating: number) => Promise<void>;
  colorSelected: (label: string) => Promise<void>;
  flagSelected: (flag: string) => Promise<void>;
  addFolder: () => Promise<void>;
  rescanFolder: (id: number) => Promise<void>;
  removeFolder: (id: number) => Promise<void>;
  createAlbum: (name: string) => Promise<void>;
  deleteAlbum: (id: number) => Promise<void>;
  addSelectedToAlbum: (albumId: number) => Promise<void>;
  removeSelectedFromAlbum: (albumId: number) => Promise<void>;
  addTagsToSelected: (tags: string[]) => Promise<void>;
  deleteTag: (id: number) => Promise<void>;
  refreshVolumes: () => Promise<void>;
  openVolume: (v: VolumeDto) => Promise<void>;
  closeVolume: () => void;
  toggleCardMark: (path: string) => void;
  markAllCard: (marked: boolean) => void;
  startIngest: (target: string, organizeByDate: boolean) => Promise<void>;
  eject: (path: string) => Promise<void>;
  computeHashes: () => Promise<void>;
  loadDupes: () => Promise<void>;
  setDupeTab: (t: "exact" | "similar") => void;
  setLang: (l: Lang) => void;
  setError: (e: string | null) => void;
  setScanning: (s: ScanProgress | null) => void;
  setHashing: (h: HashProgress | null) => void;
  setIngestProgress: (p: IngestProgress | null) => void;
}

function effectiveQuery(query: PhotoQuery, view: View): PhotoQuery {
  const base = { ...query };
  delete base.albumId;
  delete base.tagId;
  if (view.kind === "album") base.albumId = view.id;
  if (view.kind === "tag") base.tagId = view.id;
  return base;
}

function detectLang(): Lang {
  const saved = localStorage.getItem("shutter_hub.lang");
  if (saved === "zh" || saved === "en") return saved;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export const useApp = create<AppState>((set, get) => ({
  folders: [],
  albums: [],
  tags: [],
  stats: { photoCount: 0, folderCount: 0 },
  facets: null,
  photos: [],
  query: {},
  view: { kind: "library" },
  selectedIds: new Set(),
  inspectorId: null,
  viewerId: null,
  loading: true,
  error: null,
  scanning: null,
  hashing: null,
  hashBusy: false,
  dupeGroups: [],
  similarPairs: [],
  dupeTab: "exact",
  volumes: [],
  cardVolume: null,
  cardPhotos: [],
  cardMarks: new Set(),
  cardScanning: false,
  ingesting: false,
  ingestProgress: null,
  ingestResult: null,
  lang: detectLang(),

  init: async () => {
    try {
      const [folders, albums, tags, stats, facets, photos, volumes] =
        await Promise.all([
          api.listFolders(),
          api.listAlbums(),
          api.listTags(),
          api.getStats(),
          api.listFacets(),
          api.listPhotos({}),
          api.listVolumes(),
        ]);
      set({ folders, albums, tags, stats, facets, photos, volumes, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  refresh: async () => {
    try {
      const [folders, albums, tags, stats, facets] = await Promise.all([
        api.listFolders(),
        api.listAlbums(),
        api.listTags(),
        api.getStats(),
        api.listFacets(),
      ]);
      set({ folders, albums, tags, stats, facets });
      await get().refreshPhotos();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  refreshPhotos: async () => {
    const { query, view } = get();
    if (view.kind === "dupes") return;
    try {
      const photos = await api.listPhotos(effectiveQuery(query, view));
      set({ photos });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setQuery: async (q) => {
    set({ query: { ...get().query, ...q } });
    await get().refreshPhotos();
  },

  setView: (v) => {
    set({ view: v, selectedIds: new Set(), inspectorId: null, viewerId: null });
    void get().refreshPhotos();
  },

  select: (id, additive) => {
    const { selectedIds } = get();
    if (additive) {
      const next = new Set(selectedIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      set({ selectedIds: next, inspectorId: id });
    } else {
      set({ selectedIds: new Set([id]), inspectorId: id });
    }
  },

  selectAll: () => {
    set({ selectedIds: new Set(get().photos.map((p) => p.id)) });
  },

  clearSelection: () => {
    set({ selectedIds: new Set(), inspectorId: null });
  },

  setInspector: (id) => {
    if (id === null) {
      set({ inspectorId: null, selectedIds: new Set() });
    } else {
      set({ inspectorId: id, selectedIds: new Set([id]) });
    }
  },

  moveSelection: (dir) => {
    const { photos, inspectorId } = get();
    if (photos.length === 0) return;
    const index = photos.findIndex((p) => p.id === inspectorId);
    const nextIndex =
      index === -1 ? 0 : Math.min(Math.max(index + dir, 0), photos.length - 1);
    const id = photos[nextIndex].id;
    set({ inspectorId: id, selectedIds: new Set([id]) });
  },

  openViewer: (id) => {
    set({ viewerId: id, inspectorId: id, selectedIds: new Set([id]) });
  },

  closeViewer: () => {
    set({ viewerId: null });
  },

  // 看图内导航。当前照片被筛选掉（如旗标筛选 + 淘汰操作）时，
  // 前进落到原位置的照片、后退落到列表开头，保证看图不中断。
  viewerMove: (dir) => {
    const { photos, viewerId } = get();
    if (photos.length === 0) return;
    const index = photos.findIndex((p) => p.id === viewerId);
    const base = index === -1 ? (dir === 1 ? -1 : 0) : index;
    const nextIndex = Math.min(Math.max(base + dir, 0), photos.length - 1);
    const id = photos[nextIndex].id;
    set({ viewerId: id, inspectorId: id, selectedIds: new Set([id]) });
  },

  rateSelected: async (rating) => {
    const { inspectorId, selectedIds } = get();
    const ids = inspectorId != null ? [inspectorId] : [...selectedIds];
    if (ids.length === 0) return;
    try {
      await Promise.all(ids.map((id) => api.setRating(id, rating)));
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  colorSelected: async (label) => {
    const { inspectorId, selectedIds } = get();
    const ids = inspectorId != null ? [inspectorId] : [...selectedIds];
    if (ids.length === 0) return;
    try {
      await Promise.all(ids.map((id) => api.setColorLabel(id, label)));
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  flagSelected: async (flag) => {
    const { inspectorId, selectedIds } = get();
    const ids = inspectorId != null ? [inspectorId] : [...selectedIds];
    if (ids.length === 0) return;
    try {
      await Promise.all(ids.map((id) => api.setFlag(id, flag)));
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addFolder: async () => {
    try {
      set({ error: null });
      await api.addFolder();
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  rescanFolder: async (id) => {
    try {
      set({ error: null });
      await api.rescanFolder(id);
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  removeFolder: async (id) => {
    try {
      set({ error: null });
      await api.removeFolder(id);
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createAlbum: async (name) => {
    try {
      await api.createAlbum(name);
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteAlbum: async (id) => {
    try {
      await api.deleteAlbum(id);
      const view = get().view;
      if (view.kind === "album" && view.id === id) {
        set({ view: { kind: "library" } });
      }
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addSelectedToAlbum: async (albumId) => {
    const ids = [...get().selectedIds];
    if (ids.length === 0) return;
    try {
      await api.addPhotosToAlbum(albumId, ids);
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  removeSelectedFromAlbum: async (albumId) => {
    const ids = [...get().selectedIds];
    if (ids.length === 0) return;
    try {
      await api.removePhotosFromAlbum(albumId, ids);
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addTagsToSelected: async (tags) => {
    const ids = [...get().selectedIds];
    if (ids.length === 0) return;
    try {
      await api.addTags(ids, tags);
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteTag: async (id) => {
    try {
      await api.deleteTag(id);
      const view = get().view;
      if (view.kind === "tag" && view.id === id) {
        set({ view: { kind: "library" } });
      }
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  refreshVolumes: async () => {
    try {
      const volumes = await api.listVolumes();
      set({ volumes });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  openVolume: async (v) => {
    set({ cardVolume: v, cardPhotos: [], cardMarks: new Set(), cardScanning: true });
    try {
      const cardPhotos = await api.browseVolume(v.path);
      set({ cardPhotos, cardScanning: false });
    } catch (e) {
      set({ cardScanning: false, error: String(e) });
    }
  },

  closeVolume: () => {
    set({ cardVolume: null, cardPhotos: [], cardMarks: new Set(), ingestResult: null });
  },

  toggleCardMark: (path) => {
    const marks = new Set(get().cardMarks);
    if (marks.has(path)) marks.delete(path);
    else marks.add(path);
    set({ cardMarks: marks });
  },

  markAllCard: (marked) => {
    if (marked) {
      set({ cardMarks: new Set(get().cardPhotos.map((p) => p.path)) });
    } else {
      set({ cardMarks: new Set() });
    }
  },

  startIngest: async (target, organizeByDate) => {
    const files = [...get().cardMarks];
    if (files.length === 0) return;
    set({ ingesting: true, ingestProgress: null, ingestResult: null });
    try {
      const result = await api.ingestFiles(files, target, organizeByDate);
      set({ ingestResult: result, ingesting: false });
      await get().refreshVolumes();
    } catch (e) {
      set({ error: String(e), ingesting: false });
    }
  },

  eject: async (path) => {
    try {
      await api.ejectVolume(path);
      set({ cardVolume: null });
      await get().refreshVolumes();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  computeHashes: async () => {
    set({ hashBusy: true, hashing: null });
    try {
      await api.computeHashes();
      set({ hashBusy: false });
      await get().loadDupes();
    } catch (e) {
      set({ hashBusy: false, error: String(e) });
    }
  },

  loadDupes: async () => {
    try {
      const [dupeGroups, similarPairs] = await Promise.all([
        api.listDupes(),
        api.listSimilar(),
      ]);
      set({ dupeGroups, similarPairs });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setDupeTab: (t) => set({ dupeTab: t }),
  setLang: (l) => {
    localStorage.setItem("shutter_hub.lang", l);
    set({ lang: l });
  },
  setError: (e) => set({ error: e }),
  setScanning: (s) => set({ scanning: s }),
  setHashing: (h) => set({ hashing: h }),
  setIngestProgress: (p) => set({ ingestProgress: p }),
}));
