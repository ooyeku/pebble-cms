use anyhow::{bail, Result};
use image::codecs::webp::WebPEncoder;
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::io::Cursor;

const MAX_WIDTH: u32 = 1600;
const THUMBNAIL_SIZE: u32 = 200;

pub struct OptimizedImage {
    pub original: Vec<u8>,
    pub original_format: ImageFormat,
    pub webp: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct ImageVariant {
    pub width: u32,
    pub data: Vec<u8>,
    pub suffix: String,
}

pub fn is_optimizable_image(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/jpeg" | "image/png" | "image/gif" | "image/webp"
    )
}

pub fn optimize_image(
    data: &[u8],
    mime_type: &str,
    max_width: Option<u32>,
) -> Result<OptimizedImage> {
    let format = match mime_type {
        "image/jpeg" => ImageFormat::Jpeg,
        "image/png" => ImageFormat::Png,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::WebP,
        _ => bail!("Unsupported image format: {}", mime_type),
    };

    let img = image::load_from_memory_with_format(data, format)?;
    let (orig_width, orig_height) = img.dimensions();

    let max_w = max_width.unwrap_or(MAX_WIDTH);
    let resized = if orig_width > max_w {
        let ratio = max_w as f32 / orig_width as f32;
        let new_height = (orig_height as f32 * ratio) as u32;
        img.resize(max_w, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        img.clone()
    };

    let (final_width, final_height) = resized.dimensions();

    let original = encode_image(&resized, format)?;
    let webp = encode_webp(&resized)?;

    Ok(OptimizedImage {
        original,
        original_format: format,
        webp,
        width: final_width,
        height: final_height,
    })
}

pub fn generate_thumbnail(data: &[u8], size: Option<u32>) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    let thumb_size = size.unwrap_or(THUMBNAIL_SIZE);

    let thumbnail = img.resize_to_fill(
        thumb_size,
        thumb_size,
        image::imageops::FilterType::Lanczos3,
    );

    encode_webp(&thumbnail)
}

pub fn generate_srcset_variants(data: &[u8]) -> Result<Vec<ImageVariant>> {
    let widths = [400, 800, 1200, 1600];
    let img = image::load_from_memory(data)?;
    let (orig_width, _) = img.dimensions();

    let mut variants = Vec::new();

    for &width in &widths {
        if width > orig_width {
            continue;
        }

        let ratio = width as f32 / orig_width as f32;
        let new_height = (img.height() as f32 * ratio) as u32;
        let resized = img.resize(width, new_height, image::imageops::FilterType::Lanczos3);

        let webp_data = encode_webp(&resized)?;

        variants.push(ImageVariant {
            width,
            data: webp_data,
            suffix: format!("-{}w", width),
        });
    }

    Ok(variants)
}

fn encode_image(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());

    match format {
        ImageFormat::Jpeg => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 85);
            img.write_with_encoder(encoder)?;
        }
        ImageFormat::Png => {
            img.write_to(&mut buffer, ImageFormat::Png)?;
        }
        ImageFormat::Gif => {
            img.write_to(&mut buffer, ImageFormat::Gif)?;
        }
        ImageFormat::WebP => {
            return encode_webp(img);
        }
        _ => bail!("Unsupported format for encoding"),
    }

    Ok(buffer.into_inner())
}

fn encode_webp(img: &DynamicImage) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let mut buffer = Cursor::new(Vec::new());
    let encoder = WebPEncoder::new_lossless(&mut buffer);
    encoder.encode(&rgba, width, height, image::ExtendedColorType::Rgba8)?;

    Ok(buffer.into_inner())
}
