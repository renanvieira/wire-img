use std::io::Cursor;
use tracing::warn;

use anyhow::anyhow;
use image::{guess_format, ImageFormat};
use tracing::error;

#[derive(Debug)]
pub struct Position(u32, u32);

impl Position {
    pub fn x(&self) -> &u32 {
        &self.0
    }

    pub fn y(&self) -> &u32 {
        &self.1
    }
}

#[derive(Debug)]
pub struct PixelSize(u32, u32);

impl PixelSize {
    pub fn new(width: u32, height: u32) -> Self {
        PixelSize(width, height)
    }

    pub fn width(&self) -> &u32 {
        &self.0
    }

    pub fn height(&self) -> &u32 {
        &self.1
    }
}

#[derive(Debug)]
pub enum Operations {
    Resize(PixelSize),
    Crop(Position, PixelSize),
}

pub trait Encoder {
    fn transcode(
        &self,
        image: &[u8],
        extension: String,
        target: ImageFormat,
        ops: Option<Vec<Operations>>,
    ) -> anyhow::Result<Vec<u8>>;
}

#[derive(Debug)]
pub struct Transcoder;

impl Encoder for Transcoder {
    #[tracing::instrument(skip(image))]
    fn transcode(
        &self,
        image: &[u8],
        extension: String,
        target: ImageFormat,
        ops: Option<Vec<Operations>>,
    ) -> anyhow::Result<Vec<u8>> {
        let format_result = guess_format(image);

        let format = match format_result {
            Ok(format) => format,
            Err(err) => {
                warn!("error while trying to validate image format: {:?}", err);

                match extension.as_str(){
                    "png"=> ImageFormat::Png,
                    "jpg"|"jpeg"=>ImageFormat::Jpeg,
                    "avif" => ImageFormat::Avif,
                    _=> return Err(anyhow!("error while trying to validate image format by  unsupported extension: {:?}", extension))
                }
            }
        };

        let bytes: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(bytes);
        let is_jpg = target == ImageFormat::Jpeg;

        let mut image = image::load_from_memory_with_format(image, format)?;

        if let Some(operations) = ops {
            for op in operations {
                match op {
                    Operations::Resize(s) => {
                        image = image.resize_exact(
                            *s.width(),
                            *s.height(),
                            image::imageops::FilterType::CatmullRom,
                        );
                    }
                    Operations::Crop(p, s) => {
                        image = image.crop_imm(*p.x(), *p.y(), *s.width(), *s.height());
                    }
                }
            }
        }

        if is_jpg {
            let rgb8 = image.to_rgb8();
            rgb8.write_to(&mut cursor, target)?;
        } else {
            image.write_to(&mut cursor, target)?;
        }

        Ok(cursor.get_ref().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path};

    use image::DynamicImage;

    use super::*;

    fn get_png_image() -> Vec<u8> {
        let s = env::var("CARGO_MANIFEST_DIR").unwrap();
        let p = Path::new(&s).join("../resources/10x10.png");

        fs::read(p).unwrap()
    }

    fn get_jpeg_image() -> Vec<u8> {
        let s = env::var("CARGO_MANIFEST_DIR").unwrap();
        let p = Path::new(&s).join("../resources/100x100.jpg");

        fs::read(p).unwrap()
    }

    fn get_avif_image() -> Vec<u8> {
        let s = env::var("CARGO_MANIFEST_DIR").unwrap();
        let p = Path::new(&s).join("../resources/fox.avif");

        fs::read(p).unwrap()
    }

    fn transcode(
        img: Vec<u8>,
        output_format: ImageFormat,
        ops: Option<Vec<Operations>>,
    ) -> anyhow::Result<DynamicImage> {
        let t = Transcoder;
        let output_img = t.transcode(&img, "avif".to_owned(), output_format, ops)?;

        assert!(output_img.len() > 0);

        Ok(image::load_from_memory_with_format(
            &output_img,
            output_format,
        )?)
    }

    #[test]
    fn test_transcoder_avif_to_png() -> anyhow::Result<()> {
        let avif_img = get_avif_image();

        assert!(guess_format(&avif_img)? == ImageFormat::Avif);

        let output_img = transcode(avif_img, ImageFormat::Png, None)?;

        assert_eq!(output_img.width(), 1204);
        assert_eq!(output_img.height(), 800);

        Ok(())
    }

    #[test]
    fn test_transcoder_avif_to_png_with_resize() -> anyhow::Result<()> {
        let avif_img = get_avif_image();

        assert!(guess_format(&avif_img)? == ImageFormat::Avif);

        let ops = vec![Operations::Resize(PixelSize(602, 400))];
        let output_img = transcode(avif_img, ImageFormat::Png, Some(ops))?;

        assert_eq!(output_img.width(), 602);
        assert_eq!(output_img.height(), 400);

        Ok(())
    }

    #[test]
    fn test_transcoder_png_to_avif() -> anyhow::Result<()> {
        let png_img = get_png_image();

        assert!(guess_format(&png_img)? == ImageFormat::Png);

        let img = transcode(png_img, ImageFormat::Avif, None)?;

        assert_eq!(img.width(), 20);
        assert_eq!(img.height(), 20);

        Ok(())
    }

    #[test]
    fn test_transcoder_jpeg_to_avif_with_resize() -> anyhow::Result<()> {
        let jpg_img = get_jpeg_image();

        assert!(guess_format(&jpg_img)? == ImageFormat::Jpeg);

        let ops = vec![Operations::Resize(PixelSize(50, 50))];

        let img = transcode(jpg_img, ImageFormat::Avif, Some(ops))?;

        assert_eq!(img.width(), 50);
        assert_eq!(img.height(), 50);

        Ok(())
    }

    #[test]
    fn test_transcoder_jpeg_to_avif() -> anyhow::Result<()> {
        let jpg_img = get_jpeg_image();

        assert!(guess_format(&jpg_img)? == ImageFormat::Jpeg);
        let img = transcode(jpg_img, ImageFormat::Avif, None)?;

        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);

        Ok(())
    }
}
