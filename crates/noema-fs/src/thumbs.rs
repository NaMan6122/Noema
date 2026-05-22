use std::path::{Path, PathBuf};
use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::ImageFormat;
use noema_core::error::{NoemaError, Result};

pub struct ThumbnailService {
    cache_dir: PathBuf,
    size: u32,
}

impl ThumbnailService {
    pub fn new(cache_dir: PathBuf, size: u32) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir, size })
    }

    fn cache_key(&self, path: &Path) -> Result<String> {
        let meta = std::fs::metadata(path)?;
        let mtime = meta.modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let input = format!("{}:{}", path.display(), mtime);
        Ok(blake3::hash(input.as_bytes()).to_hex().to_string())
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.webp", key))
    }

    pub fn is_supported(path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        matches!(ext.as_deref(), Some("jpg" | "jpeg" | "png" | "gif" | "webp"))
    }

    pub async fn get_thumbnail(&self, path: PathBuf) -> Result<String> {
        let cache_dir = self.cache_dir.clone();
        let size = self.size;

        tokio::task::spawn_blocking(move || {
            Self::generate_sync(&path, &cache_dir, size)
        })
        .await
        .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
    }

    fn generate_sync(path: &Path, cache_dir: &Path, size: u32) -> Result<String> {
        let meta = std::fs::metadata(path)?;
        let mtime = meta.modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let input = format!("{}:{}", path.display(), mtime);
        let key = blake3::hash(input.as_bytes()).to_hex().to_string();
        let cached = cache_dir.join(format!("{}.webp", &key));

        if cached.exists() {
            let bytes = std::fs::read(&cached)?;
            return Ok(format!("data:image/webp;base64,{}", BASE64.encode(&bytes)));
        }

        let img = image::open(path).map_err(|e| NoemaError::Io(
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        ))?;

        let thumb = img.thumbnail(size, size);
        let mut buf = Cursor::new(Vec::new());
        thumb.write_to(&mut buf, ImageFormat::WebP).map_err(|e| NoemaError::Io(
            std::io::Error::new(std::io::ErrorKind::Other, e)
        ))?;

        let bytes = buf.into_inner();
        std::fs::write(&cached, &bytes)?;

        Ok(format!("data:image/webp;base64,{}", BASE64.encode(&bytes)))
    }
}
