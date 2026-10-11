# Shutter Hub

> 开源的本地摄影素材管理软件。你的素材，永远属于你。

Shutter Hub 是一个面向摄影爱好者的本地素材管理器：照片（含 RAW）留在你自己的硬盘上，扫描、索引、浏览全部在本地完成，无账号、无云上传、无订阅。

## ✨ 当前功能（M0）

- 📁 添加本地文件夹，递归扫描照片与 RAW（NEF/CR2/CR3/ARW/RAF/DNG/ORF/RW2…）
- 🗄️ SQLite 本地索引（EXIF：相机、镜头、焦距、光圈、快门、ISO、拍摄时间）
- 🖼️ 缩略图自动生成：RAW 优先抽取相机内嵌 JPEG 预览，零解码等待
- 🧭 虚拟滚动网格浏览，按拍摄时间排序，万张规模流畅
- 🔁 增量索引：文件未变化自动跳过；文件夹可重新扫描
- 🗑️ 移除文件夹只删索引，绝不碰你的文件

## 🗺️ 路线图

- **M1** 元数据完善：EXIF 信息面板、评分/色标/旗标、XMP sidecar 回写（兼容 Lightroom/Bridge）
- **M2** 筛选与搜索：按相机/镜头/参数/日期过滤，FTS5 全文搜索
- **M3** 卡工作流：SD/TF 卡检测 → 卡上快速选片 → 校验拷贝归档
- **M4** 相册/标签、重复检测（blake3 + 感知哈希）
- **M5** 打包签名、自动更新、深色模式打磨

## 🧱 技术栈

| 层 | 选型 |
| --- | --- |
| 外壳 | [Tauri 2](https://tauri.app)（macOS / Windows / Linux） |
| 后端 | Rust（rusqlite / kamadak-exif / image / rayon / walkdir） |
| 前端 | React 19 + TypeScript + Vite + TanStack Virtual |
| 数据库 | SQLite（WAL 模式，索引可随时重建） |

**架构原则**：文件系统是唯一真实数据源，SQLite 只是可重建的索引。评分/标签将写入 XMP sidecar，不绑架你的素材。

## 📦 仓库结构

```
shutter_hub/
├── apps/
│   └── desktop/          # Tauri 桌面应用
│       ├── src/          # React 前端
│       └── src-tauri/    # Rust 后端
│           └── src/
│               ├── db.rs        # SQLite schema 与连接
│               ├── indexer.rs   # 目录扫描与增量索引
│               ├── metadata.rs  # EXIF 解析
│               ├── thumbs.rs    # 缩略图生成（内嵌预览优先）
│               └── commands.rs  # Tauri IPC 命令
└── packages/             # 共享包（规划中）
```

## 🚀 开发

环境要求：Node.js ≥ 20、pnpm ≥ 10、Rust ≥ 1.77（stable）、[Tauri 平台依赖](https://tauri.app/start/prerequisites/)

```bash
pnpm install        # 安装依赖
pnpm dev            # 启动开发模式（热更新）
pnpm build          # 构建前端
pnpm typecheck      # TS 类型检查
pnpm check          # （见下：Rust 检查）

# Rust 侧
cd apps/desktop/src-tauri
cargo check         # 编译检查
cargo test          # 单元测试
```

打包发布：

```bash
cd apps/desktop
pnpm tauri build    # 产出 dmg / msi / deb 等安装包
```

## 📄 许可证

[MIT](./LICENSE)

## 🤝 贡献

欢迎 Issue 与 PR。功能讨论请先开 Issue 说明意图，再动工。
