use std::{env, fs, io::Read, path::PathBuf};

use image_processing::transcoder::{Encoder, Transcoder};
use notify::Watcher;
use storage::disk::{DiskStorage, File};
use tokio::sync::oneshot;
use tracing::info;

#[tracing::instrument]
pub async fn load_images(path: PathBuf) -> anyhow::Result<()> {
    let (_tx, rx) = oneshot::channel::<()>();

    info!("Loading images in the input folder");
    for dir in fs::read_dir(&path)? {
        match dir {
            Ok(d) => {
                if d.path().is_file() {
                    match load_file(d.path()) {
                        Ok(_) => (),
                        Err(e) => {
                            tracing::error!(
                                "Failed trying to convert image '{:?}': {}",
                                d.path(),
                                e
                            )
                        }
                    }
                }
            }
            Err(e) => tracing::error!("Error reading input folder: {}", e),
        }
    }

    let mut watcher = notify::recommended_watcher(|res| match res {
        Ok(event) => {
            let res = handle_fs_event(event);
            match res {
                Ok(_) => info!("Done with event."),
                Err(e) => tracing::error!("Error while handling the fs events: {e:?}"),
            }
        }
        Err(error) => {
            tracing::error!("Error: {error:?}");
        }
    })?;

    info!("Watching {:?}", path);
    watcher.watch(&path, notify::RecursiveMode::Recursive)?;

    let _ = rx.await;

    Ok(())
}

#[tracing::instrument]
fn handle_fs_event(event: notify::Event) -> anyhow::Result<()> {
    match event.kind {
        notify::EventKind::Create(create_kind) => match create_kind {
            notify::event::CreateKind::File => {
                if let Some(p) = event.paths.into_iter().next() {
                    load_file(p)?;
                }
                Ok(())
            }
            _ => Ok(()),
        },
        _ => {
            tracing::warn!("not supported event: {:?}", event);
            Ok(())
        }
    }
}

#[tracing::instrument]
pub fn load_file(path: PathBuf) -> anyhow::Result<()> {
    let content_result = fs::OpenOptions::new().read(true).open(path.clone());

    match content_result {
        Ok(mut f) => {
            let filename = path
                .file_stem()
                .expect("file has no stem (filename)")
                .to_str()
                .expect("filename is not valid UTF8");

            let mut buf: Vec<u8> = Vec::new();

            while buf.is_empty() {
                f.read_to_end(&mut buf)?;
            }

            let _new_format =
                Transcoder.transcode(&buf, image_processing::ImageFormat::Avif, None)?;

            let storage = DiskStorage::new("/tmp/watch-out")?;
            let new_path = storage.add_new_file(File::new(filename, "avif"), &_new_format);

            // TODO: make a global settings struct for env vars
            if env::var("DELETE_ORIGINAL_FILE").is_ok() && env::var("DELETE_ORIGINAL_FILE")? == "1"
            {
                fs::remove_file(path.clone())?;
            }

            info!("'{:?}' stored at '{:?}'", &path, new_path);

            return Ok(());
        }
        Err(e) => anyhow::bail!("failed to read file '{}'", e),
    }
}
