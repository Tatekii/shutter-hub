//! 本地 AI：CLIP 语义向量 + UltraFace 人脸检测（ONNX Runtime）。
//!
//! 模型首次使用时从 HuggingFace 下载到 `<app_data>/models/`
//! （约 180 MB，一次性）。未下载时所有 AI 功能优雅降级。

use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const CLIP_DIM: usize = 512;
pub const CLIP_IMAGE_SIZE: u32 = 224;
pub const FACE_SCORE_THRESHOLD: f32 = 0.7;
pub const FACE_NMS_THRESHOLD: f32 = 0.4;

pub struct ModelSpec {
    pub file: &'static str,
    pub url: &'static str,
}

/// 需要下载的模型文件清单（URL 已验证可直接下载，无需鉴权）。
pub const MODEL_FILES: &[ModelSpec] = &[
    ModelSpec {
        file: "clip_text_quantized.onnx",
        url: "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/text_model_quantized.onnx",
    },
    ModelSpec {
        file: "clip_vision_quantized.onnx",
        url: "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_quantized.onnx",
    },
    ModelSpec {
        file: "clip_tokenizer.json",
        url: "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/tokenizer.json",
    },
    ModelSpec {
        file: "ultraface_rfb_320.onnx",
        url: "https://huggingface.co/Arm/ultraface-rfb-320-onnx-raspberry/resolve/main/ultraface-rfb-320_raspberry_onnx_original.onnx",
    },
];

pub const CLIP_TEXT_FILE: &str = "clip_text_quantized.onnx";
pub const CLIP_VISION_FILE: &str = "clip_vision_quantized.onnx";
pub const CLIP_TOKENIZER_FILE: &str = "clip_tokenizer.json";
pub const FACE_MODEL_FILE: &str = "ultraface_rfb_320.onnx";

pub fn clip_ready(dir: &Path) -> bool {
    [CLIP_TEXT_FILE, CLIP_VISION_FILE, CLIP_TOKENIZER_FILE]
        .iter()
        .all(|f| dir.join(f).exists())
}

pub fn face_ready(dir: &Path) -> bool {
    dir.join(FACE_MODEL_FILE).exists()
}

pub fn model_path(dir: &Path, file: &str) -> PathBuf {
    dir.join(file)
}

/// 下载缺失的模型文件；已有文件跳过。`on_progress(file, received, total)`。
pub fn download_models<F>(dir: &Path, mut on_progress: F) -> Result<()>
where
    F: FnMut(&str, u64, Option<u64>),
{
    std::fs::create_dir_all(dir)?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(900))
        .build();

    for spec in MODEL_FILES {
        let out = dir.join(spec.file);
        if out.exists() {
            continue;
        }
        let tmp = dir.join(format!("{}.part", spec.file));
        let resp = agent
            .get(spec.url)
            .call()
            .with_context(|| format!("download {}", spec.url))?;
        let total: Option<u64> = resp.header("Content-Length").and_then(|s| s.parse().ok());
        let mut reader = resp.into_reader();
        let mut file =
            std::fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        let mut buf = vec![0u8; 256 * 1024];
        let mut received: u64 = 0;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            received += n as u64;
            on_progress(spec.file, received, total);
        }
        file.flush()?;
        drop(file);
        std::fs::rename(&tmp, &out)?;
    }
    Ok(())
}

/// 删除模型文件（用于重试下载）。
pub fn remove_models(dir: &Path) {
    for spec in MODEL_FILES {
        let _ = std::fs::remove_file(dir.join(spec.file));
        let _ = std::fs::remove_file(dir.join(format!("{}.part", spec.file)));
    }
}

// ---------- 向量数学 ----------

/// L2 归一化（0 向量保持原样）。
pub fn l2_normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// 归一化向量的点积（= 余弦相似度）。取 top-k 返回 (photo_id, score)。
pub fn cosine_topk(query: &[f32], rows: &[(i64, Vec<f32>)], k: usize) -> Vec<(i64, f32)> {
    let mut scored: Vec<(i64, f32)> = rows
        .iter()
        .filter(|(_, emb)| emb.len() == query.len())
        .map(|(id, emb)| {
            let dot: f32 = query.iter().zip(emb.iter()).map(|(a, b)| a * b).sum();
            (*id, dot)
        })
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(k);
    scored
}

