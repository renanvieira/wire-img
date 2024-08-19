mod file_watcher;

use std::{env, io::ErrorKind, path::PathBuf, str::FromStr};

use axum::{
    extract::Path,
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use dotenv::dotenv;
use file_watcher::load_images;
use tokio::{io::AsyncReadExt, net::TcpListener};
use tracing::{info, Level};
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
        .route("/:image", get(serve_image));

    let listener = TcpListener::bind("0.0.0.0:5000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    info!("Exiting...");
    Ok(())
}

pub async fn serve_image(Path(image): Path<String>) -> axum::response::Result<impl IntoResponse> {
    let mut full_path = PathBuf::from_str("/tmp/watch-out")?;
    full_path.push(image);
    full_path.set_extension("avif");

    let handle = tokio::fs::OpenOptions::new()
        .read(true)
        .open(full_path.clone())
        .await;

    match handle {
        Ok(mut f) => {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, "image/avif".parse().unwrap());
            let mut bytes: Vec<u8> = Vec::new();

            let read_result = f.read_to_end(&mut bytes).await;
            match read_result {
                Ok(s) => info!("Read {} bytes for {:?}", s, &full_path),
                Err(e) => {
                    tracing::error!("Failed reading Image file: {:?} : {}", &full_path, e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR.into());
                }
            }

            Ok((headers, bytes).into_response())
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("Failed serve Image: {:?}: {}", &full_path, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR.into())
        }
    }
}
