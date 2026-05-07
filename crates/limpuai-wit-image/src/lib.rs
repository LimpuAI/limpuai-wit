wit_bindgen::generate!({
    world: "image-data",
    path: "../../wit",
});

use std::cell::RefCell;
use std::io::Cursor;

use exports::limpuai::data::image_processor::{
    ColorType, ImageHandle as ImageHandleHandle, Guest, GuestImageHandle, ImageMeta, OutputFormat,
};

// ---------------------------------------------------------------------------
// Internal state held by the image-handle resource.
// ---------------------------------------------------------------------------
struct ImageInner {
    image: image::DynamicImage,
}

// ---------------------------------------------------------------------------
// WIT resource wrapper — uses RefCell because GuestImageHandle methods take &self.
// ---------------------------------------------------------------------------
struct ImageHandleImpl {
    inner: RefCell<ImageInner>,
}

// ---------------------------------------------------------------------------
// Component — entry point for the WIT world.
// ---------------------------------------------------------------------------
struct Component;

// ---------------------------------------------------------------------------
// Type conversion helpers
// ---------------------------------------------------------------------------

fn color_type_from_dynamic(img: &image::DynamicImage) -> ColorType {
    match img {
        image::DynamicImage::ImageLuma8(_) => ColorType::L8,
        image::DynamicImage::ImageLumaA8(_) => ColorType::La8,
        image::DynamicImage::ImageRgb8(_) => ColorType::Rgb8,
        image::DynamicImage::ImageRgba8(_) => ColorType::Rgba8,
        image::DynamicImage::ImageLuma16(_) => ColorType::L16,
        image::DynamicImage::ImageLumaA16(_) => ColorType::La16,
        image::DynamicImage::ImageRgb16(_) => ColorType::Rgb16,
        image::DynamicImage::ImageRgba16(_) => ColorType::Rgba16,
        image::DynamicImage::ImageRgb32F(_) => ColorType::Rgb32f,
        image::DynamicImage::ImageRgba32F(_) => ColorType::Rgba32f,
        _ => ColorType::Rgba8,
    }
}

fn color_type_from_image_color_type(ct: image::ColorType) -> ColorType {
    match ct {
        image::ColorType::L8 => ColorType::L8,
        image::ColorType::La8 => ColorType::La8,
        image::ColorType::Rgb8 => ColorType::Rgb8,
        image::ColorType::Rgba8 => ColorType::Rgba8,
        image::ColorType::L16 => ColorType::L16,
        image::ColorType::La16 => ColorType::La16,
        image::ColorType::Rgb16 => ColorType::Rgb16,
        image::ColorType::Rgba16 => ColorType::Rgba16,
        image::ColorType::Rgb32F => ColorType::Rgb32f,
        image::ColorType::Rgba32F => ColorType::Rgba32f,
        _ => ColorType::Rgba8,
    }
}

fn format_name(img_format: image::ImageFormat) -> String {
    match img_format {
        image::ImageFormat::Png => "png".to_string(),
        image::ImageFormat::Jpeg => "jpeg".to_string(),
        image::ImageFormat::WebP => "webp".to_string(),
        image::ImageFormat::Pnm => "pnm".to_string(),
        image::ImageFormat::Tiff => "tiff".to_string(),
        image::ImageFormat::Bmp => "bmp".to_string(),
        image::ImageFormat::Ico => "ico".to_string(),
        image::ImageFormat::Gif => "gif".to_string(),
        image::ImageFormat::Tga => "tga".to_string(),
        image::ImageFormat::Dds => "dds".to_string(),
        image::ImageFormat::OpenExr => "exr".to_string(),
        image::ImageFormat::Hdr => "hdr".to_string(),
        image::ImageFormat::Farbfeld => "ff".to_string(),
        image::ImageFormat::Qoi => "qoi".to_string(),
        _ => format!("{img_format:?}").to_lowercase(),
    }
}

