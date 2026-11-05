use crate::{error::invalid, model::Limits, Result};
use image::{DynamicImage, GrayImage, ImageDecoder, ImageFormat, ImageReader, RgbaImage};
use std::io::Cursor;

pub struct Decoded {
    pub image: RgbaImage,
    pub extension: &'static str,
}
pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Decoded> {
    limits.validate()?;
    decode_bounded(bytes, limits.bytes(), limits.pixels())
}
pub fn decode_stored(bytes: &[u8]) -> Result<RgbaImage> {
    Ok(decode_bounded(bytes, 256 * 1024 * 1024, 40_000_000)?.image)
}
fn decode_bounded(bytes: &[u8], max_bytes: u64, max_pixels: u64) -> Result<Decoded> {
    if bytes.len() as u64 > max_bytes {
        return Err(invalid(
            "Le fichier dépasse la limite de taille configurée.",
        ));
    }
    let format = image::guess_format(bytes)?;
    let extension = match format {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
        ImageFormat::WebP => "webp",
        _ => return Err(invalid("Seuls JPEG, PNG et WebP sont acceptés.")),
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut budget = image::Limits::default();
    budget.max_alloc = Some(max_pixels * 8);
    budget.max_image_width = Some(16_000);
    budget.max_image_height = Some(16_000);
    reader.limits(budget);
    let mut decoder = reader.into_decoder()?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > max_pixels {
        return Err(invalid("La résolution dépasse la limite configurée."));
    }
    let orientation = decoder.orientation()?;
    if decoder.icc_profile()?.is_some() {
        return Err(invalid("Les profils ICC intégrés ne sont pas encore pris en charge. Importez une copie convertie en sRGB sans profil intégré pour éviter une dérive des couleurs."));
    }
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(Decoded {
        image: img.to_rgba8(),
        extension,
    })
}
pub fn png(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, ImageFormat::Png)?;
    Ok(out.into_inner())
}
pub fn remote_mask(
    bytes: &[u8],
    source: &RgbaImage,
    limits: &Limits,
) -> Result<(GrayImage, RgbaImage, Vec<String>)> {
    if image::guess_format(bytes)? != ImageFormat::Png {
        return Err(invalid("Le fournisseur n’a pas retourné un PNG."));
    }
    // La couleur originale est préservée ; seul l’alpha distant est utilisé.
    let remote = decode(bytes, limits)?.image;
    if remote.dimensions() != source.dimensions() {
        return Err(invalid(
            "Le résultat distant n’a pas les dimensions et l’orientation de la source.",
        ));
    }
    if remote.pixels().all(|p| p[3] == 255) {
        return Err(invalid(
            "Le résultat distant ne contient aucune transparence effective.",
        ));
    }
    let mut mask = GrayImage::new(source.width(), source.height());
    let mut cutout = source.clone();
    for ((m, c), r) in mask
        .pixels_mut()
        .zip(cutout.pixels_mut())
        .zip(remote.pixels())
    {
        m[0] = ((u16::from(r[3]) * u16::from(c[3])) / 255) as u8;
        c[3] = m[0];
    }
    let warnings = quality(&mask)?;
    Ok((mask, cutout, warnings))
}
pub fn bounds(mask: &GrayImage) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (mask.width(), mask.height(), 0, 0);
    let mut found = false;
    for (x, y, p) in mask.enumerate_pixels() {
        if p[0] > 0 {
            found = true;
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if found {
        Some((x0, y0, x1 - x0 + 1, y1 - y0 + 1))
    } else {
        None
    }
}
pub fn quality(mask: &GrayImage) -> Result<Vec<String>> {
    let (x, y, w, h) =
        bounds(mask).ok_or_else(|| invalid("Le masque est vide : aucun sujet détecté."))?;
    let mut warnings = Vec::new();
    let area = mask.pixels().filter(|p| p[0] > 127).count() as f64
        / (u64::from(mask.width()) * u64::from(mask.height())) as f64;
    if area < 0.03 {
        warnings.push("Sujet très petit : vérifier le détourage.".into());
    }
    if area > 0.95 {
        warnings.push("Masque presque plein : vérifier le fond restant.".into());
    }
    if x == 0 || y == 0 || x + w == mask.width() || y + h == mask.height() {
        warnings.push(
            "Le sujet touche un bord ; vérifier qu’il n’est pas coupé dans l’original.".into(),
        );
    }
    if mask.width() < 1200 || mask.height() < 900 {
        warnings.push("Source peu définie pour un export catalogue.".into());
    }
    // Indicateur de composantes à petite résolution, jamais un score de qualité.
    let small = image::imageops::resize(mask, 128, 128, image::imageops::FilterType::Nearest);
    let mut visited = vec![false; 128 * 128];
    let mut components = 0;
    for i in 0..visited.len() {
        if visited[i] || small.as_raw()[i] < 128 {
            continue;
        }
        let mut stack = vec![i];
        visited[i] = true;
        let mut count = 0;
        while let Some(k) = stack.pop() {
            count += 1;
            for next in [
                if k % 128 > 0 { Some(k - 1) } else { None },
                if k % 128 < 127 { Some(k + 1) } else { None },
                k.checked_sub(128),
                if k + 128 < 16384 { Some(k + 128) } else { None },
            ]
            .into_iter()
            .flatten()
            {
                if !visited[next] && small.as_raw()[next] >= 128 {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        if count >= 3 {
            components += 1;
        }
    }
    if components > 1 {
        warnings.push(
            "Éléments détachés possibles : contrôler antennes, rétroviseurs et parasites.".into(),
        );
    }
    Ok(warnings)
}
