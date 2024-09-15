mod file_watcher;

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use configuration::{
    config::{Settings, TemplateSettings, TemplateType},
    ImageEncoding,
};
use core::panic;
use dotenv::dotenv;
use file_watcher::ImageWatcher;
use image_processing::transcoder::{Encoder, Operations, PixelSize};
use image_processing::{transcoder::Transcoder, ImageFormat};
use std::{
    fs::OpenOptions,
    io::{ErrorKind, Read},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, LazyLock},
};
use tokio::{io::AsyncReadExt, net::TcpListener};
use tracing::{error, info, warn, Level};
use tracing_subscriber::fmt::format::FmtSpan;

static CONFIGURATION: LazyLock<Settings> = LazyLock::new(|| {
    // TODO: use a env var to find configuration file
    let config_file = OpenOptions::new().read(true).open("settings.toml");

    match config_file {
        Ok(mut f) => {
            let mut toml_str = String::new();
            let read_result = f.read_to_string(&mut toml_str);

            match read_result {
                Ok(_) => toml::from_str(&toml_str).unwrap(),
                Err(e) => {
                    panic!("Failed to read the file: {}", e)
                }
            }
        }
        Err(e) => {
            error!("Error while opening the configuration file: {}", e);
            warn!("Configuration file not found. Loading defaults...");
            // TODO: load default configuration
            Settings::default()
        }
    }
});

#[derive(Debug)]
pub struct APIState<'a> {
    configuration: &'a Settings,
    transcoder: Transcoder,
}

impl<'a> APIState<'a> {
    pub fn new(configuration: &'a Settings, transcoder: Transcoder) -> Self {
        Self {
            configuration,
            transcoder,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let config = &*CONFIGURATION;
    let transcoder = Transcoder;

    let state = APIState::new(config, transcoder);
    let state_arc = Arc::new(state);

    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::FULL)
        .with_max_level(Level::DEBUG)
        .pretty()
        .init();

    info!("Watching new images at {:?}", &config.image.input_path);
    info!("Storing encoded images at {:?}", &config.image.output_path);

    let watcher = ImageWatcher::new(config.image.input_path.clone(), Arc::clone(&state_arc))?;
    tokio::spawn(watcher.watch());

    let app = Router::new()
        .route("/", get(|| async { "home" }))
        .route("/:image", get(default_serve_image))
        .route("/:image/:extension", get(serve_image))
        .route("/:width/:height/:image/:extension", get(serve_resized))
        .with_state(Arc::clone(&state_arc));

    let address = format!("{}:{}", config.server.host, config.server.port);
    info!("Starting server at {}", address);

    let listener = TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    info!("Exiting...");
    Ok(())
}

pub async fn serve_resized(
    Path((width, height, image, ext)): Path<(u32, u32, String, String)>,
    State(state): State<Arc<APIState<'_>>>,
) -> axum::response::Result<impl IntoResponse> {
    let resize_params = PixelSize::new(width, height);

    let extension = match ext.as_str() {
        "png" => ImageEncoding::PNG,
        "jpg" | "jpeg" => ImageEncoding::JPEG,
        "avif" => ImageEncoding::AVIF,
        _ => return Err(StatusCode::BAD_REQUEST.into()),
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, extension.content_type().parse().unwrap());

    let encoded_image_bytes =
        process(image, extension, Some(resize_params), &state.configuration).await;

    match encoded_image_bytes {
        Ok(b) => Ok((headers, b).into_response()),
        Err(e) => {
            error!("Failed to encode image to {:?}: {}", ext, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR.into())
        }
    }
}
#[tracing::instrument]
pub async fn default_serve_image(
    Path(image): Path<String>,
    state: State<Arc<APIState<'_>>>,
) -> axum::response::Result<impl IntoResponse> {
    // TODO: Choose default based on Accept header. Order: avif, jpg, png
    serve_image(Path((image, "avif".to_string())), state).await
}

#[tracing::instrument]
pub async fn serve_image(
    Path((image, ext)): Path<(String, String)>,
    State(state): State<Arc<APIState<'_>>>,
) -> axum::response::Result<impl IntoResponse> {
    let extension = match ext.as_str() {
        "png" => ImageEncoding::PNG,
        "jpg" | "jpeg" => ImageEncoding::JPEG,
        "avif" => ImageEncoding::AVIF,
        _ => return Err(StatusCode::BAD_REQUEST.into()),
    };

    let allowed_formats = &state.configuration.image.formats;

    if !allowed_formats.contains(&extension) {
        // TODO: serve an image with written error
        return Err(StatusCode::BAD_REQUEST.into());
    }

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, extension.content_type().parse().unwrap());

