use image::{DynamicImage, GrayImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use serde_json::json;
use std::{
    fs,
    io::Cursor,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use studio_core::{
    compose, imaging,
    provider::{RemoteTask, SegmentationProvider},
    storage, *,
};
use tempfile::TempDir;

fn source() -> RgbaImage {
    RgbaImage::from_fn(80, 60, |x, y| Rgba([x as u8 * 3, y as u8 * 4, 91, 255]))
}
fn remote() -> Vec<u8> {
    imaging::png(&DynamicImage::ImageRgba8(RgbaImage::from_fn(
        80,
        60,
        |x, y| {
            Rgba([
                0,
                0,
                0,
                if (10..70).contains(&x) && (20..50).contains(&y) {
                    255
                } else {
                    0
                },
            ])
        },
    )))
    .unwrap()
}
fn fixture(path: &Path) {
    fs::write(
        path,
        imaging::png(&DynamicImage::ImageRgba8(source())).unwrap(),
    )
    .unwrap();
}
fn pin() -> ProviderPin {
    ProviderPin {
        app_id: "birefnet-app".into(),
        version_id: "v-pinned".into(),
        function: None,
    }
}
struct Provider {
    calls: AtomicUsize,
    ambiguous: bool,
}
impl Provider {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            ambiguous: false,
        }
    }
}
impl SegmentationProvider for Provider {
    fn describe(&self, _: &str) -> Result<ProviderPin> {
        Ok(pin())
    }
    fn upload(&self, _: &str, _: Vec<u8>) -> Result<String> {
        Ok("inf://files/source-1".into())
    }
    fn submit(&self, _: &str, _: &ProviderPin, _: &str) -> Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.ambiguous {
            Err(Error::Network)
        } else {
            Ok("task-1".into())
        }
    }
    fn task(&self, _: &str, id: &str) -> Result<RemoteTask> {
        Ok(RemoteTask {
            id: id.into(),
            status: 10,
            app_id: pin().app_id,
            app_version_id: pin().version_id,
            input: json!({"image":"inf://files/source-1"}),
        })
    }
    fn result(&self, _: &str, _: &str, _: u64) -> Result<Vec<u8>> {
        Ok(remote())
    }
}
fn setup() -> (TempDir, Library, Photo, Provider) {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("voiture.png");
    fixture(&input);
    let lib = Library::open(dir.path().join("library")).unwrap();
    let p = lib.import(&input).unwrap();
    let provider = Provider::new();
    (dir, lib, p, provider)
}
fn segmented(lib: &Library, p: &Photo, provider: &Provider) {
    lib.classify(&p.id, Category::Exterior).unwrap();
    lib.pin_provider(provider, "test-only").unwrap();
    lib.submit(&p.id, &pin().version_id, provider, "test-only")
        .unwrap();
    lib.synchronize(&p.id, None, provider, "test-only").unwrap();
}
#[test]
fn end_to_end_cache_composition_review_and_exports() {
    let (dir, lib, p, provider) = setup();
    assert_eq!(p.category, Category::Unknown);
    assert!(lib.plan(&p.id).is_err());
    assert_eq!(
        lib.import(&dir.path().join("voiture.png")).unwrap().id,
        p.id
    );
    segmented(&lib, &p, &provider);
    assert_eq!(lib.plan(&p.id).unwrap().cached, 1);
    let c = Composition {
        width: 256,
        height: 256,
        ..Composition::default()
    };
    let composed = lib.compose(&p.id, &c).unwrap();
    assert_eq!(composed.review, "NEEDS_REVIEW");
    assert!(lib
        .export(&p.id, dir.path(), ExportFormat::Webp, 90)
        .is_err());
    lib.review(&p.id, true).unwrap();
    for f in [
        ExportFormat::Webp,
        ExportFormat::Jpeg,
        ExportFormat::Cutout,
        ExportFormat::Mask,
    ] {
        let path = lib.export(&p.id, dir.path(), f, 90).unwrap();
        assert!(image::open(path).is_ok());
    }
    let a = lib
        .export(&p.id, dir.path(), ExportFormat::Original, 90)
        .unwrap();
    let b = lib
        .export(&p.id, dir.path(), ExportFormat::Original, 90)
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(
        fs::read(&a).unwrap(),
        fs::read(dir.path().join("voiture.png")).unwrap()
    );
    let dark = Composition {
        background: "STUDIO_FONCE".into(),
        ..c
    };
    let updated = lib.compose(&p.id, &dark).unwrap();
    assert_ne!(updated.render_key, composed.render_key);
    assert_eq!(updated.review, "NEEDS_REVIEW");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    lib.submit(&p.id, &pin().version_id, &provider, "test-only")
        .unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let mut settings = lib.settings().unwrap();
    settings.provider.as_mut().unwrap().version_id = "v-other".into();
    lib.save_settings(&settings).unwrap();
    assert_eq!(lib.plan(&p.id).unwrap().cached, 0);
    lib.review(&p.id, false).unwrap();
    assert!(lib
        .export(&p.id, dir.path(), ExportFormat::Webp, 90)
        .is_err());
    // Un nouvel envoi avec une autre version invalide aussi l’ancienne approbation.
    lib.review(&p.id, true).unwrap();
    let pending = lib
        .submit(&p.id, "v-other", &provider, "test-only")
        .unwrap();
    assert_eq!(pending.review, "NEEDS_REVIEW");
    assert!(lib
        .export(&p.id, dir.path(), ExportFormat::Webp, 90)
        .is_err());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
}
#[test]
fn ambiguous_submission_survives_restart_without_double_charge() {
    let (dir, lib, p, mut provider) = setup();
    provider.ambiguous = true;
    lib.classify(&p.id, Category::Exterior).unwrap();
    lib.pin_provider(&provider, "test-only").unwrap();
    assert!(lib
        .submit(&p.id, &pin().version_id, &provider, "test-only")
        .is_err());
    drop(lib);
    let lib = Library::open(dir.path().join("library")).unwrap();
    assert!(lib
        .submit(&p.id, &pin().version_id, &provider, "test-only")
        .is_err());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    lib.synchronize(&p.id, Some("task-1"), &provider, "test-only")
        .unwrap();
    assert_eq!(lib.photo(&p.id).unwrap().mask_version, 1);
    assert_eq!(lib.submissions().unwrap().len(), 1);
}
#[test]
fn double_click_and_remote_recovery() {
    let (dir, lib, p, provider) = setup();
    lib.classify(&p.id, Category::Exterior).unwrap();
    lib.pin_provider(&provider, "test-only").unwrap();
    lib.submit(&p.id, &pin().version_id, &provider, "test-only")
        .unwrap();
    assert!(lib
        .submit(&p.id, &pin().version_id, &provider, "test-only")
        .is_err());
    drop(lib);
    let lib = Library::open(dir.path().join("library")).unwrap();
    assert_eq!(
        lib.photo(&p.id).unwrap().remote_id.as_deref(),
        Some("task-1")
    );
    lib.synchronize(&p.id, None, &provider, "test-only")
        .unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}
