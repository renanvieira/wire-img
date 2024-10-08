#![allow(dead_code)]

use std::{net::Ipv4Addr, path::PathBuf};

use serde::Deserialize;

use crate::ImageEncoding;

#[derive(Debug, Deserialize, Default)]
pub struct Settings {
    pub server: ServerSettings,
    pub image: ImageSettings,
    pub templates: Vec<TemplateSettings>,
}

#[derive(Debug, Deserialize)]
pub struct ServerSettings {
    pub port: u16,
    pub host: Ipv4Addr,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            port: 3000,
            host: Ipv4Addr::LOCALHOST,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ImageSettings {
    pub formats: Vec<ImageEncoding>,
    pub storage_format: ImageEncoding,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            formats: vec![ImageEncoding::AVIF, ImageEncoding::JPEG, ImageEncoding::PNG],
            storage_format: ImageEncoding::AVIF,
            input_path: "/var/lib/wire-img/in".into(),
            output_path: "/var/lib/wire-img/out".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TemplateSettings {
    pub location: TemplateType,
    pub name: String,
    pub size: [u32; 2],
    pub format: ImageEncoding,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum TemplateType {
    #[serde(alias = "prefix")]
    Prefix,
    #[serde(alias = "suffix")]
    Suffix,
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, path::PathBuf, str::FromStr};

    use crate::{
        config::{Settings, TemplateType},
        ImageEncoding,
    };

    #[test]
    fn test_valid_toml() -> anyhow::Result<()> {
        let valid_toml: &str = r#"
            [server]
            port = 8080
            host = "192.168.1.1"

            [image]
            formats = ["PNG", "JPEG"]
            storage_format = "PNG"
            input_path = "/tmp/watch-in"
            output_path = "/tmp/watch-out"

            [[templates]]
            location = "Prefix"
            name = "large"
            size = [1920, 1080]
            format = "PNG"

            [[templates]]
            location = "Suffix"
            name = "full"
            size = [1280, 720]
            format = "JPEG"
        "#;
        let result = toml::from_str::<Settings>(valid_toml)?;

        // asserts server settings
        let server = result.server;
        assert_eq!(server.port, 8080);
        assert_eq!(server.host, Ipv4Addr::new(192, 168, 1, 1));

        // image settings
        let image_settings = result.image;
        assert_eq!(
            image_settings.formats,
            vec![ImageEncoding::PNG, ImageEncoding::JPEG]
        );
        assert_eq!(image_settings.storage_format, ImageEncoding::PNG);
        assert_eq!(
            image_settings.input_path,
            PathBuf::from_str("/tmp/watch-in")?
        );
        assert_eq!(
            image_settings.output_path,
            PathBuf::from_str("/tmp/watch-out")?
        );

        // templates settings

        let template_settings = result.templates;
        assert_eq!(template_settings.len(), 2);

        let first_template = template_settings.first().unwrap();
        assert_eq!(first_template.name, "large");
        assert_eq!(first_template.location, TemplateType::Prefix);
        assert_eq!(first_template.size, [1920, 1080]);
        assert_eq!(first_template.format, ImageEncoding::PNG);

        let second_template = template_settings.last().unwrap();
        assert_eq!(second_template.name, "full");
        assert_eq!(second_template.location, TemplateType::Suffix);
        assert_eq!(second_template.size, [1280, 720]);
        assert_eq!(second_template.format, ImageEncoding::JPEG);

        Ok(())
    }

    #[test]
    fn test_valid_toml_lowercase_enum_values() -> anyhow::Result<()> {
        let valid_toml: &str = r#"
            [server]
            port = 8080
            host = "192.168.1.1"

            [image]
            formats = ["png", "jpeg"]
            storage_format = "png"
            input_path = "/tmp/watch-in"
            output_path = "/tmp/watch-out"

            [[templates]]
            location = "prefix"
            name = "large"
            size = [1920, 1080]
            format = "png"

            [[templates]]
            location = "suffix"
            name = "full"
            size = [1280, 720]
            format = "jpeg"
        "#;
        let result = toml::from_str::<Settings>(valid_toml)?;

        // asserts server settings
        let server = result.server;
        assert_eq!(server.port, 8080);
        assert_eq!(server.host, Ipv4Addr::new(192, 168, 1, 1));

        // image settings
        let image_settings = result.image;
        assert_eq!(
            image_settings.formats,
            vec![ImageEncoding::PNG, ImageEncoding::JPEG]
        );
        assert_eq!(image_settings.storage_format, ImageEncoding::PNG);
        assert_eq!(
            image_settings.input_path,
            PathBuf::from_str("/tmp/watch-in")?
        );
        assert_eq!(
            image_settings.output_path,
            PathBuf::from_str("/tmp/watch-out")?
        );

        // templates settings

        let template_settings = result.templates;
        assert_eq!(template_settings.len(), 2);

        let first_template = template_settings.first().unwrap();
        assert_eq!(first_template.name, "large");
        assert_eq!(first_template.location, TemplateType::Prefix);
        assert_eq!(first_template.size, [1920, 1080]);
        assert_eq!(first_template.format, ImageEncoding::PNG);

        let second_template = template_settings.last().unwrap();
        assert_eq!(second_template.name, "full");
        assert_eq!(second_template.location, TemplateType::Suffix);
        assert_eq!(second_template.size, [1280, 720]);
        assert_eq!(second_template.format, ImageEncoding::JPEG);

        Ok(())
    }
}