    let encoded_image_bytes = process(image, extension, None, &state.configuration).await;

    match encoded_image_bytes {
        Ok(b) => Ok((headers, b).into_response()),
        Err(e) => {
            error!("Failed to encode image to {:?}: {}", ext, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR.into())
        }
    }
}

async fn process(
    name: String,
    target_format: ImageEncoding,
    new_size: Option<PixelSize>,
    config: &Settings,
) -> anyhow::Result<Vec<u8>> {
    let mut full_path = config.image.input_path.clone();
    full_path.push(&name);
    full_path.set_extension("avif");

    let mut op = Vec::new();

    if let Some(resize_param) = new_size {
        op.push(Operations::Resize(resize_param));
    }

    let template_result = check_templates(&name, &config.templates);

    if let Ok(template) = template_result {
        let image_name = remove_template_pattern(&name, template);
        full_path.push(image_name);
    } else {
        full_path.push(&name);
    }

    full_path.set_extension("avif");

    let handle = tokio::fs::OpenOptions::new()
        .read(true)
        .open(full_path.clone())
        .await;

    match handle {
        Ok(mut f) => {
            let mut bytes: Vec<u8> = Vec::new();

            let read_result = f.read_to_end(&mut bytes).await;
            match read_result {
                Ok(s) => {
                    info!("Read {} bytes for {:?}", s, &full_path);

                    let encoded_image_bytes = if let Ok(template) = template_result {
                        let format = match template.format {
                            ImageEncoding::AVIF => ImageFormat::Avif,
                            ImageEncoding::JPEG => ImageFormat::Jpeg,
                            ImageEncoding::PNG => ImageFormat::Png,
                        };

                        Transcoder.transcode(
                            &bytes,
                            template.format.extension().to_string(),
                            format,
                            Some(vec![Operations::Resize(PixelSize::new(
                                template.size[0],
                                template.size[1],
                            ))]),
                        )
                    } else {
                        match target_format {
                            ImageEncoding::AVIF => Ok(bytes),
                            ImageEncoding::JPEG => Transcoder.transcode(
                                &bytes,
                                "avif".to_owned(),
                                image_processing::ImageFormat::Jpeg,
                                None,
                            ),
                            ImageEncoding::PNG => Transcoder.transcode(
                                &bytes,
                                "avif".to_owned(),
                                image_processing::ImageFormat::Png,
                                None,
                            ),
                        }
                    };

                    Ok(encoded_image_bytes?)
                }
                Err(e) => {
                    tracing::error!("Failed reading Image file: {:?} : {}", &full_path, e);
                    Err(anyhow!("Failed reading image file"))
                }
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Err(anyhow!("Not found")),
        Err(e) => {
            tracing::error!("Failed opening Image: {:?}: {}", &full_path, e);
            Err(anyhow!("Failed opening image file"))
        }
    }
}

fn remove_template_pattern(image: &str, template: &TemplateSettings) -> String {
    match template.location {
        TemplateType::Prefix => {
            let pattern = format!("{}_", &template.name);
            return image.strip_prefix(&pattern).unwrap().to_string();
        }
        TemplateType::Suffix => {
            let pattern = format!("_{}", &template.name);
            return image.strip_suffix(&pattern).unwrap().to_string();
        }
    }
}

#[tracing::instrument]
fn check_templates<'a>(
    image_name: &String,
    config_templates: &'a Vec<TemplateSettings>,
) -> anyhow::Result<&'a TemplateSettings> {
    let split_name: Vec<_> = image_name.split("_").collect();

    let maybe_prefix = split_name.first();
    let maybe_suffix = split_name.last();

    if maybe_prefix.is_none() && maybe_suffix.is_none() {
        return Err(anyhow::anyhow!("File has no prefix and suffix patterns."));
    }

    for template in config_templates.iter() {
        let is_suffix_template = maybe_suffix.is_some()
            && template.location == TemplateType::Suffix
            && template.name == *maybe_suffix.unwrap();

        let is_prefix_template = maybe_prefix.is_some()
            && template.location == TemplateType::Prefix
            && template.name == *maybe_prefix.unwrap();

        if is_prefix_template || is_suffix_template {
            return Ok(template);
        }
    }

    Err(anyhow!("No templates matched"))
}
