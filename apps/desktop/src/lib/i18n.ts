import { useApp } from "./store";

export type Lang = "zh" | "en";

/**
 * 极简 i18n：zh / en 双语字典 + 占位符插值（{name}）。
 * 语言状态存放在 zustand store（lang），组件通过 useT() 订阅更新。
 */
const STRINGS: Record<string, { zh: string; en: string }> = {
  // header
  "header.searchPlaceholder": {
    zh: "搜索文件名、路径、相机、镜头…",
    en: "Search filename, path, camera, lens…",
  },
  "header.clearSearch": { zh: "清空搜索", en: "Clear search" },
  "header.filters": { zh: "筛选条件", en: "Filters" },
  "header.filterBtn": { zh: "⚙ 筛选", en: "⚙ Filter" },
  "header.hashing": { zh: "哈希计算中…", en: "Hashing…" },
  "header.scanDupes": { zh: "扫描重复", en: "Scan duplicates" },
  "header.addFolder": { zh: "+ 添加文件夹", en: "+ Add folder" },
  "header.helpTitle": { zh: "键盘快捷键（?）", en: "Keyboard shortcuts (?)" },
  "header.stats": { zh: "{n} 张 · {m} 个文件夹", en: "{n} photos · {m} folders" },
  "header.ai": { zh: "AI", en: "AI" },

  // filter
  "filter.camera": { zh: "相机", en: "Camera" },
  "filter.all": { zh: "全部", en: "All" },
  "filter.lens": { zh: "镜头", en: "Lens" },
  "filter.isoMin": { zh: "ISO 不低于", en: "ISO ≥" },
  "filter.isoPlaceholder": { zh: "如 800", en: "e.g. 800" },
  "filter.focalMin": { zh: "最小焦距 (mm)", en: "Min focal (mm)" },
  "filter.focalPlaceholder": { zh: "如 35", en: "e.g. 35" },
  "filter.dateFrom": { zh: "拍摄日期从", en: "Date from" },
  "filter.dateTo": { zh: "到", en: "to" },
  "filter.minRating": { zh: "最低评分", en: "Min rating" },
  "filter.any": { zh: "不限", en: "Any" },
  "filter.colorLabel": { zh: "色标", en: "Color label" },
  "filter.flag": { zh: "旗标", en: "Flag" },
  "filter.pick": { zh: "✓ 留存", en: "✓ Pick" },
  "filter.reject": { zh: "✕ 淘汰", en: "✕ Reject" },
  "filter.hasFace": { zh: "有人脸", en: "Has faces" },
  "filter.mediaType": { zh: "类型", en: "Type" },
  "filter.photo": { zh: "照片", en: "Photo" },
  "filter.video": { zh: "视频", en: "Video" },
  "filter.clearAll": { zh: "清除全部筛选", en: "Clear all filters" },
  "filter.collapse": { zh: "收起", en: "Collapse" },

  // sidebar
  "sidebar.folders": { zh: "文件夹", en: "Folders" },
  "sidebar.addFolder": { zh: "添加文件夹", en: "Add folder" },
  "sidebar.noFolders": { zh: "还没有文件夹", en: "No folders yet" },
  "sidebar.rescan": { zh: "重新扫描", en: "Rescan" },
  "sidebar.removeFolder": { zh: "移除（不删除文件）", en: "Remove (files are not deleted)" },
  "sidebar.albums": { zh: "相册", en: "Albums" },
  "sidebar.newAlbum": { zh: "新建相册", en: "New album" },
  "sidebar.albumName": { zh: "相册名称", en: "Album name" },
  "sidebar.deleteAlbum": { zh: "删除相册（不删除照片）", en: "Delete album (photos are kept)" },
  "sidebar.tags": { zh: "标签", en: "Tags" },
  "sidebar.deleteTag": { zh: "删除标签", en: "Delete tag" },
  "sidebar.findDupes": { zh: "查找重复照片", en: "Find duplicates" },
  "sidebar.checkUpdates": { zh: "检查更新", en: "Check for updates" },
  "sidebar.checking": { zh: "检查中…", en: "Checking…" },
  "sidebar.upToDate": { zh: "当前已是最新版本", en: "You're up to date" },
  "sidebar.foundUpdate": {
    zh: "发现新版本 {v}，正在下载…",
    en: "Found version {v}, downloading…",
  },
  "sidebar.installedUpdate": {
    zh: "已安装 {v}，重启应用后生效",
    en: "Installed {v}, restart to apply",
  },
  "sidebar.updateFailed": { zh: "检查更新失败：{e}", en: "Update check failed: {e}" },
  "sidebar.colorKeyTitle": { zh: "色标快捷键 6-9", en: "Color label shortcuts 6-9" },
  "sidebar.viewLibrary": { zh: "资料库", en: "Library" },
  "sidebar.viewAlbum": { zh: "相册 · {name}", en: "Album · {name}" },
  "sidebar.viewTag": { zh: "标签 · #{name}", en: "Tag · #{name}" },
  "sidebar.viewDupes": { zh: "重复照片", en: "Duplicates" },

  // selection bar
  "select.count": { zh: "已选 {n} 张", en: "{n} selected" },
  "select.addToAlbum": { zh: "加入相册…", en: "Add to album…" },
  "select.addToAlbumTitle": { zh: "加入相册", en: "Add to album" },
  "select.tagPlaceholder": { zh: "添加标签（逗号分隔）", en: "Add tags (comma separated)" },
  "select.cancel": { zh: "取消选择", en: "Clear selection" },

  // inspector
  "inspector.title": { zh: "照片详情", en: "Photo details" },
  "inspector.close": { zh: "关闭 (Esc)", en: "Close (Esc)" },
  "inspector.noPreview": { zh: "无预览", en: "No preview" },
  "inspector.rate": { zh: "评分 {n}", en: "Rate {n}" },
  "inspector.colorToggle": {
    zh: "色标 {name}（再次点击取消）",
    en: "Color {name} (click again to clear)",
  },
  "inspector.pick": { zh: "留存 (P)", en: "Pick (P)" },
  "inspector.reject": { zh: "淘汰 (X)", en: "Reject (X)" },
  "inspector.pickLabel": { zh: "留存", en: "Pick" },
  "inspector.rejectLabel": { zh: "淘汰", en: "Reject" },
  "inspector.tagPlaceholder": { zh: "添加标签（逗号分隔）", en: "Add tags (comma separated)" },
  "inspector.rowFilename": { zh: "文件名", en: "Filename" },
  "inspector.rowTaken": { zh: "拍摄时间", en: "Taken" },
  "inspector.rowDims": { zh: "尺寸", en: "Dimensions" },
  "inspector.rowSize": { zh: "文件大小", en: "Size" },
  "inspector.rowCamera": { zh: "相机", en: "Camera" },
  "inspector.rowLens": { zh: "镜头", en: "Lens" },
  "inspector.rowFocal": { zh: "焦距", en: "Focal length" },
  "inspector.rowAperture": { zh: "光圈", en: "Aperture" },
  "inspector.rowShutter": { zh: "快门", en: "Shutter" },
  "inspector.rowIso": { zh: "ISO", en: "ISO" },
  "inspector.rowPath": { zh: "路径", en: "Path" },
  "inspector.rowType": { zh: "类型", en: "Type" },
  "inspector.rowDuration": { zh: "时长", en: "Duration" },
  "inspector.rowFaces": { zh: "人脸", en: "Faces" },
  "inspector.unknown": { zh: "未知", en: "Unknown" },

  // help
  "help.title": { zh: "键盘快捷键", en: "Keyboard shortcuts" },
  "help.done": { zh: "知道了", en: "Got it" },
  "help.prevNext": { zh: "切换照片", en: "Switch photo" },
  "help.openViewer": { zh: "打开看图模式", en: "Open viewer" },
  "help.rate": { zh: "评分 1-5 星", en: "Rate 1-5 stars" },
  "help.clearRate": { zh: "清除评分", en: "Clear rating" },
  "help.color": { zh: "色标：红 / 黄 / 绿 / 蓝", en: "Color: red / yellow / green / blue" },
  "help.pick": { zh: "旗标：留存", en: "Flag: pick" },
  "help.reject": { zh: "旗标：淘汰", en: "Flag: reject" },
  "help.clearFlag": { zh: "清除旗标", en: "Clear flag" },
  "help.multiSelect": { zh: "多选照片", en: "Multi-select" },
  "help.selectAll": { zh: "全选当前列表", en: "Select all" },
  "help.autoAdvance": {
    zh: "标记后自动前进（可关闭）",
    en: "Auto-advance after rating (toggleable)",
  },
  "help.zoomToggle": { zh: "适应窗口 / 1:1 切换", en: "Fit / 1:1 toggle" },
  "help.wheelDrag": { zh: "缩放 / 平移", en: "Zoom / pan" },
  "help.escape": {
    zh: "退出看图 / 关闭面板 / 清除选择",
    en: "Close viewer / panel / clear selection",
  },
  "help.help": { zh: "显示本帮助", en: "Show this help" },

  // empty state
  "empty.title": { zh: "开始整理你的素材", en: "Start organizing your library" },
  "empty.body": {
    zh: "添加一个存放照片的文件夹，Shutter Hub 会自动扫描并建立索引。",
    en: "Add a folder of photos and Shutter Hub will scan and index it automatically.",
  },
  "empty.addFolder": { zh: "添加文件夹", en: "Add folder" },

  // scan bar
  "scan.progress": { zh: "正在扫描 {done}/{total}", en: "Scanning {done}/{total}" },

  // dupes
  "dupe.exactTab": { zh: "完全重复（{n} 组）", en: "Exact duplicates ({n} groups)" },
  "dupe.similarTab": { zh: "相似照片（{n} 组）", en: "Similar photos ({n} groups)" },
  "dupe.rescan": { zh: "重新扫描哈希", en: "Rescan hashes" },
  "dupe.hashing": { zh: "哈希计算中…", en: "Hashing…" },
  "dupe.hashProgress": {
    zh: "正在计算哈希 {done}/{total} · {current}",
    en: "Hashing {done}/{total} · {current}",
  },
  "dupe.exactEmpty": {
    zh: "没有发现完全重复的照片。点击「重新扫描哈希」计算照片内容指纹。",
    en: "No exact duplicates found. Click \"Rescan hashes\" to compute content fingerprints.",
  },
  "dupe.exactGroupTitle": {
    zh: "{n} 张完全相同的照片 · 每张 {size}",
    en: "{n} identical photos · {size} each",
  },
  "dupe.similarEmpty": {
    zh: "没有发现相似照片。相似度基于感知哈希（dHash），可用于发现连拍中的近似照片。",
    en: "No similar photos found. Similarity uses perceptual hashes (dHash) to spot near-duplicates from bursts.",
  },
  "dupe.similarGroupTitle": {
    zh: "相似组 · 汉明距离 {d}",
    en: "Similar group · Hamming distance {d}",
  },
  "dupe.copied": { zh: "已复制", en: "Copied" },
  "dupe.copyPath": { zh: "复制路径", en: "Copy path" },

  // card browser
  "card.close": { zh: "关闭", en: "Close" },
  "card.scanning": { zh: "正在扫描存储卡…", en: "Scanning card…" },
  "card.space": { zh: "可用 {free} / {total}", en: "Free {free} / {total}" },
  "card.selected": {
    zh: "已选 {n}/{total} 张（{size}）",
    en: "{n}/{total} selected ({size})",
  },
  "card.toggleAll": { zh: "全选 / 全不选", en: "Select all / none" },
  "card.importing": { zh: "导入中…", en: "Importing…" },
  "card.import": { zh: "导入 {n} 张", en: "Import {n}" },
  "card.eject": { zh: "安全弹出", en: "Safely eject" },
  "card.ejectConfirm": { zh: "安全弹出「{name}」？", en: "Safely eject \"{name}\"?" },
  "card.importTo": { zh: "导入到：", en: "Import to:" },
  "card.noTarget": { zh: "未选择目标文件夹", en: "No target folder selected" },
  "card.choose": { zh: "选择…", en: "Choose…" },
  "card.organizeByDate": {
    zh: "按拍摄日期归档（YYYY/YYYY-MM-DD）",
    en: "Organize by date (YYYY/YYYY-MM-DD)",
  },
  "card.progress": { zh: "导入中 {done}/{total} · {current}", en: "Importing {done}/{total} · {current}" },
  "card.done": {
    zh: "✓ 导入完成：{n} 张，共 {size}",
    en: "✓ Import complete: {n} files, {size}",
  },
  "card.failed": { zh: "失败 {n} 张", en: "{n} failed" },

  // viewer
  "viewer.loading": { zh: "加载中…", en: "Loading…" },
  "viewer.unsupported": {
    zh: "无法预览此格式（仅支持 JPEG / 带内嵌预览的 RAW / 常规图像格式）",
    en: "Cannot preview this format (JPEG / RAW with embedded preview / common image formats only)",
  },
  "viewer.autoAdvance": { zh: "自动前进", en: "Auto-advance" },
  "viewer.autoAdvanceOn": { zh: "开", en: "On" },
  "viewer.autoAdvanceOff": { zh: "关", en: "Off" },
  "viewer.autoAdvanceTitle": {
    zh: "评分 / 色标 / 旗标后自动前进下一张",
    en: "Auto-advance to the next photo after rating / color / flag",
  },
  "viewer.close": { zh: "关闭", en: "Close" },
  "viewer.closeTitle": { zh: "关闭 (Esc)", en: "Close (Esc)" },
  "viewer.pick": { zh: "✓ 留存", en: "✓ Pick" },
  "viewer.reject": { zh: "✕ 淘汰", en: "✕ Reject" },
  "viewer.rate": { zh: "评分 {n}", en: "Rating {n}" },
  "viewer.play": { zh: "▶ 在系统播放器中打开", en: "▶ Open in system player" },
  "viewer.videoHint": { zh: "视频不支持内置播放", en: "In-app video playback is not supported" },

  // app
  "app.volumeDetected": { zh: "💾 检测到存储设备（{n}）", en: "💾 Storage devices detected ({n})" },
  "app.browserMode": {
    zh: "浏览器预览模式：Tauri 后端不可用，仅可调试界面；完整功能请用 pnpm tauri dev 启动",
    en: "Browser preview: Tauri backend unavailable. UI-only debugging. Run pnpm tauri dev for full functionality.",
  },
  "app.aiResults": { zh: "AI 语义搜索结果 · {n} 张", en: "AI semantic search results · {n} photos" },
  "app.clearAiResults": { zh: "清除 AI 结果", en: "Clear AI results" },

  // ai panel
  "ai.title": { zh: "AI 功能（本地 ONNX）", en: "AI features (local ONNX)" },
  "ai.notDownloaded": {
    zh: "模型尚未下载（约 180 MB，一次性）",
    en: "Models not downloaded yet (~180 MB, one-time)",
  },
  "ai.download": { zh: "下载模型", en: "Download models" },
  "ai.downloading": {
    zh: "下载中 {file} · {received}/{total}",
    en: "Downloading {file} · {received}/{total}",
  },
  "ai.ready": { zh: "模型已就绪", en: "Models ready" },
  "ai.computeEmbeddings": { zh: "生成语义索引", en: "Build semantic index" },
  "ai.detectFaces": { zh: "检测人脸", en: "Detect faces" },
  "ai.embeddingsStatus": {
    zh: "语义索引：{done}/{total}",
    en: "Embeddings: {done}/{total}",
  },
  "ai.facesStatus": { zh: "人脸扫描：{done}/{total}", en: "Face scan: {done}/{total}" },
  "ai.progress": {
    zh: "{label} {done}/{total} · {current}",
    en: "{label} {done}/{total} · {current}",
  },
  "ai.labelEmbeddings": { zh: "生成语义索引", en: "Building embeddings" },
  "ai.labelFaces": { zh: "检测人脸", en: "Detecting faces" },
  "ai.busy": { zh: "AI 任务进行中…", en: "AI task running…" },
  "ai.semanticSearch": { zh: "AI 语义", en: "AI semantic" },
  "ai.keywordSearch": { zh: "关键词", en: "Keyword" },

  // colors
  "color.red": { zh: "红", en: "red" },
  "color.yellow": { zh: "黄", en: "yellow" },
  "color.green": { zh: "绿", en: "green" },
  "color.blue": { zh: "蓝", en: "blue" },
  "color.purple": { zh: "紫", en: "purple" },
};

export function translate(
  lang: Lang,
  key: string,
  vars?: Record<string, string | number>,
): string {
  const entry = STRINGS[key];
  let s = entry ? entry[lang] : key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.split(`{${k}}`).join(String(v));
    }
  }
  return s;
}

/** 组件内获取翻译函数；语言切换时自动重渲染。 */
export function useT() {
  const lang = useApp((s) => s.lang);
  return (key: string, vars?: Record<string, string | number>) =>
    translate(lang, key, vars);
}