// ---------- 人脸检测（纯解码逻辑，便于测试） ----------

/// 解析 UltraFace 输出：scores [N,2] + boxes [N,4]（归一化 x1y1x2y2），
/// 输出归一化的 (x, y, w, h)（相对原图），含分数阈值与 NMS。
pub fn decode_ultraface(
    scores: &[f32],
    boxes: &[f32],
    score_threshold: f32,
    nms_threshold: f32,
) -> Vec<[f32; 4]> {
    let n = boxes.len() / 4;
    let mut candidates: Vec<([f32; 4], f32)> = Vec::new();

    for i in 0..n {
        let face_score = scores.get(i * 2 + 1).copied().unwrap_or(0.0);
        if face_score < score_threshold {
            continue;
        }
        let b = &boxes[i * 4..i * 4 + 4];
        let x1 = b[0].clamp(0.0, 1.0);
        let y1 = b[1].clamp(0.0, 1.0);
        let x2 = b[2].clamp(0.0, 1.0);
        let y2 = b[3].clamp(0.0, 1.0);
        if x2 <= x1 || y2 <= y1 {
            continue;
        }
        candidates.push(([x1, y1, x2, y2], face_score));
    }

    candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut kept: Vec<[f32; 4]> = Vec::new();
    for (box_a, _) in &candidates {
        let mut suppressed = false;
        for box_b in &kept {
            if iou(box_a, box_b) > nms_threshold {
                suppressed = true;
                break;
            }
        }
        if !suppressed {
            kept.push(*box_a);
        }
    }

    kept.iter()
        .map(|b| [b[0], b[1], b[2] - b[0], b[3] - b[1]])
        .collect()
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let x1 = a[0].max(b[0]);
    let y1 = a[1].max(b[1]);
    let x2 = a[2].min(b[2]);
    let y2 = a[3].min(b[3]);
    let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let area_a = (a[2] - a[0]) * (a[3] - a[1]);
    let area_b = (b[2] - b[0]) * (b[3] - b[1]);
    let union = area_a + area_b - inter;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}

// ---------- ONNX 会话 ----------

fn session_from_file(path: &Path) -> Result<ort::session::Session> {
    ort::session::Session::builder()
        .map_err(|e| anyhow::anyhow!("ort session builder: {e}"))?
        .commit_from_file(path)
        .map_err(|e| anyhow::anyhow!("load model {}: {e}", path.display()))
}

fn input_names(session: &ort::session::Session) -> Vec<String> {
    session.inputs().iter().map(|i| i.name().to_string()).collect()
}

fn output_names(session: &ort::session::Session) -> Vec<String> {
    session.outputs().iter().map(|o| o.name().to_string()).collect()
}

/// 按名字子串偏好从输出名列表里挑一个，找不到用第一个。
fn pick_name(names: &[String], preferred: &[&str]) -> Option<String> {
    preferred
        .iter()
        .find_map(|p| names.iter().find(|n| n.contains(p)))
        .cloned()
        .or_else(|| names.first().cloned())
}

fn extract_f32(outputs: &ort::session::SessionOutputs<'_>, name: &str) -> Result<Vec<f32>> {
    let (_, data) = outputs[name]
        .try_extract_tensor::<f32>()
        .map_err(|e| anyhow::anyhow!("extract output {name}: {e}"))?;
    Ok(data.to_vec())
}

fn tensor_f32(shape: Vec<usize>, data: Vec<f32>) -> Result<ort::value::Tensor<f32>> {
    ort::value::Tensor::from_array((shape, data)).map_err(|e| anyhow::anyhow!("tensor: {e}"))
}

