use std::{cmp::max, error, fmt};

use image::{codecs::png::PngEncoder, EncodableLayout as _, ImageEncoder as _, Pixel, Rgba};
// use image::{DynamicImage, ImageOutputFormat};
use webp_animation::Decoder as AwebPDecoder;

//
//
//
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AwebpFramePosition {
    First,
    Specific(usize),
    Last,
}
impl Default for AwebpFramePosition {
    fn default() -> Self {
        Self::First
    }
}

pub fn awebp_to_single_png(
    awebp_bytes: impl AsRef<[u8]>,
    frame_position: impl Into<Option<AwebpFramePosition>>,
) -> Result<Vec<u8>, AwebpToPngError> {
    let frame_position: AwebpFramePosition = frame_position.into().unwrap_or_default();

    let awebp_decoder =
        AwebPDecoder::new(awebp_bytes.as_ref()).map_err(|_| AwebpToPngError::DecodeAwebpFailed)?;

    let awebp_decoder_iter = awebp_decoder.into_iter();

    let webp_frame = match frame_position {
        AwebpFramePosition::First => {
            awebp_decoder_iter
                .enumerate()
                .find(|(i, _)| *i == 0)
                .ok_or(AwebpToPngError::AwebpSpecificFrameIsNone)?
                .1
        }
        AwebpFramePosition::Specific(n) => {
            let n = max(1, n);

            awebp_decoder_iter
                .enumerate()
                .find(|(i, _)| *i == n - 1)
                .ok_or(AwebpToPngError::AwebpSpecificFrameIsNone)?
                .1
        }
        AwebpFramePosition::Last => awebp_decoder_iter
            .last()
            .ok_or(AwebpToPngError::AwebpSpecificFrameIsNone)?,
    };

    let image = webp_frame
        .into_image()
        .map_err(|_| AwebpToPngError::ToImageFailed)?;

    // https://github.com/image-rs/image/blob/v0.23.14/src/buffer.rs#L926
    // https://github.com/image-rs/image/blob/v0.23.14/src/dynimage.rs#L1280
    // https://github.com/image-rs/image/blob/v0.23.14/src/io/free_functions.rs#L174

    let mut buf = Vec::with_capacity(image.as_bytes().len());

    // DynamicImage::ImageRgba8(image)
    //     .write_to(&mut buf, ImageOutputFormat::Png)
    //     .map_err(|_| AwebpToPngError::EncodePngFailed)?;

    PngEncoder::new(&mut buf)
        .write_image(
            image.as_bytes(),
            image.width(),
            image.height(),
            Rgba::<u8>::COLOR_TYPE,
        )
        .map_err(|_| AwebpToPngError::EncodePngFailed)?;

    Ok(buf)
}

pub fn awebp_to_multi_png(awebp_bytes: impl AsRef<[u8]>) -> Result<Vec<Vec<u8>>, AwebpToPngError> {
    let awebp_decoder =
        AwebPDecoder::new(awebp_bytes.as_ref()).map_err(|_| AwebpToPngError::DecodeAwebpFailed)?;

    let awebp_decoder_iter = awebp_decoder.into_iter();

    awebp_decoder_iter
        .map(|webp_frame| {
            let image = webp_frame
                .into_image()
                .map_err(|_| AwebpToPngError::ToImageFailed)?;

            let mut buf = Vec::with_capacity(image.as_bytes().len());

            PngEncoder::new(&mut buf)
                .write_image(
                    image.as_bytes(),
                    image.width(),
                    image.height(),
                    Rgba::<u8>::COLOR_TYPE,
                )
                .map_err(|_| AwebpToPngError::EncodePngFailed)?;

            Ok(buf)
        })
        .collect::<Result<_, _>>()
}

//
//
//
#[derive(Debug)]
pub enum AwebpToPngError {
    DecodeAwebpFailed,
    AwebpSpecificFrameIsNone,
    ToImageFailed,
    EncodePngFailed,
}
impl fmt::Display for AwebpToPngError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl error::Error for AwebpToPngError {}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        fs::{self, File},
        io::Write as _,
    };

    use tempfile::{tempdir, Builder};

    #[test]
    fn test_awebp_to_single_png_with_animated() {
        let awebp_bytes = include_bytes!("../tests/images/animated-webp-supported.webp");
        let png_bytes = awebp_to_single_png(awebp_bytes, AwebpFramePosition::Last).unwrap();

        let png_decoder = png::Decoder::new(&png_bytes[..]);
        png_decoder.read_info().unwrap();

        let tmp_dir = tempdir().unwrap();

        let mut file = File::create(tmp_dir.path().join("animated-webp-supported.png")).unwrap();
        file.write_all(&png_bytes[..]).unwrap();
        file.sync_all().unwrap();
    }

    #[test]
    fn test_awebp_to_multi_png_with_animated() {
        let awebp_bytes = include_bytes!("../tests/images/animated-webp-supported.webp");
        let png_bytes_list = awebp_to_multi_png(awebp_bytes).unwrap();

        let tmp_dir = Builder::new()
            .prefix("animated-webp-supported")
            .tempdir()
            .unwrap();

        for (i, png_bytes) in png_bytes_list.into_iter().enumerate() {
            let png_decoder = png::Decoder::new(&png_bytes[..]);
            png_decoder.read_info().unwrap();

            let mut file = File::create(tmp_dir.path().join(format!("{}.png", i))).unwrap();
            file.write_all(&png_bytes[..]).unwrap();
            file.sync_all().unwrap();
        }

        println!(
            "{:?}",
            fs::read_dir(tmp_dir.path()).unwrap().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_awebp_to_single_png_with_not_animated() {
        let awebp_bytes = include_bytes!("../tests/images/3_webp_ll.webp");
        let png_bytes = awebp_to_single_png(awebp_bytes, None).unwrap();

        let png_decoder = png::Decoder::new(&png_bytes[..]);
        png_decoder.read_info().unwrap();

        let tmp_dir = tempdir().unwrap();

        let mut file = File::create(tmp_dir.path().join("3_webp_ll.png")).unwrap();
        file.write_all(&png_bytes[..]).unwrap();
        file.sync_all().unwrap();
    }

    #[test]
    fn test_awebp_to_multi_png_with_not_animated() {
        let awebp_bytes = include_bytes!("../tests/images/3_webp_ll.webp");
        let png_bytes_list = awebp_to_multi_png(awebp_bytes).unwrap();

        let tmp_dir = Builder::new().prefix("3_webp_ll").tempdir().unwrap();

        for (i, png_bytes) in png_bytes_list.into_iter().enumerate() {
            let png_decoder = png::Decoder::new(&png_bytes[..]);
            png_decoder.read_info().unwrap();

            let mut file = File::create(tmp_dir.path().join(format!("{}.png", i))).unwrap();
            file.write_all(&png_bytes[..]).unwrap();
            file.sync_all().unwrap();
        }

        println!(
            "{:?}",
            fs::read_dir(tmp_dir.path()).unwrap().collect::<Vec<_>>()
        );
    }
}
