//! 拼图合成引擎：网格 / 胶片条 / 大图 三种模板。
//!
//! 全部使用方形单元格：cover 裁剪 → Lanczos 缩放 → 圆角蒙版混合到背景。
//! 输出尺寸以「长边像素」为基准，间距 / 圆角以输出坐标系像素计。

use image::{Rgb, RgbImage};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub col: u32,
    pub row: u32,
    pub col_span: u32,
    pub row_span: u32,
}

/// 计算模板的单元格排布，返回 (单元格列表, 列数, 行数)。
pub fn layout(kind: &str, cols: u32, count: usize) -> (Vec<Cell>, u32, u32) {
    let n = count.max(1) as u32;
    match kind {
        "strip" => {
            let cols = n.clamp(2, 6);
            let cells = (0..n.min(cols))
                .map(|i| Cell {
                    col: i,
                    row: 0,
                    col_span: 1,
                    row_span: 1,
                })
                .collect();
            (cells, cols, 1)
        }
        "featured" => {
            let n = n.clamp(2, 3);
            let mut cells = vec![Cell {
                col: 0,
                row: 0,
                col_span: 2,
                row_span: 2,
            }];
            for i in 1..n {
                cells.push(Cell {
                    col: 2,
                    row: i - 1,
                    col_span: 1,
                    row_span: 1,
                });
            }
            (cells, 3, 2)
        }
        _ => {
            let cols = cols.clamp(2, 4);
            let cells = (0..n)
                .map(|i| Cell {
                    col: i % cols,
                    row: i / cols,
                    col_span: 1,
                    row_span: 1,
                })
                .collect();
            let rows = n.div_ceil(cols);
            (cells, cols, rows)
        }
    }
}

/// 解析 "#rrggbb" / "rrggbb" 颜色。
pub fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// cover 裁剪为正方形并缩放到 side。
fn cover_square(img: &image::DynamicImage, side: u32) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let s = w.min(h);
    let x = (w - s) / 2;
    let y = (h - s) / 2;
    img.crop_imm(x, y, s, s)
        .resize_exact(side, side, image::imageops::FilterType::Lanczos3)
        .to_rgb8()
}

/// 圆角蒙版覆盖率（0-1，1px 软边）。
fn corner_alpha(px: u32, py: u32, w: u32, h: u32, r: f32) -> f32 {
    if r <= 0.5 {
        return 1.0;
    }
    let fx = px as f32;
    let fy = py as f32;
    let cx = if fx < r {
        Some(r)
    } else if fx > w as f32 - r - 1.0 {
        Some(w as f32 - r - 1.0)
    } else {
        None
    };
    let cy = if fy < r {
        Some(r)
    } else if fy > h as f32 - r - 1.0 {
        Some(h as f32 - r - 1.0)
    } else {
        None
    };
    match (cx, cy) {
        (Some(cx), Some(cy)) => {
            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            (r + 0.5 - dist).clamp(0.0, 1.0)
        }
        _ => 1.0,
    }
}

fn paste_rounded(canvas: &mut RgbImage, tile: &RgbImage, x: u32, y: u32, radius: u32, bg: [u8; 3]) {
    let (w, h) = tile.dimensions();
    let r = radius.min(w / 2).min(h / 2) as f32;
    for (px, py, pixel) in tile.enumerate_pixels() {
        let alpha = corner_alpha(px, py, w, h, r);
        if alpha <= 0.001 {
            continue;
        }
        let (tx, ty) = (x + px, y + py);
        if tx >= canvas.width() || ty >= canvas.height() {
            continue;
        }
        let dst = canvas.get_pixel_mut(tx, ty);
        if alpha >= 0.999 {
            *dst = *pixel;
        } else {
            for c in 0..3 {
                dst[c] = (pixel[c] as f32 * alpha + bg[c] as f32 * (1.0 - alpha)) as u8;
            }
        }
    }
}

