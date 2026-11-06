//! Artwork import, centre-cropped thumbnail generation and an in-memory texture
//! cache. Covers are completely independent from the `Icon=` key of the
//! desktop file; source desktop files are never modified.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::{DynamicImage, ImageFormat, ImageReader, RgbaImage};

use crate::paths::XdgPaths;

/// Refuse absurdly large source files before handing them to the decoder.
/// This keeps an accidental 2 GB PNG from exhausting memory.
const MAX_SOURCE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkKind {
    Cover,
    Background,
    Avatar,
    Icon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Target {
    width: u32,
    height: u32,
    fit: bool,
}

impl ArtworkKind {
    fn target(self) -> Target {
        match self {
            Self::Cover => Target {
                // 3:4 is allowed by the design spec and leaves room for two
                // rows on a 1080p TV.
                width: 720,
                height: 960,
                fit: false,
            },
            Self::Background => Target {
                width: 2560,
                height: 1440,
                fit: false,
            },
            Self::Avatar => Target {
                width: 512,
                height: 512,
                fit: false,
            },
            Self::Icon => Target {
                width: 512,
                height: 512,
                fit: true,
            },
        }
    }

    fn subdir(self) -> &'static str {
        match self {
            Self::Cover => "covers",
            Self::Background => "backgrounds",
            Self::Avatar => "avatars",
            Self::Icon => "icons",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    path: PathBuf,
    target: Target,
    modified_ns: u128,
    len: u64,
}

#[derive(Debug, Clone)]
struct Failure {
    modified_ns: u128,
    len: u64,
    message: String,
}

#[derive(Default)]
pub struct ArtworkCache {
    textures: HashMap<CacheKey, TextureHandle>,
    failures: HashMap<PathBuf, Failure>,
}

impl ArtworkCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Last decode error for diagnostics. The UI still renders a neutral tile.
    pub fn last_error(&self, path: &Path) -> Option<&str> {
        self.failures.get(path).map(|f| f.message.as_str())
    }

    /// Load and crop/fit an image as an egui texture. Returns `None` for
    /// missing or unsupported files; this is never fatal.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        paths: &XdgPaths,
        path: &Path,
        kind: ArtworkKind,
    ) -> Option<TextureHandle> {
        let target = kind.target();
        let metadata = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(err) => {
                crate::logging::log_warn(
                    "artwork",
                    format!("Bild fehlt: {} ({err})", path.display()),
                );
                self.fail(path, 0, 0, format!("Bild fehlt: {err}"));
                return None;
            }
        };
        if !metadata.is_file() {
            self.fail(path, 0, 0, "Bildpfad ist keine Datei.".to_owned());
            return None;
        }
        if metadata.len() > MAX_SOURCE_BYTES {
            let message = format!(
                "Bild ist zu groß ({} MiB, Maximum {} MiB).",
                metadata.len() / (1024 * 1024),
                MAX_SOURCE_BYTES / (1024 * 1024)
            );
            crate::logging::log_warn("artwork", format!("{}: {message}", path.display()));
            self.fail(path, 0, metadata.len(), message);
            return None;
        }
        let modified_ns = metadata
            .modified()
            .ok()
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let len = metadata.len();
        if let Some(failure) = self.failures.get(path) {
            if failure.modified_ns == modified_ns && failure.len == len {
                return None;
            }
        }
        let key = CacheKey {
            path: path.to_path_buf(),
            target,
            modified_ns,
            len,
        };
        if let Some(texture) = self.textures.get(&key) {
            return Some(texture.clone());
        }

        let cache_path = thumbnail_cache_path(paths, &key);
        let image = if cache_path.is_file() {
            let decoded = (|| -> image::ImageResult<DynamicImage> {
                let reader = ImageReader::open(&cache_path)?.with_guessed_format()?;
                reader.decode()
            })();
            match decoded {
                Ok(image) => Some(image),
                Err(_) => {
                    let _ = fs::remove_file(&cache_path);
                    None
                }
            }
        } else {
            None
        };
        let image = match image {
            Some(image) => image,
            None => match decode_and_prepare(path, target) {
                Ok(image) => {
                    let _ = save_thumbnail(&cache_path, &image);
                    image
                }
                Err(message) => {
                    crate::logging::log_warn("artwork", format!("{}: {message}", path.display()));
                    self.fail(path, modified_ns, len, message);
                    return None;
                }
            },
        };

        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let name = format!("artwork-{:x}", hash_key(&key));
        let texture = ctx.load_texture(name, color, TextureOptions::LINEAR);
        self.failures.remove(path);
        self.textures.insert(key, texture.clone());
        Some(texture)
    }

    fn fail(&mut self, path: &Path, modified_ns: u128, len: u64, message: String) {
        self.failures.insert(
            path.to_path_buf(),
            Failure {
                modified_ns,
                len,
                message,
            },
        );
    }
}

