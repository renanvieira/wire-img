use std::{env, fs, io::Read, path::Path};

use image_processing::transcoder::{Encoder, Transcoder};
use notify::Watcher;
use storage::disk::{DiskStorage, File};
use tokio::{sync::oneshot, task::JoinSet};
use tracing::{info, info_span};
use tracing_subscriber::fmt::format::FmtSpan;

#[tracing::instrument]
async fn load_images(path: &Path) -> anyhow::Result<()> {
    let (_tx, rx) = oneshot::channel::<()>();

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
    watcher.watch(path, notify::RecursiveMode::Recursive)?;

    let _ = rx.await;

    Ok(())
}

#[tracing::instrument]
fn handle_fs_event(event: notify::Event) -> anyhow::Result<()> {
    match event.kind {
        notify::EventKind::Create(create_kind) => match create_kind {
            notify::event::CreateKind::File => {
                if let Some(p) = event.paths.into_iter().next() {
                    let content_result = fs::OpenOptions::new().read(true).open(p.clone());

                    match content_result {
                        Ok(mut f) => {
                            let filename = p
                                .file_stem()
                                .expect("file has no stem (filename)")
                                .to_str()
                                .expect("filename is not valid UTF8");

                            let mut buf: Vec<u8> = Vec::new();

                            let file_read_span = info_span!("File read").entered();

                            while buf.is_empty() {
                                f.read_to_end(&mut buf)?;
                            }

                            file_read_span.exit();

                            let _new_format = Transcoder.transcode(
                                &buf,
                                image_processing::ImageFormat::Avif,
                                None,
                            )?;

                            let storage = DiskStorage::new("/tmp/watch-out")?;
                            let new_path =
                                storage.add_new_file(File::new(filename, "avif"), &_new_format);

                            info!("'{:?}' stored at '{:?}'", p, new_path);

                            return Ok(());
                        }
                        Err(e) => anyhow::bail!("failed to read file '{}'", e),
                    }
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::FULL)
        .pretty()
        .init();
    let _input_path = env::var("INPUT_PATH");

    let mut tasks = JoinSet::new();

    tasks.spawn(load_images(Path::new("/tmp/watch-test")));

    while let Some(t) = tasks.join_next().await {
        match t {
            Ok(_) => println!("Done"),
            Err(e) => eprintln!("{}", e),
        }
    }
}