/// 渲染拼图画布。images 与 layout 单元一一对应，None 表示该格留背景色。
pub fn render(
    images: &[Option<image::DynamicImage>],
    kind: &str,
    cols: u32,
    gap: u32,
    radius: u32,
    bg: [u8; 3],
    size: u32,
) -> RgbImage {
    let (cells, grid_cols, grid_rows) = layout(kind, cols, images.len());
    let size = size.clamp(64, 8192);
    let gap = gap.min(size / 8);
    let raw_cell = (size as i64 - gap as i64 * (grid_cols as i64 + 1)) / grid_cols as i64;
    let cell = raw_cell.max(16) as u32;

    let out_w = grid_cols * cell + gap * (grid_cols + 1);
    let out_h = grid_rows * cell + gap * (grid_rows + 1);
    let mut canvas = RgbImage::from_pixel(out_w, out_h, Rgb(bg));

    for (img, cell_def) in images.iter().zip(cells.iter()) {
        let Some(img) = img else { continue };
        let w = cell * cell_def.col_span + gap * (cell_def.col_span - 1);
        let h = cell * cell_def.row_span + gap * (cell_def.row_span - 1);
        let x = gap + cell_def.col * (cell + gap);
        let y = gap + cell_def.row * (cell + gap);
        let tile = cover_square(img, w.min(h));
        paste_rounded(&mut canvas, &tile, x, y, radius, bg);
    }

    canvas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_layout_rows_and_cols() {
        let (cells, cols, rows) = layout("grid", 2, 5);
        assert_eq!((cols, rows), (2, 3));
        assert_eq!(cells.len(), 5);
        assert_eq!(cells[4], Cell { col: 0, row: 2, col_span: 1, row_span: 1 });
    }

    #[test]
    fn featured_layout_has_one_big_cell() {
        let (cells, cols, rows) = layout("featured", 3, 4);
        assert_eq!((cols, rows), (3, 2));
        assert_eq!(cells[0].col_span, 2);
        assert_eq!(cells[0].row_span, 2);
        assert_eq!(cells.len(), 3, "featured supports at most 1 big + 2 small");
        assert_eq!(
            cells[2],
            Cell {
                col: 2,
                row: 1,
                col_span: 1,
                row_span: 1
            }
        );
    }

    #[test]
    fn strip_layout_is_single_row() {
        let (cells, cols, rows) = layout("strip", 2, 3);
        assert_eq!((cols, rows), (3, 1));
        assert!(cells.iter().all(|c| c.row == 0));
    }

    #[test]
    fn parse_colors() {
        assert_eq!(parse_hex_color("#ffffff"), Some([255, 255, 255]));
        assert_eq!(parse_hex_color("1a1a1e"), Some([26, 26, 30]));
        assert_eq!(parse_hex_color("#f5f0e8"), Some([245, 240, 232]));
        assert_eq!(parse_hex_color("bad"), None);
    }

    #[test]
    fn render_grid_produces_expected_canvas() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            80,
            80,
            Rgb([200, 30, 30]),
        ));
        let images: Vec<Option<image::DynamicImage>> =
            (0..4).map(|_| Some(img.clone())).collect();
        let canvas = render(&images, "grid", 2, 4, 8, [255, 255, 255], 200);

        // cell = (200 - 4*3) / 2 = 94 → 2*94 + 3*4 = 200
        assert_eq!(canvas.dimensions(), (200, 200));
        // 左上角是背景（圆角外）
        assert_eq!(canvas.get_pixel(0, 0), &Rgb([255, 255, 255]));
        // 第一格中心是照片
        let px = canvas.get_pixel(4 + 47, 4 + 47);
        assert!(px[0] > 150 && px[1] < 80, "photo red at cell center: {px:?}");
        // 间隙处是背景
        let gap_px = canvas.get_pixel(4 + 94 + 2, 4 + 47);
        assert_eq!(gap_px, &Rgb([255, 255, 255]));
    }

    #[test]
    fn render_handles_missing_cells() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            60,
            60,
            Rgb([10, 120, 200]),
        ));
        let images = vec![Some(img), None, None];
        let canvas = render(&images, "grid", 2, 8, 0, [0, 0, 0], 256);
        // 第二格为背景黑
        let cell = ((256 - 8 * 3) / 2) as u32;
        let px = canvas.get_pixel(8 + cell + 8 + cell / 2, 8 + cell / 2);
        assert_eq!(px, &Rgb([0, 0, 0]));
    }
}