fn output_format_to_image_format(fmt: OutputFormat) -> image::ImageFormat {
    match fmt {
        OutputFormat::Png => image::ImageFormat::Png,
        OutputFormat::Jpeg => image::ImageFormat::Jpeg,
        OutputFormat::Bmp => image::ImageFormat::Bmp,
        OutputFormat::Gif => image::ImageFormat::Gif,
        OutputFormat::Tiff => image::ImageFormat::Tiff,
        OutputFormat::Ico => image::ImageFormat::Ico,
        OutputFormat::Webp => image::ImageFormat::WebP,
        OutputFormat::Pnm => image::ImageFormat::Pnm,
    }
}

fn new_handle(img: image::DynamicImage) -> ImageHandleHandle {
    ImageHandleHandle::new(ImageHandleImpl {
        inner: RefCell::new(ImageInner { image: img }),
    })
}

fn decode_image_inner(data: Vec<u8>) -> Result<ImageInner, String> {
    if data.is_empty() {
        return Err("empty input".to_string());
    }

    let image = image::load_from_memory(&data)
        .map_err(|e| format!("decode error: {e}"))?;

    Ok(ImageInner { image })
}

fn encode_dynamic_image(
    img: &image::DynamicImage,
    format: OutputFormat,
    quality: Option<u8>,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();

    match format {
        OutputFormat::Jpeg => {
            use image::codecs::jpeg::JpegEncoder;
            let q = quality.unwrap_or(75);
            let encoder = JpegEncoder::new_with_quality(&mut buf, q);
            img.write_with_encoder(encoder)
                .map_err(|e| format!("jpeg encode error: {e}"))?;
        }
        _ => {
            let img_format = output_format_to_image_format(format);
            img.write_to(&mut Cursor::new(&mut buf), img_format)
                .map_err(|e| format!("encode error: {e}"))?;
        }
    }

    Ok(buf)
}

// ---------------------------------------------------------------------------
// Guest trait — entry points.
// ---------------------------------------------------------------------------
impl Guest for Component {
    type ImageHandle = ImageHandleImpl;

    fn decode_image(data: Vec<u8>) -> Result<ImageHandleHandle, String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let inner = decode_image_inner(data)?;
            Ok(ImageHandleHandle::new(ImageHandleImpl {
                inner: RefCell::new(inner),
            }))
        }))
        .map_err(|_| "image decode panic: invalid data".to_string())?
    }

    fn encode_image(
        img: ImageHandleHandle,
        format: OutputFormat,
        quality: Option<u8>,
    ) -> Result<Vec<u8>, String> {
        let handle = img.get::<ImageHandleImpl>();
        let image = handle.inner.borrow().image.clone();
        encode_dynamic_image(&image, format, quality)
    }

    fn image_info(data: Vec<u8>) -> Result<ImageMeta, String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if data.is_empty() {
                return Err("empty input".to_string());
            }

            use image::ImageDecoder;

            let reader = image::ImageReader::new(Cursor::new(&data))
                .with_guessed_format()
                .map_err(|e| format!("format detection error: {e}"))?;

            let format_str = reader
                .format()
                .map(format_name)
                .unwrap_or_else(|| "unknown".to_string());

            let decoder = reader
                .into_decoder()
                .map_err(|e| format!("decoder error: {e}"))?;

            let (width, height) = decoder.dimensions();
            let color_type = decoder.color_type();

            Ok(ImageMeta {
                width,
                height,
                color_type: color_type_from_image_color_type(color_type),
                format: format_str,
            })
        }))
        .map_err(|_| "image info panic: invalid data".to_string())?
    }
}

// ---------------------------------------------------------------------------
// GuestImageHandle — resource method implementations.
// ---------------------------------------------------------------------------
impl GuestImageHandle for ImageHandleImpl {
    fn width(&self) -> u32 {
        self.inner.borrow().image.width()
    }

    fn height(&self) -> u32 {
        self.inner.borrow().image.height()
    }

    fn color_type(&self) -> ColorType {
        let inner = self.inner.borrow();
        color_type_from_dynamic(&inner.image)
    }

    fn pixel_data(&self) -> Vec<u8> {
        let inner = self.inner.borrow();
        inner.image.as_bytes().to_vec()
    }

