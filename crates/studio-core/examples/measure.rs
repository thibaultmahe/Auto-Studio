//! Mesure reproductible du cœur uniquement, avec un masque SYNTHÉTIQUE, sans API.
use image::{DynamicImage, GrayImage, Rgba, RgbaImage};
use std::{fs, path::PathBuf, time::Instant};
use studio_core::{compose, imaging, Composition, Library};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let base = PathBuf::from(args.get(2).ok_or("usage: measure prepare|run DOSSIER")?);
    fs::create_dir_all(&base)?;
    let input = base.join("synthetic-24mp.png");
    if args.get(1).map(String::as_str) == Some("prepare") {
        let image = RgbaImage::from_fn(6000, 4000, |x, y| {
            let inside = (800..5200).contains(&x) && (1600..3400).contains(&y);
            Rgba([
                (x % 256) as u8,
                (y % 256) as u8,
                100,
                if inside { 255 } else { 0 },
            ])
        });
        fs::write(input, imaging::png(&DynamicImage::ImageRgba8(image))?)?;
        return Ok(());
    }
    let begin = Instant::now();
    let lib = Library::open(base.join("library"))?;
    let p = lib.import(&input)?;
    let import_ms = begin.elapsed().as_millis();
    let bytes = fs::read(&input)?;
    let image = imaging::decode(&bytes, &lib.settings()?.limits)?.image;
    drop(bytes);
    let mask = GrayImage::from_fn(image.width(), image.height(), |x, y| {
        image::Luma([image[(x, y)][3]])
    });
    let begin = Instant::now();
    let mut last = String::new();
    for _ in 0..30 {
        let output = compose::render(&image, &mask, &Composition::default())?;
        let bytes = imaging::png(&DynamicImage::ImageRgba8(output))?;
        last = studio_core::storage::hash(&bytes);
    }
    println!(
        "{}",
        serde_json::json!({"fixture":"synthetic-alpha-not-an-inference","pixels":p.width as u64*p.height as u64,"import_ms":import_ms,"renders":30,"renders_total_ms":begin.elapsed().as_millis(),"last_render_sha256":last})
    );
    Ok(())
}