fn tensor_i64(shape: Vec<usize>, data: Vec<i64>) -> Result<ort::value::Tensor<i64>> {
    ort::value::Tensor::from_array((shape, data)).map_err(|e| anyhow::anyhow!("tensor: {e}"))
}

// ---------- CLIP 图像向量 ----------

/// 预处理：等比缩放 + 中心裁剪到 224，CLIP 归一化，NCHW。
pub fn clip_image_data(img: &image::DynamicImage) -> Vec<f32> {
    let resized = img.resize_to_fill(
        CLIP_IMAGE_SIZE,
        CLIP_IMAGE_SIZE,
        image::imageops::FilterType::Triangle,
    );
    let rgb = resized.to_rgb8();
    let mean = [0.48145466f32, 0.4578275, 0.40821073];
    let std = [0.26862954f32, 0.26130258, 0.27577711];
    let size = CLIP_IMAGE_SIZE as usize;
    let mut data = Vec::with_capacity(3 * size * size);
    for c in 0..3usize {
        for y in 0..size {
            for x in 0..size {
                let p = rgb.get_pixel(x as u32, y as u32)[c] as f32 / 255.0;
                data.push((p - mean[c]) / std[c]);
            }
        }
    }
    data
}

/// 生成图片的 CLIP 向量（已归一化，CLIP_DIM 维）。
pub fn embed_image(model_dir: &Path, img: &image::DynamicImage) -> Result<Vec<f32>> {
    let mut session = session_from_file(&model_path(model_dir, CLIP_VISION_FILE))?;
    let names = input_names(&session);
    let input_name = names
        .iter()
        .find(|n| n.contains("pixel") || n.contains("image"))
        .cloned()
        .or_else(|| names.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("vision model has no inputs"))?;

    let data = clip_image_data(img);
    let tensor = tensor_f32(
        vec![1, 3, CLIP_IMAGE_SIZE as usize, CLIP_IMAGE_SIZE as usize],
        data,
    )?;
    let out_names = output_names(&session);
    let name = pick_name(&out_names, &["image_embeds", "embeds", "pooler"])
        .ok_or_else(|| anyhow::anyhow!("vision model has no outputs"))?;
    let outputs = session
        .run(ort::inputs![input_name.as_str() => tensor])
        .map_err(|e| anyhow::anyhow!("vision run: {e}"))?;
    let mut emb = extract_f32(&outputs, &name)?;

    if emb.len() % CLIP_DIM != 0 {
        anyhow::bail!("unexpected embedding size {}", emb.len());
    }
    if emb.len() > CLIP_DIM {
        // last_hidden_state 兜底：取 CLS token（前 512 维）
        emb.truncate(CLIP_DIM);
    }
    l2_normalize(&mut emb);
    Ok(emb)
}

// ---------- CLIP 文本向量 ----------

