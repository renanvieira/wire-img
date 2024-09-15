use serde::{Deserialize, Serialize};

pub mod config;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ImageEncoding {
    #[serde(alias = "avif")]
    #[default]
    AVIF,
    #[serde(alias = "jpg", alias = "JPG", alias = "jpeg")]
    JPEG,
    #[serde(alias = "png")]
    PNG,
}

impl ImageEncoding {
    pub fn content_type(&self) -> &str {
        match self {
            ImageEncoding::AVIF => "image/avif",
            ImageEncoding::JPEG => "image/jpeg",
            ImageEncoding::PNG => "image/png",
        }
    }
    pub fn extension(&self) -> &str {
        match self {
            ImageEncoding::AVIF => ".avif",
            ImageEncoding::JPEG => ".jpg",
            ImageEncoding::PNG => ".png",
        }
    }
}
