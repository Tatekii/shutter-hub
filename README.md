# Shutter Hub

> 开源的本地摄影素材管理软件。你的素材，永远属于你。

Shutter Hub 是一个面向摄影爱好者的本地素材管理器：照片（含 RAW）留在你自己的硬盘上，扫描、索引、浏览、选片全部在本地完成，无账号、无云上传、无订阅。

## ✨ 功能

### 素材管理

- 📁 添加本地文件夹，递归扫描照片与 RAW（NEF/CR2/CR3/ARW/RAF/DNG/ORF/RW2…）
- 🗄️ SQLite 本地索引（EXIF：相机、镜头、焦距、光圈、快门、ISO、拍摄时间）
- 🖼️ 缩略图自动生成：RAW 优先抽取相机内嵌 JPEG 预览，零解码等待
- 🧭 虚拟滚动网格浏览，按拍摄时间排序，万张规模流畅
- 🔁 增量索引：文件未变化自动跳过；文件夹可重新扫描
- 🗑️ 移除文件夹只删索引，绝不碰你的文件

### 选片与元数据

- ⭐ 评分（1-5 星）、色标（红黄绿蓝紫）、旗标（留存/淘汰），纯键盘操作（1-5 / 6-9 / P / X）
- 📝 XMP sidecar 回写，与 Lightroom / Bridge 互通，不绑架素材
- 🔍 FTS5 全文搜索（文件名/路径/相机/镜头）+ 组合筛选（相机、镜头、ISO、焦距、日期、评分、色标、旗标）
- 🧩 相册与标签，多选批量操作

### 看图模式

- 🔍 双击 / 回车进入看图：适应窗口与 1:1 原尺寸检查，滚轮缩放、拖拽平移（Z 切换）
- ⚡ 看图内直接选片：评分 / 色标 / 旗标（1-5 / 6-9 / P / X / U），标记后自动前进下一张（可开关）
- 🖼️ 全尺寸预览懒加载缓存：JPEG 字节级直拷不重编码，RAW 抽取相机内嵌全尺寸预览

### 卡工作流（差异化卖点）

- 💾 自动检测 SD/TF 卡等可移动设备
- ⚡ 卡上直接快速选片（零导入），勾选后校验拷贝入库
- 🗂️ 可选按拍摄日期归档（YYYY/YYYY-MM-DD）
- ✅ blake3 校验保证拷贝完整，防坏卡丢片
- ⏏️ 安全弹出存储卡

### 重复检测

- 🔁 完全重复：blake3 内容哈希分组
- 👯 相似照片：dHash 感知哈希发现连拍近似图

## 🗺️ 路线图

- 人脸识别、AI 语义搜索（本地 ONNX）
- 视频素材支持
- 多语言界面（i18n）

## 🧱 技术栈

| 层 | 选型 |
| --- | --- |
| 外壳 | [Tauri 2](https://tauri.app)（macOS / Windows / Linux） |
| 后端 | Rust（rusqlite / kamadak-exif / image / rayon / walkdir / blake3 / sysinfo） |
| 前端 | React 19 + TypeScript + Vite + TanStack Virtual + Zustand |
| 数据库 | SQLite（WAL 模式，索引可随时重建） |

**架构原则**：文件系统是唯一真实数据源，SQLite 只是可重建的索引。评分/标签写入 XMP sidecar，素材随时可带着走。

## 📦 仓库结构

```
shutter_hub/
├── apps/
│   └── desktop/          # Tauri 桌面应用
│       ├── src/          # React 前端（components / lib）
│       └── src-tauri/    # Rust 后端
│           ├── src/
│           │   ├── db.rs        # SQLite schema 与迁移
│           │   ├── indexer.rs   # 目录扫描与增量索引
│           │   ├── metadata.rs  # EXIF 解析
│           │   ├── thumbs.rs    # 缩略图 + dHash
│           │   ├── preview.rs   # 看图全尺寸预览生成
│           │   ├── xmp.rs       # XMP sidecar 读写
│           │   ├── volumes.rs   # 存储卡检测 / 导入 / 弹出
│           │   └── commands/    # Tauri IPC（library / organize / dupes）
│           └── tests/           # 后端集成测试
├── .github/workflows/    # 三平台打包 + 测试 CI
└── packages/             # 共享包（规划中）
```

## 🚀 开发

环境要求：Node.js ≥ 20、pnpm ≥ 10、Rust ≥ 1.77（stable）、[Tauri 平台依赖](https://tauri.app/start/prerequisites/)

```bash
pnpm install        # 安装依赖
pnpm dev            # 启动开发模式（热更新）
pnpm build          # 构建前端
pnpm typecheck      # TS 类型检查

# Rust 侧
cd apps/desktop/src-tauri
cargo check         # 编译检查
cargo test          # 单元 + 集成测试（扫描/XMP/查询/去重全链路）
```

打包发布：

```bash
cd apps/desktop
pnpm tauri build    # 产出 dmg / msi / deb 等安装包
```

## 📦 发布与自动更新

- 推送 `v*` 标签触发 GitHub Actions 三平台构建（macOS x2 架构 / Windows / Linux），产物自动附加到 Release
- 自动更新使用 tauri-plugin-updater + 签名清单（`latest.json`）
- 签名密钥通过仓库 Secrets 注入：`TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- 本地生成密钥：`pnpm tauri signer generate -w ~/.tauri/shutter-hub.key`，公钥写入 `tauri.conf.json`

## 📄 许可证

[MIT](./LICENSE)

## 🤝 贡献

欢迎 Issue 与 PR。功能讨论请先开 Issue 说明意图，再动工。