fn hash_key(key: &CacheKey) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn thumbnail_cache_path(paths: &XdgPaths, key: &CacheKey) -> PathBuf {
    paths.thumbnails_dir().join(format!(
        "{:016x}-{}x{}{}.png",
        hash_key(key),
        key.target.width,
        key.target.height,
        if key.target.fit { "-fit" } else { "" }
    ))
}

fn save_thumbnail(path: &Path, image: &DynamicImage) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    image
        .save_with_format(&tmp, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        e.to_string()
    })
}

fn decode_and_prepare(path: &Path, target: Target) -> Result<DynamicImage, String> {
    let reader = ImageReader::open(path)
        .map_err(|err| format!("Bild kann nicht geöffnet werden: {err}"))?
        .with_guessed_format()
        .map_err(|err| format!("Bildformat nicht erkennbar: {err}"))?;
    let image = reader
        .decode()
        .map_err(|err| format!("Bild kann nicht dekodiert werden: {err}"))?;
    Ok(prepare_image(&image, target))
}

fn prepare_image(image: &DynamicImage, target: Target) -> DynamicImage {
    let rgba = image.to_rgba8();
    let (sw, sh) = rgba.dimensions();
    if sw == 0 || sh == 0 {
        return DynamicImage::ImageRgba8(RgbaImage::new(target.width, target.height));
    }
    if target.fit {
        let scaled = DynamicImage::ImageRgba8(rgba).resize(
            target.width,
            target.height,
            image::imageops::FilterType::Triangle,
        );
        let mut canvas = RgbaImage::new(target.width, target.height);
        let ox = (target.width.saturating_sub(scaled.width())) / 2;
        let oy = (target.height.saturating_sub(scaled.height())) / 2;
        image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), ox as i64, oy as i64);
        return DynamicImage::ImageRgba8(canvas);
    }

    let wanted = target.width as f64 / target.height as f64;
    let have = sw as f64 / sh as f64;
    let (cw, ch) = if have > wanted {
        let cw = ((sh as f64 * wanted).round() as u32).max(1).min(sw);
        (cw, sh)
    } else {
        let ch = ((sw as f64 / wanted).round() as u32).max(1).min(sh);
        (sw, ch)
    };
    let x = (sw - cw) / 2;
    let y = (sh - ch) / 2;
    let cropped = image::imageops::crop_imm(&rgba, x, y, cw, ch).to_image();
    DynamicImage::ImageRgba8(cropped).resize_exact(
        target.width,
        target.height,
        image::imageops::FilterType::Triangle,
    )
}

fn extension_for_format(format: ImageFormat) -> Option<&'static str> {
    match format {
        ImageFormat::Png => Some("png"),
        ImageFormat::Jpeg => Some("jpg"),
        ImageFormat::WebP => Some("webp"),
        _ => None,
    }
}

fn sanitize_slug(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else if ch.is_whitespace() || matches!(ch, '/' | '\\' | '.') {
            out.push('-');
        }
    }
    let out: String = out.trim_matches('-').chars().take(60).collect();
    if out.is_empty() {
        "artwork".to_owned()
    } else {
        out
    }
}

