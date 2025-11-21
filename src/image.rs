use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use iced::widget;

use crate::dataset::DatasetState;

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn decode_image(path: &Path) -> Result<DecodedImage> {
    let img = image::open(path).with_context(|| format!("Open image {}", path.display()))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        width,
        height,
        rgba: rgba.into_raw(),
    })
}

pub fn apply_brightness_contrast(
    image: &DecodedImage,
    brightness: f32,
    contrast: f32,
) -> widget::image::Handle {
    let mut adjusted = image.rgba.clone();
    let b = brightness.max(0.0);
    let c = contrast.max(0.0);
    for chunk in adjusted.chunks_mut(4) {
        let apply = |v: u8| {
            let mut val = v as f32 * b;
            val = ((val - 128.0) * c) + 128.0;
            val.clamp(0.0, 255.0) as u8
        };
        chunk[0] = apply(chunk[0]);
        chunk[1] = apply(chunk[1]);
        chunk[2] = apply(chunk[2]);
    }

    widget::image::Handle::from_rgba(image.width, image.height, adjusted)
}

pub fn load_image_with_cache(
    dataset: &DatasetState,
    file: &str,
    brightness: f32,
    contrast: f32,
    cache: &mut HashMap<PathBuf, DecodedImage>,
) -> Option<widget::image::Handle> {
    let path = dataset.image_path(file);
    let decoded = if let Some(existing) = cache.get(&path) {
        existing.clone()
    } else {
        let loaded = decode_image(&path).ok()?;
        cache.insert(path.clone(), loaded.clone());
        loaded
    };

    Some(apply_brightness_contrast(&decoded, brightness, contrast))
}
