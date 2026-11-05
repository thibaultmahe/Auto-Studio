use crate::{error::invalid, imaging::bounds, Composition, Result};
use image::{
    imageops::{self, FilterType},
    GrayImage, Rgba, RgbaImage,
};

pub const BACKGROUND_VERSION: u32 = 1;
pub fn background(width: u32, height: u32, dark: bool) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        let v = y as f32 / height as f32;
        let center = 1.0 - (2.0 * x as f32 / width as f32 - 1.0).abs();
        let level = if dark {
            33.0 + 12.0 * v + 6.0 * center
        } else {
            223.0 + 16.0 * (1.0 - v) + 7.0 * center
        };
        let level = level.clamp(0.0, 255.0) as u8;
        Rgba([level, level, level, 255])
    })
}

pub fn render(cutout: &RgbaImage, mask: &GrayImage, p: &Composition) -> Result<RgbaImage> {
    p.validate()?;
    if cutout.dimensions() != mask.dimensions() {
        return Err(invalid("Masque et source incompatibles."));
    }
    let (bx, by, bw, bh) = bounds(mask).ok_or_else(|| invalid("Le masque est vide."))?;
    let mut canvas = background(p.width, p.height, p.background == "STUDIO_FONCE");
    let margin_x = (p.width as f32 * p.margin).ceil();
    let margin_y = (p.height as f32 * p.margin).ceil();
    let ground = (p.height as f32 * p.ground).floor();
    let ratio = ((p.width as f32 - 2.0 * margin_x) / bw as f32)
        .min((ground - margin_y) / bh as f32)
        * p.scale;
    let w = (bw as f32 * ratio).floor().max(1.0) as u32;
    let h = (bh as f32 * ratio).floor().max(1.0) as u32;
    let x = (margin_x + (p.width as f32 - 2.0 * margin_x - w as f32) * p.horizontal).round() as u32;
    let y = ground as u32 - h;
    if x + w > p.width || y + h > p.height {
        return Err(invalid("Le cadrage couperait le sujet."));
    }
    if p.shadow {
        let cx = x as f32 + w as f32 / 2.0;
        let cy = ground + p.shadow_offset * p.height as f32;
        let sx = (w as f32 * 0.33).max(1.0);
        let sy = (p.height as f32 * p.shadow_blur).max(1.0);
        for (px, py, pixel) in canvas.enumerate_pixels_mut() {
            let d = ((px as f32 - cx) / sx).powi(2) + ((py as f32 - cy) / sy).powi(2);
            let alpha = p.shadow_opacity * (-d * 0.5).exp();
            for c in &mut pixel.0[..3] {
                *c = (*c as f32 * (1.0 - alpha)).round() as u8;
            }
        }
    }
    let mut subject = imageops::crop_imm(cutout, bx, by, bw, bh).to_image();
    // Redimensionnement prémultiplié pour éviter les franges colorées.
    for pixel in subject.pixels_mut() {
        for c in 0..3 {
            pixel[c] = (u16::from(pixel[c]) * u16::from(pixel[3]) / 255) as u8;
        }
    }
    let mut subject = imageops::resize(&subject, w, h, FilterType::Lanczos3);
    for pixel in subject.pixels_mut() {
        if pixel[3] > 0 {
            for c in 0..3 {
                pixel[c] = (u32::from(pixel[c]) * 255 / u32::from(pixel[3])).min(255) as u8;
            }
        }
    }
    imageops::overlay(&mut canvas, &subject, i64::from(x), i64::from(y));
    Ok(canvas)
}
