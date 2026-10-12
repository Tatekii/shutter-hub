use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderDto {
    pub id: i64,
    pub path: String,
    pub photo_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoDto {
    pub id: i64,
    pub folder_id: i64,
    pub path: String,
    pub filename: String,
    pub size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<f64>,
    pub aperture: Option<f64>,
    pub shutter_speed: Option<String>,
    pub iso: Option<i64>,
    pub taken_at: Option<String>,
    pub has_thumb: bool,
    pub thumb_path: Option<String>,
    pub rating: i64,
    pub color_label: String,
    pub flag: String,
    /// "photo" | "video"
    pub media_type: String,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PhotoQuery {
    /// FTS5 全文搜索（文件名/路径/相机/镜头）
    pub q: Option<String>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso_min: Option<i64>,
    pub iso_max: Option<i64>,
    pub focal_min: Option<f64>,
    pub focal_max: Option<f64>,
    pub aperture_min: Option<f64>,
    pub aperture_max: Option<f64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub min_rating: Option<i64>,
    pub color_label: Option<String>,
    pub flag: Option<String>,
    /// "photo" | "video"
    pub media_type: Option<String>,
    /// 仅看/排除有人脸的照片
    pub has_face: Option<bool>,
    pub album_id: Option<i64>,
    pub tag_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub folder_id: i64,
    pub processed: u64,
    pub total: u64,
    pub current: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub folder_id: i64,
    pub added: u64,
    pub updated: u64,
    pub skipped: u64,
    pub total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub photo_count: i64,
    pub folder_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDto {
    pub id: i64,
    pub name: String,
    pub photo_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: i64,
    pub name: String,
    pub photo_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeDto {
    pub name: String,
    pub path: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPhotoDto {
    pub path: String,
    pub filename: String,
    pub size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub camera_model: Option<String>,
    pub taken_at: Option<String>,
    pub thumb_path: Option<String>,
    /// "photo" | "video"
    pub media_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestRequest {
    pub files: Vec<String>,
    pub target: String,
    pub organize_by_date: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResult {
    pub copied: u64,
    pub failed: Vec<String>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestProgress {
    pub processed: u64,
    pub total: u64,
    pub current: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupeGroup {
    pub hash: String,
    pub total_size: i64,
    pub photos: Vec<PhotoDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarPair {
    pub distance: u32,
    pub photos: Vec<PhotoDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashProgress {
    pub processed: u64,
    pub total: u64,
    pub current: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetDto {
    pub camera_models: Vec<FacetValue>,
    pub lens_models: Vec<FacetValue>,
    pub iso_range: (Option<i64>, Option<i64>),
    pub focal_range: (Option<f64>, Option<f64>),
    pub date_range: (Option<String>, Option<String>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetValue {
    pub value: String,
    pub count: i64,
}

// ---------- 本地 AI ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    pub clip_ready: bool,
    pub face_ready: bool,
    pub embeddings_done: i64,
    pub embeddings_total: i64,
    pub faces_done: i64,
    pub faces_total: i64,
    pub busy: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiDownloadProgress {
    pub file: String,
    pub received: u64,
    pub total: Option<u64>,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiJobProgress {
    /// "embeddings" | "faces"
    pub kind: String,
    pub processed: u64,
    pub total: u64,
    pub current: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiJobDone {
    pub kind: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceDto {
    /// 归一化坐标（0-1，相对图片尺寸）
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

// ---------- 出图导出 ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub photo_ids: Vec<i64>,
    pub target_dir: String,
    /// "jpeg" | "png" | "webp"
    pub format: String,
    /// 1-100，JPEG / WebP 有效
    pub quality: u8,
    /// 长边像素上限（None = 原尺寸）
    pub max_dimension: Option<u32>,
    /// 文件名后缀，如 "_edit"
    pub filename_suffix: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub processed: u64,
    pub total: u64,
    pub current: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub exported: u64,
    pub failed: Vec<String>,
    pub total_bytes: u64,
    pub target_dir: String,
}
