use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct DifferentialMetrics {
    pub total_pixels: usize,
    pub differing_pixels: usize,
    pub diff_percentage: f64,
    pub rmse: f64,
    pub psnr: f64,
    pub max_channel_delta: u8,
}

pub struct DifferentialComparator;

impl DifferentialComparator {
    /// Compares two RGBA buffers of identical dimensions and computes differential metrics.
    pub fn compare(
        buf_a: &[u8],
        buf_b: &[u8],
        width: u32,
        height: u32,
        threshold: u8,
    ) -> Option<DifferentialMetrics> {
        let expected_len = (width * height * 4) as usize;
        if buf_a.len() != expected_len || buf_b.len() != expected_len {
            return None;
        }

        let total_pixels = (width * height) as usize;
        let mut differing_pixels = 0usize;
        let mut sum_squared_error = 0.0f64;
        let mut max_delta = 0u8;

        for (px_a, px_b) in buf_a.chunks_exact(4).zip(buf_b.chunks_exact(4)) {
            let dr = (px_a[0] as i32 - px_b[0] as i32).abs() as u8;
            let dg = (px_a[1] as i32 - px_b[1] as i32).abs() as u8;
            let db = (px_a[2] as i32 - px_b[2] as i32).abs() as u8;
            let da = (px_a[3] as i32 - px_b[3] as i32).abs() as u8;

            let px_max = dr.max(dg).max(db).max(da);
            if px_max > max_delta {
                max_delta = px_max;
            }

            if px_max > threshold {
                differing_pixels += 1;
            }

            let se = (dr as f64).powi(2)
                + (dg as f64).powi(2)
                + (db as f64).powi(2)
                + (da as f64).powi(2);
            sum_squared_error += se / 4.0;
        }

        let mse = sum_squared_error / total_pixels as f64;
        let rmse = mse.sqrt();
        let diff_percentage = (differing_pixels as f64 / total_pixels as f64) * 100.0;
        let psnr = if mse < 1e-10 {
            100.0
        } else {
            20.0 * (255.0f64).log10() - 10.0 * mse.log10()
        };

        Some(DifferentialMetrics {
            total_pixels,
            differing_pixels,
            diff_percentage,
            rmse,
            psnr,
            max_channel_delta: max_delta,
        })
    }

    /// Generates a diff heatmap image: matching pixels are faded, differing pixels highlighted in bright red/magenta.
    pub fn generate_diff_image(
        buf_a: &[u8],
        buf_b: &[u8],
        width: u32,
        height: u32,
        threshold: u8,
    ) -> Option<Vec<u8>> {
        let expected_len = (width * height * 4) as usize;
        if buf_a.len() != expected_len || buf_b.len() != expected_len {
            return None;
        }

        let mut diff_rgba = Vec::with_capacity(expected_len);

        for (px_a, px_b) in buf_a.chunks_exact(4).zip(buf_b.chunks_exact(4)) {
            let dr = (px_a[0] as i32 - px_b[0] as i32).abs() as u8;
            let dg = (px_a[1] as i32 - px_b[1] as i32).abs() as u8;
            let db = (px_a[2] as i32 - px_b[2] as i32).abs() as u8;
            let da = (px_a[3] as i32 - px_b[3] as i32).abs() as u8;

            let px_max = dr.max(dg).max(db).max(da);

            if px_max > threshold {
                // Bright red highlight for differences
                diff_rgba.push(255);
                diff_rgba.push(0);
                diff_rgba.push((px_max as u16 * 2).min(255) as u8);
                diff_rgba.push(255);
            } else {
                // Dimmed grayscale for identical background
                let gray = ((px_a[0] as u16 + px_a[1] as u16 + px_a[2] as u16) / 6) as u8 + 128;
                diff_rgba.push(gray);
                diff_rgba.push(gray);
                diff_rgba.push(gray);
                diff_rgba.push(255);
            }
        }

        Some(diff_rgba)
    }

    /// Saves an RGBA pixmap to a PNG file on disk.
    pub fn save_png<P: AsRef<Path>>(
        rgba: &[u8],
        width: u32,
        height: u32,
        path: P,
    ) -> Result<(), String> {
        let pixmap = tiny_skia::PixmapRef::from_bytes(rgba, width, height)
            .ok_or_else(|| "Invalid pixmap bytes".to_string())?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        pixmap
            .save_png(path.as_ref())
            .map_err(|e| format!("Failed to encode/save PNG: {e}"))
    }
}