    fn resize(&self, width: u32, height: u32) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let resized = inner
            .image
            .resize_exact(width, height, image::imageops::FilterType::Triangle);
        Ok(new_handle(resized))
    }

    fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let cropped = inner.image.crop_imm(x, y, width, height);
        Ok(new_handle(cropped))
    }

    fn blur(&self, sigma: f32) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let blurred = inner.image.blur(sigma);
        Ok(new_handle(blurred))
    }

    fn brighten(&self, value: f32) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.brighten(value.round() as i32);
        Ok(new_handle(result))
    }

    fn contrast(&self, value: f32) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.adjust_contrast(value);
        Ok(new_handle(result))
    }

    fn grayscale(&self) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.grayscale();
        Ok(new_handle(result))
    }

    fn invert(&self) -> Result<ImageHandleHandle, String> {
        // Invert in place, then return a new handle wrapping the modified image.
        {
            let mut inner = self.inner.borrow_mut();
            inner.image.invert();
        }
        let inner = self.inner.borrow();
        let cloned = inner.image.clone();
        Ok(new_handle(cloned))
    }

    fn fliph(&self) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.fliph();
        Ok(new_handle(result))
    }

    fn flipv(&self) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.flipv();
        Ok(new_handle(result))
    }

    fn rotate90(&self) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.rotate90();
        Ok(new_handle(result))
    }

    fn rotate180(&self) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.rotate180();
        Ok(new_handle(result))
    }

    fn rotate270(&self) -> Result<ImageHandleHandle, String> {
        let inner = self.inner.borrow();
        let result = inner.image.rotate270();
        Ok(new_handle(result))
    }
}

export!(Component);

