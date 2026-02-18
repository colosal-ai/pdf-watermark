use anyhow::{anyhow, Result};
use image::{DynamicImage, RgbaImage};
use std::io::Cursor;

const WM_MAX_W: u32 = 120;
const WM_MIN_W: u32 = 107;
const WM_MIN_H: u32 = 21;
const WM_OPACITY: f32 = 1.0;
const WM_MARGIN: u32 = 0;

pub enum Quality {
    Lossless,
    Jpeg(u8),
}

pub fn parse_quality(s: &str) -> Result<Quality> {
    if s == "lossless" {
        Ok(Quality::Lossless)
    } else {
        let q: u8 = s
            .parse()
            .map_err(|_| anyhow!("--quality debe ser 'lossless' o un número 1-100"))?;
        if !(1..=100).contains(&q) {
            return Err(anyhow!("--quality debe estar entre 1 y 100"));
        }
        Ok(Quality::Jpeg(q))
    }
}

pub fn prepare_from_bytes(data: &[u8]) -> Result<RgbaImage> {
    let cursor = Cursor::new(data);
    let logo = image::load(cursor, image::ImageFormat::Png)
        .or_else(|_| {
            let cursor = Cursor::new(data);
            image::load(cursor, image::ImageFormat::Jpeg)
        })
        .or_else(|_| image::load_from_memory(data))?
        .into_rgba8();
    prepare_logo(logo)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn prepare(logo_path: &str) -> Result<RgbaImage> {
    let logo = image::open(logo_path)?.into_rgba8();
    prepare_logo(logo)
}

fn prepare_logo(logo: RgbaImage) -> Result<RgbaImage> {
    let (orig_w, orig_h) = logo.dimensions();
    let (new_w, new_h) = calc_size(orig_w, orig_h);

    let resized = image::imageops::resize(
        &logo,
        new_w,
        new_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut result = resized;
    if WM_OPACITY < 1.0 {
        for pixel in result.pixels_mut() {
            pixel[3] = (pixel[3] as f32 * WM_OPACITY) as u8;
        }
    }

    Ok(result)
}

pub fn apply(page: &DynamicImage, wm: &RgbaImage) -> DynamicImage {
    let mut canvas = page.to_rgba8();
    let (pw, ph) = canvas.dimensions();
    let (ww, wh) = wm.dimensions();

    let x = (pw - ww - WM_MARGIN) as i64;
    let y = (ph - wh - WM_MARGIN) as i64;

    image::imageops::overlay(&mut canvas, wm, x, y);
    DynamicImage::ImageRgba8(canvas)
}

fn calc_size(orig_w: u32, orig_h: u32) -> (u32, u32) {
    let ratio = orig_h as f64 / orig_w as f64;

    let mut new_w = WM_MAX_W;
    let mut new_h = (new_w as f64 * ratio).round() as u32;

    if new_h < WM_MIN_H {
        new_h = WM_MIN_H;
        new_w = (new_h as f64 / ratio).round() as u32;
    }

    if new_w < WM_MIN_W {
        new_w = WM_MIN_W;
        new_h = (new_w as f64 * ratio).round() as u32;
    }

    (new_w, new_h)
}
