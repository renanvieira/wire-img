#![allow(dead_code)]

use std::net::Ipv4Addr;

use image_processing::transcoder::ImageEncoding;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    server: ServerSettings,
    image: ImageSettings,
    templates: Vec<TemplateSettings>,
}

#[derive(Debug, Deserialize)]
pub struct ServerSettings {
    port: u16,
    host: Ipv4Addr,
}

#[derive(Debug, Deserialize)]
pub struct ImageSettings {
    formats: Vec<ImageEncoding>,
    storage_format: ImageEncoding,
}

#[derive(Debug, Deserialize)]
pub struct TemplateSettings {
    location: TemplateType,
    name: String,
    size: [u16; 2],
    format: ImageEncoding,
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
    use std::net::Ipv4Addr;

    use image_processing::transcoder::ImageEncoding;

    use crate::config::{Settings, TemplateType};

    #[test]
    fn test_valid_toml() -> anyhow::Result<()> {
        let valid_toml: &str = r#"
            # Settings for the server
            [server]
            port = 8080
            host = "192.168.1.1"

            # Settings for images
            [image]
            formats = ["PNG", "JPEG"]
            storage_format = "PNG"

            # Settings for templates
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
            # Settings for the server
            [server]
            port = 8080
            host = "192.168.1.1"

            # Settings for images
            [image]
            formats = ["png", "jpeg"]
            storage_format = "png"

            # Settings for templates
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