// ---------------------------------------------------------------------------
// Tests — exercise internal logic directly, bypassing WIT handles.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use exports::limpuai::data::image_processor::{ColorType, OutputFormat};
    use image::GenericImage;

    fn parse_image(data: &[u8]) -> Result<image::DynamicImage, String> {
        decode_image_inner(data.to_vec()).map(|inner| inner.image)
    }

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 0, 0, 255]));
        let mut buf = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Png,
        )
        .expect("PNG encode");
        buf
    }

    fn make_test_jpeg(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([255, 0, 0]));
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
        img.write_with_encoder(encoder).expect("JPEG encode");
        buf
    }

    fn image_info_for_test(data: &[u8]) -> Result<(u32, u32, String, image::ColorType), String> {
        if data.is_empty() {
            return Err("empty input".to_string());
        }
        use image::ImageDecoder;
        let reader = image::ImageReader::new(std::io::Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| format!("format detection error: {e}"))?;
        let fmt = reader
            .format()
            .map(format_name)
            .unwrap_or_else(|| "unknown".to_string());
        let decoder = reader
            .into_decoder()
            .map_err(|e| format!("decoder error: {e}"))?;
        let (w, h) = decoder.dimensions();
        let ct = decoder.color_type();
        Ok((w, h, fmt, ct))
    }

    // ── Decode tests ────────────────────────────────────────────────────

    #[test]
    fn decode_png() {
        let img = parse_image(&make_test_png(64, 48)).expect("parse PNG");
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 48);
    }

    #[test]
    fn decode_empty() {
        let result = decode_image_inner(Vec::new());
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("empty"));
    }

    #[test]
    fn decode_invalid() {
        let result = decode_image_inner(vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_jpeg() {
        let img = parse_image(&make_test_jpeg(32, 24)).expect("parse JPEG");
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 24);
    }

    // ── Metadata tests ──────────────────────────────────────────────────

    #[test]
    fn image_info_metadata() {
        let (w, h, fmt, ct) = image_info_for_test(&make_test_png(100, 50)).expect("image info");
        assert_eq!(w, 100);
        assert_eq!(h, 50);
        assert_eq!(fmt, "png");
        assert_eq!(ct, image::ColorType::Rgba8);
    }

    #[test]
    fn width_height() {
        let img = parse_image(&make_test_png(200, 100)).expect("parse");
        assert_eq!(img.width(), 200);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn color_type() {
        let img = parse_image(&make_test_png(10, 10)).expect("parse");
        assert_eq!(color_type_from_dynamic(&img), ColorType::Rgba8);
    }

    #[test]
    fn pixel_data() {
        let img = parse_image(&make_test_png(10, 10)).expect("parse");
        assert_eq!(img.as_bytes().len(), 10 * 10 * 4);
    }

    // ── Transform tests ─────────────────────────────────────────────────

    #[test]
    fn resize() {
        let img = parse_image(&make_test_png(100, 80)).expect("parse");
        let resized = img.resize_exact(50, 50, image::imageops::FilterType::Triangle);
        assert_eq!(resized.width(), 50);
        assert_eq!(resized.height(), 50);
    }

    #[test]
    fn crop() {
        let img = parse_image(&make_test_png(100, 100)).expect("parse");
        let cropped = img.crop_imm(10, 10, 25, 25);
        assert_eq!(cropped.width(), 25);
        assert_eq!(cropped.height(), 25);
    }

    #[test]
    fn blur() {
        let img = parse_image(&make_test_png(50, 50)).expect("parse");
        let blurred = img.blur(2.0);
        assert_eq!(blurred.width(), 50);
        assert_eq!(blurred.height(), 50);
    }

    #[test]
    fn brighten() {
        let img = parse_image(&make_test_png(50, 50)).expect("parse");
        let result = img.brighten(10);
        assert_eq!(result.width(), 50);
    }

    #[test]
    fn contrast() {
        let img = parse_image(&make_test_png(50, 50)).expect("parse");
        let result = img.adjust_contrast(50.0);
        assert_eq!(result.width(), 50);
    }

    #[test]
    fn grayscale() {
        let img = parse_image(&make_test_png(30, 30)).expect("parse");
        let gray = img.grayscale();
        assert_eq!(color_type_from_dynamic(&gray), ColorType::La8);
    }

    #[test]
    fn invert() {
        let img = parse_image(&make_test_png(10, 10)).expect("parse");
        let original = img.as_bytes().to_vec();
        let mut inverted = img;
        inverted.invert();
        assert_ne!(original.as_slice(), inverted.as_bytes());
    }

    #[test]
    fn fliph() {
        let img = parse_image(&make_test_png(50, 50)).expect("parse");
        let flipped = img.fliph();
        assert_eq!(flipped.width(), 50);
        assert_eq!(flipped.height(), 50);
    }

    #[test]
    fn flipv() {
        let img = parse_image(&make_test_png(50, 50)).expect("parse");
        let flipped = img.flipv();
        assert_eq!(flipped.width(), 50);
        assert_eq!(flipped.height(), 50);
    }

    #[test]
    fn rotate90() {
        let img = parse_image(&make_test_png(80, 40)).expect("parse");
        let rotated = img.rotate90();
        assert_eq!(rotated.width(), 40);
        assert_eq!(rotated.height(), 80);
    }

    // ── Encode tests ────────────────────────────────────────────────────

    #[test]
    fn encode_png() {
        let img = parse_image(&make_test_png(20, 20)).expect("parse");
        let encoded = encode_dynamic_image(&img, OutputFormat::Png, None).expect("encode PNG");
        assert!(!encoded.is_empty());
    }

    #[test]
    fn encode_jpeg() {
        let img = parse_image(&make_test_png(20, 20)).expect("parse");
        let encoded =
            encode_dynamic_image(&img, OutputFormat::Jpeg, Some(90)).expect("encode JPEG");
        assert!(!encoded.is_empty());
    }

    #[test]
    fn encode_jpeg_quality() {
        let mut gradient = parse_image(&make_test_png(100, 100)).expect("parse");
        for y in 0..gradient.height() {
            for x in 0..gradient.width() {
                let r = ((x * 255) / gradient.width()) as u8;
                let g = ((y * 255) / gradient.height()) as u8;
                let b = (((x + y) * 128) / (gradient.width() + gradient.height())) as u8;
                gradient.put_pixel(x, y, image::Rgba([r, g, b, 255]));
            }
        }

        let low =
            encode_dynamic_image(&gradient, OutputFormat::Jpeg, Some(10)).expect("q10");
        let high =
            encode_dynamic_image(&gradient, OutputFormat::Jpeg, Some(100)).expect("q100");
        assert!(
            high.len() > low.len(),
            "quality 100 ({}) should be larger than quality 10 ({})",
            high.len(),
            low.len(),
        );
    }
}