/// Validate and copy a user-selected image into CouchDeck's own asset tree.
/// Returns a path relative to the CouchDeck data directory.
pub fn import_image(
    source: &Path,
    paths: &XdgPaths,
    kind: ArtworkKind,
    slug: &str,
) -> Result<PathBuf, String> {
    let size = fs::metadata(source)
        .map_err(|err| format!("Bild {} kann nicht geprüft werden: {err}", source.display()))?
        .len();
    if size > MAX_SOURCE_BYTES {
        return Err(format!(
            "Bild ist zu groß ({} MiB, Maximum {} MiB).",
            size / (1024 * 1024),
            MAX_SOURCE_BYTES / (1024 * 1024)
        ));
    }
    let reader = ImageReader::open(source)
        .map_err(|err| {
            format!(
                "Bild {} kann nicht geöffnet werden: {err}",
                source.display()
            )
        })?
        .with_guessed_format()
        .map_err(|err| format!("Bildformat nicht erkennbar: {err}"))?;
    let format = reader
        .format()
        .ok_or_else(|| "Bildformat nicht erkennbar (unterstützt: PNG, JPEG, WebP).".to_owned())?;
    let ext = extension_for_format(format)
        .ok_or_else(|| "Nur PNG, JPEG und WebP werden unterstützt.".to_owned())?;
    // Fully decode once so a corrupt file never lands in the assets directory.
    reader
        .decode()
        .map_err(|err| format!("Bild kann nicht dekodiert werden: {err}"))?;

    let target_dir = paths.assets_dir().join(kind.subdir());
    fs::create_dir_all(&target_dir)
        .map_err(|err| format!("Asset-Verzeichnis kann nicht erstellt werden: {err}"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let stem = format!("{}-{stamp}", sanitize_slug(slug));
    let mut final_path = target_dir.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while final_path.exists() {
        final_path = target_dir.join(format!("{stem}-{n}.{ext}"));
        n += 1;
    }
    let tmp = target_dir.join(format!(".import-{}-{stamp}.tmp", std::process::id()));
    fs::copy(source, &tmp).map_err(|err| format!("Bild konnte nicht kopiert werden: {err}"))?;
    if let Err(err) = fs::rename(&tmp, &final_path) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Bild konnte nicht abgelegt werden: {err}"));
    }
    paths
        .data_relative(&final_path)
        .ok_or_else(|| "Interner Fehler: Asset liegt außerhalb des Datenverzeichnisses.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn png(path: &Path, width: u32, height: u32) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut image = RgbImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgb([(x % 255) as u8, (y % 255) as u8, 128]);
        }
        image.save_with_format(path, ImageFormat::Png).unwrap();
    }

    #[test]
    fn import_copies_into_assets_and_returns_relative_path() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let source = tmp.path().join("source.png");
        png(&source, 40, 30);
        let relative = import_image(&source, &paths, ArtworkKind::Cover, "My Game!").unwrap();
        assert!(!relative.is_absolute());
        assert!(relative.starts_with("assets/covers/"));
        let absolute = paths.data_dir().join(&relative);
        assert!(absolute.is_file());
        assert!(!fs::read(&absolute).unwrap().is_empty());
    }

    #[test]
    fn invalid_image_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let source = tmp.path().join("broken.png");
        fs::write(&source, b"definitely not a png").unwrap();
        let err = import_image(&source, &paths, ArtworkKind::Cover, "x").unwrap_err();
        assert!(!err.is_empty());
        assert!(!paths.covers_dir().join("x.png").exists());
    }

    #[test]
    fn cover_is_centre_cropped_to_2_3() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("wide.png");
        png(&source, 400, 200);
        let image = decode_and_prepare(
            &source,
            Target {
                width: 720,
                height: 1080,
                fit: false,
            },
        )
        .unwrap();
        assert_eq!(image.width(), 720);
        assert_eq!(image.height(), 1080);
    }

    #[test]
    fn icon_fit_keeps_aspect_and_pads() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("wide.png");
        png(&source, 400, 200);
        let image = decode_and_prepare(
            &source,
            Target {
                width: 512,
                height: 512,
                fit: true,
            },
        )
        .unwrap();
        assert_eq!(image.width(), 512);
        assert_eq!(image.height(), 512);
    }

    #[test]
    fn sanitize_slug_is_safe() {
        assert_eq!(sanitize_slug("../../etc/passwd"), "etc-passwd");
        assert_eq!(sanitize_slug("My Game"), "My-Game");
    }
}