/// 生成查询文本的 CLIP 向量（已归一化）。
pub fn embed_text(model_dir: &Path, text: &str) -> Result<Vec<f32>> {
    let tokenizer = tokenizers::Tokenizer::from_file(model_path(model_dir, CLIP_TOKENIZER_FILE))
        .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;
    let encoding = tokenizer
        .encode(text, true)
        .map_err(|e| anyhow::anyhow!("tokenize: {e}"))?;

    let context = 77usize;
    let eot = tokenizer.token_to_id("<|endoftext|>").unwrap_or(49407) as i64;
    let ids = encoding.get_ids();
    let len = ids.len().min(context);
    let mut input_ids = vec![eot; context];
    let mut attention = vec![0i64; context];
    for i in 0..len {
        input_ids[i] = ids[i] as i64;
        attention[i] = 1;
    }

    let mut session = session_from_file(&model_path(model_dir, CLIP_TEXT_FILE))?;
    let names = input_names(&session);
    let ids_name = names
        .iter()
        .find(|n| n.contains("input_ids"))
        .cloned()
        .or_else(|| names.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("text model has no inputs"))?;
    let mask_name = names.iter().find(|n| n.contains("attention")).cloned();

    let ids_tensor = tensor_i64(vec![1, context], input_ids)?;
    let out_names = output_names(&session);
    let name = pick_name(&out_names, &["text_embeds", "embeds", "pooler"])
        .ok_or_else(|| anyhow::anyhow!("text model has no outputs"))?;
    let outputs = match &mask_name {
        Some(mask_name) => {
            let mask_tensor = tensor_i64(vec![1, context], attention.clone())?;
            session
                .run(ort::inputs![ids_name.as_str() => ids_tensor, mask_name.as_str() => mask_tensor])
        }
        None => session.run(ort::inputs![ids_name.as_str() => ids_tensor]),
    }
    .map_err(|e| anyhow::anyhow!("text run: {e}"))?;

    let mut emb = extract_f32(&outputs, &name)?;

    if emb.len() == context * CLIP_DIM {
        // last_hidden_state：按 attention mask 做平均池化
        let mut pooled = vec![0f32; CLIP_DIM];
        let mut count = 0f32;
        for (t, &keep) in attention.iter().enumerate() {
            if keep == 0 {
                continue;
            }
            count += 1.0;
            for d in 0..CLIP_DIM {
                pooled[d] += emb[t * CLIP_DIM + d];
            }
        }
        if count > 0.0 {
            for v in pooled.iter_mut() {
                *v /= count;
            }
        }
        emb = pooled;
    } else if emb.len() != CLIP_DIM {
        anyhow::bail!("unexpected text embedding size {}", emb.len());
    }

    l2_normalize(&mut emb);
    Ok(emb)
}

// ---------- UltraFace ----------

pub struct FaceModel {
    session: ort::session::Session,
    input_name: String,
    width: u32,
    height: u32,
    score_output: String,
    box_output: String,
}

impl FaceModel {
    pub fn load(model_dir: &Path) -> Result<Self> {
        let session = session_from_file(&model_path(model_dir, FACE_MODEL_FILE))?;
        let input_name = input_names(&session)
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("face model has no inputs"))?;

        // UltraFace RFB-320：固定 240x320 输入
        let (width, height) = (320u32, 240u32);

        // 输出按最后一维分类：2 = scores，4 = boxes
        let mut score_output = None;
        let mut box_output = None;
        for outlet in session.outputs() {
            if let Some(shape) = outlet.dtype().tensor_shape() {
                match shape.last() {
                    Some(2) => score_output = Some(outlet.name().to_string()),
                    Some(4) => box_output = Some(outlet.name().to_string()),
                    _ => {}
                }
            }
        }
        let score_output =
            score_output.ok_or_else(|| anyhow::anyhow!("face scores output not found"))?;
        let box_output = box_output.ok_or_else(|| anyhow::anyhow!("face boxes output not found"))?;