#[test]
fn masks_use_source_colors_and_detect_invalid_results() {
    let (mask, rgba, _) = imaging::remote_mask(&remote(), &source(), &Limits::default()).unwrap();
    assert_eq!(mask[(20, 30)][0], 255);
    assert_eq!(rgba[(20, 30)], source()[(20, 30)]);
    assert_eq!(rgba[(0, 0)][3], 0);
    let encoded = imaging::png(&DynamicImage::ImageLuma8(mask)).unwrap();
    let roundtrip = image::load_from_memory(&encoded).unwrap().to_luma8();
    assert_eq!(roundtrip[(20, 30)][0], 255);
    let opaque = imaging::png(&DynamicImage::ImageRgba8(source())).unwrap();
    assert!(imaging::remote_mask(&opaque, &source(), &Limits::default()).is_err());
    let wrong = imaging::png(&DynamicImage::ImageRgba8(RgbaImage::new(10, 10))).unwrap();
    assert!(imaging::remote_mask(&wrong, &source(), &Limits::default()).is_err());
    assert!(imaging::quality(&GrayImage::new(10, 10)).is_err());
}
#[test]
fn proportional_framing_does_not_crop_subject() {
    let mut mask = GrayImage::new(100, 80);
    let mut cut = RgbaImage::new(100, 80);
    for y in 10..70 {
        for x in 20..80 {
            mask[(x, y)][0] = 255;
            cut[(x, y)] = Rgba([255, 0, 0, 255]);
        }
    }
    for horizontal in [0.0, 0.5, 1.0] {
        for ground in [0.55, 0.88, 0.95] {
            let p = Composition {
                width: 256,
                height: 320,
                horizontal,
                ground,
                shadow: false,
                ..Composition::default()
            };
            let img = compose::render(&cut, &mask, &p).unwrap();
            let mut reds = GrayImage::new(256, 320);
            for (x, y, pixel) in img.enumerate_pixels() {
                if pixel[0] > 200 && pixel[1] < 20 {
                    reds[(x, y)][0] = 255;
                }
            }
            let (x, y, w, h) = imaging::bounds(&reds).unwrap();
            assert!(x > 0 && y > 0 && x + w < 256 && y + h < 320);
            assert!((w as i32 - h as i32).abs() <= 1);
        }
    }
    let p = Composition {
        width: 256,
        height: 256,
        ..Composition::default()
    };
    assert_eq!(
        compose::render(&cut, &mask, &p).unwrap(),
        compose::render(&cut, &mask, &p).unwrap()
    );
    let no_shadow = Composition {
        shadow: false,
        ..p.clone()
    };
    assert_ne!(
        compose::render(&cut, &mask, &p).unwrap(),
        compose::render(&cut, &mask, &no_shadow).unwrap()
    );
}
#[test]
fn content_limits_rgb_rgba_and_exif_orientation() {
    assert!(imaging::decode(b"not an image.png", &Limits::default()).is_err());
    let rgb = RgbImage::from_pixel(80, 60, Rgb([220, 80, 30]));
    for img in [
        DynamicImage::ImageRgb8(rgb.clone()),
        DynamicImage::ImageRgba8(source()),
    ] {
        for format in [ImageFormat::Png, ImageFormat::WebP, ImageFormat::Jpeg] {
            let mut buf = Cursor::new(Vec::new());
            let img = if format == ImageFormat::Jpeg {
                DynamicImage::ImageRgb8(img.to_rgb8())
            } else {
                img.clone()
            };
            img.write_to(&mut buf, format).unwrap();
            assert_eq!(
                imaging::decode(buf.get_ref(), &Limits::default())
                    .unwrap()
                    .image
                    .dimensions(),
                (80, 60)
            );
        }
    }
    let mut jpeg = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(rgb)
        .write_to(&mut jpeg, ImageFormat::Jpeg)
        .unwrap();
    let mut exif =
        b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0".to_vec();
    let len = (exif.len() + 2) as u16;
    let mut oriented = vec![0xff, 0xd8, 0xff, 0xe1];
    oriented.extend(len.to_be_bytes());
    oriented.append(&mut exif);
    oriented.extend(&jpeg.get_ref()[2..]);
    assert_eq!(
        imaging::decode(&oriented, &Limits::default())
            .unwrap()
            .image
            .dimensions(),
        (60, 80)
    );
    let png = imaging::png(&DynamicImage::ImageRgba8(RgbaImage::new(1100, 1000))).unwrap();
    assert!(imaging::decode(
        &png,
        &Limits {
            max_file_mib: 32,
            max_megapixels: 1
        }
    )
    .is_err());
    assert!(Limits {
        max_file_mib: 0,
        max_megapixels: 24
    }
    .validate()
    .is_err());
}
#[test]
fn migration_persistence_path_validation_and_library_lock() {
    let (dir, lib, p, _) = setup();
    assert!(Library::open(dir.path().join("library")).is_err());
    assert!(lib.photo("../../outside").is_err());
    assert!(lib
        .preview("../../outside", Asset::Original, false)
        .is_err());
    assert!(
        storage::safe_export(dir.path(), "../../A:CON", "png", b"safe")
            .unwrap()
            .starts_with(dir.path())
    );
    let settings = Settings {
        limits: Limits {
            max_file_mib: 16,
            max_megapixels: 12,
        },
        provider: Some(pin()),
    };
    lib.save_settings(&settings).unwrap();
    drop(lib);
    let lib = Library::open(dir.path().join("library")).unwrap();
    assert_eq!(lib.photo(&p.id).unwrap().sha256, p.sha256);
    assert_eq!(lib.settings().unwrap().limits.max_file_mib, 16);
    let db = rusqlite::Connection::open(dir.path().join("library/studio.sqlite3")).unwrap();
    let v: u32 = db
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(v, 1);
    assert!(!fs::read(dir.path().join("library/studio.sqlite3"))
        .unwrap()
        .windows(9)
        .any(|s| s == b"test-only"));
}
#[test]
fn category_changes_invalidate_commercial_render() {
    let (dir, lib, p, provider) = setup();
    segmented(&lib, &p, &provider);
    lib.compose(
        &p.id,
        &Composition {
            width: 256,
            height: 256,
            ..Composition::default()
        },
    )
    .unwrap();
    lib.review(&p.id, true).unwrap();
    let interior = lib.classify(&p.id, Category::Interior).unwrap();
    assert!(interior.render_key.is_none());
    assert!(lib.compose(&p.id, &Composition::default()).is_err());
    assert!(lib
        .export(&p.id, dir.path(), ExportFormat::Webp, 90)
        .is_err());
    assert!(lib
        .export(&p.id, dir.path(), ExportFormat::Original, 90)
        .is_ok());
}
