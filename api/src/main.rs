mod file_watcher;

use anyhow::anyhow;
use axum::{
    extract::Path,
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use dotenv::dotenv;
use file_watcher::load_images;
use image_processing::transcoder::{Encoder, ImageEncoding, Operations, PixelSize, Transcoder};
use std::{env, io::ErrorKind, path::PathBuf, str::FromStr};
use tokio::{io::AsyncReadExt, net::TcpListener};
use tracing::{error, info, Level};
use tracing_subscriber::fmt::format::FmtSpan;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::FULL)
        .with_max_level(Level::DEBUG)
        .pretty()
        .init();

    let input_path = env::var("INPUT_PATH")?;
    let path = PathBuf::from_str(&input_path)?;

    tokio::spawn(load_images(path));

    let app = Router::new()
        .route("/", get(|| async { "home" }))
        .route("/:image", get(default_serve_image))
        .route("/:image/:extension", get(serve_image))
        .route("/:width/:height/:image/:extension", get(serve_resized));

    let listener = TcpListener::bind("0.0.0.0:5000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    info!("Exiting...");
    Ok(())
}

pub async fn serve_resized(
    Path((width, height, image, ext)): Path<(u32, u32, String, String)>,
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

    let encoded_image_bytes = process(image, extension, Some(resize_params)).await;

    match encoded_image_bytes {
        Ok(b) => Ok((headers, b).into_response()),
        Err(e) => {
            error!("Failed to encode image to {:?}: {}", ext, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR.into())
        }
    }
}
pub async fn default_serve_image(
    Path(image): Path<String>,
) -> axum::response::Result<impl IntoResponse> {
    // TODO: Choose default based on Accept header. Order: avif, jpg, png
    serve_image(Path((image, "avif".to_string()))).await
}

pub async fn serve_image(
    Path((image, ext)): Path<(String, String)>,
) -> axum::response::Result<impl IntoResponse> {
    let extension = match ext.as_str() {
        "png" => ImageEncoding::PNG,
        "jpg" | "jpeg" => ImageEncoding::JPEG,
        "avif" => ImageEncoding::AVIF,
        _ => return Err(StatusCode::BAD_REQUEST.into()),
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, extension.content_type().parse().unwrap());

    let encoded_image_bytes = process(image, extension, None).await;

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
) -> anyhow::Result<Vec<u8>> {
    let mut full_path = PathBuf::from_str("/tmp/watch-out")?;
    full_path.push(name);
    full_path.set_extension("avif");

    let mut op = Vec::new();

    if let Some(resize_param) = new_size {
        op.push(Operations::Resize(resize_param));
    }

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

                    let encoded_image_bytes = match target_format {
                        ImageEncoding::AVIF => Ok(bytes), // TODO: apply resize to avif
                        ImageEncoding::JPEG => Transcoder.transcode(
                            &bytes,
                            "avif".to_owned(),
                            image_processing::ImageFormat::Jpeg,
                            Some(op),
                        ),
                        ImageEncoding::PNG => Transcoder.transcode(
                            &bytes,
                            "avif".to_owned(),
                            image_processing::ImageFormat::Png,
                            Some(op),
                        ),
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