        Ok(Self {
            session,
            input_name,
            width,
            height,
            score_output,
            box_output,
        })
    }

    /// 检测一张图的人脸框（归一化 x/y/w/h）。
    pub fn detect(&mut self, img: &image::DynamicImage) -> Result<Vec<[f32; 4]>> {
        let resized =
            img.resize_exact(self.width, self.height, image::imageops::FilterType::Triangle);
        let rgb = resized.to_rgb8();
        let mut data = Vec::with_capacity((3 * self.width * self.height) as usize);
        for c in 0..3usize {
            for y in 0..self.height {
                for x in 0..self.width {
                    let p = rgb.get_pixel(x, y)[c] as f32;
                    data.push(p / 127.5 - 1.0);
                }
            }
        }
        let tensor = tensor_f32(vec![1, 3, self.height as usize, self.width as usize], data)?;
        let outputs = self
            .session
            .run(ort::inputs![self.input_name.as_str() => tensor])
            .map_err(|e| anyhow::anyhow!("face run: {e}"))?;

        let scores = extract_f32(&outputs, &self.score_output)?;
        let boxes = extract_f32(&outputs, &self.box_output)?;

        Ok(decode_ultraface(
            &scores,
            &boxes,
            FACE_SCORE_THRESHOLD,
            FACE_NMS_THRESHOLD,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_topk_orders_by_similarity() {
        let query = vec![1.0, 0.0];
        let rows = vec![
            (1i64, vec![0.0, 1.0]),
            (2i64, vec![1.0, 0.0]),
            (3i64, vec![0.7, 0.7]),
            (4i64, vec![1.0]), // 维度不符，忽略
        ];
        let top = cosine_topk(&query, &rows, 2);
        assert_eq!(top[0].0, 2);
        assert_eq!(top[1].0, 3);
        assert_eq!(top.len(), 2);
    }

    #[test]
    fn ultraface_decode_filters_low_scores_and_nms() {
        // 3 个候选：高分框 A、与 A 高度重叠的高分框 B、低分框 C
        let scores = vec![
            0.9, 0.95, // A
            0.1, 0.92, // B（与 A 重叠）
            0.9, 0.3, // C（低于阈值）
        ];
        let boxes = vec![
            0.10, 0.10, 0.30, 0.30, // A
            0.11, 0.11, 0.31, 0.31, // B
            0.60, 0.60, 0.80, 0.80, // C
        ];
        let faces = decode_ultraface(&scores, &boxes, 0.7, 0.4);
        assert_eq!(faces.len(), 1, "B suppressed by NMS, C below threshold");
        let f = faces[0];
        assert!((f[0] - 0.10).abs() < 1e-5);
        assert!((f[2] - 0.20).abs() < 1e-5);
    }

    #[test]
    fn clip_image_data_shape_and_range() {
        let img = image::RgbImage::from_fn(640, 480, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let data = clip_image_data(&image::DynamicImage::ImageRgb8(img));
        assert_eq!(data.len(), 3 * 224 * 224);
        assert!(data.iter().all(|v| v.abs() < 4.0));
    }

    #[test]
    fn models_ready_checks_files() {
        let dir = std::env::temp_dir().join(format!("shutter_hub_ai_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!clip_ready(&dir));
        assert!(!face_ready(&dir));
        std::fs::write(dir.join(CLIP_TEXT_FILE), b"x").unwrap();
        std::fs::write(dir.join(CLIP_VISION_FILE), b"x").unwrap();
        std::fs::write(dir.join(CLIP_TOKENIZER_FILE), b"x").unwrap();
        std::fs::write(dir.join(FACE_MODEL_FILE), b"x").unwrap();
        assert!(clip_ready(&dir));
        assert!(face_ready(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 真实模型冒烟测试：下载约 180MB 并跑完整推理链路。
    /// 默认 ignore，手动验证：`cargo test -- --ignored ai_models_download`
    #[test]
    #[ignore = "downloads ~180MB models from HuggingFace"]
    fn ai_models_download_and_infer() {
        let dir = std::env::temp_dir().join("shutter_hub_ai_models_smoke");
        download_models(&dir, |_, _, _| {}).expect("download models");
        assert!(clip_ready(&dir), "clip files present");
        assert!(face_ready(&dir), "face file present");

        // 图像向量：512 维、已归一化
        let img = image::RgbImage::from_fn(640, 480, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 120])
        });
        let emb = embed_image(&dir, &image::DynamicImage::ImageRgb8(img)).expect("image embed");
        assert_eq!(emb.len(), CLIP_DIM);
        let norm: f32 = emb.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "normalized: {norm}");

        // 文本向量：512 维、与图像向量余弦有限
        let text = embed_text(&dir, "a colorful gradient").expect("text embed");
        assert_eq!(text.len(), CLIP_DIM);
        let dot: f32 = emb.iter().zip(text.iter()).map(|(a, b)| a * b).sum();
        assert!(dot.is_finite());

        // 人脸模型加载 + 空白图检测（应无人脸，但不报错）
        let mut model = FaceModel::load(&dir).expect("face model load");
        let blank = image::DynamicImage::ImageRgb8(image::RgbImage::new(320, 240));
        let faces = model.detect(&blank).expect("face detect");
        assert!(faces.is_empty(), "blank image has no faces");

        std::fs::remove_dir_all(&dir).ok();
    }
}
