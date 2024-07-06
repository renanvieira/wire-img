use std::{env, fs, io::Read, path::Path, time::Duration};

use image_processing::transcoder::{Encoder, Transcoder};
use notify::{Config, RecommendedWatcher, Watcher};

async fn load_images(path: &Path, _duration: tokio::time::Interval) -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let watcher = RecommendedWatcher::new(tx, Config::default());

    let mut watcher = match watcher {
        Ok(w) => w,
        Err(e) => anyhow::bail!("Filesystem watcher could not be initialized:{}", e),
    };

    watcher.watch(path, notify::RecursiveMode::Recursive)?;

    loop {
        // TODO: make this run every few seconds and not loop infinitely.
        for res in &rx {
            match res {
                Ok(event) => {
                    let res = handle_fs_event(event);
                    match res {
                        Ok(_) => continue,
                        Err(e) => tracing::error!("Error while handling the fs events: {e:?}"),
                    }
                }
                Err(error) => {
                    tracing::error!("Error: {error:?}");
                }
            }
        }
    }
}

fn handle_fs_event(event: notify::Event) -> anyhow::Result<()> {
    match event.kind {
        notify::EventKind::Create(create_kind) => match create_kind {
            notify::event::CreateKind::File => {
                if let Some(p) = event.paths.into_iter().next() {
                    let content_result = fs::OpenOptions::new().read(true).open(p.clone());

                    match content_result {
                        Ok(mut f) => {
                            let mut buf: Vec<u8> = Vec::new();
                            f.read_to_end(&mut buf)?;
                            let _new_format = Transcoder.transcode(
                                &buf,
                                image_processing::ImageFormat::Avif,
                                None,
                            )?;

                            // TODO: call storage to store the avif version
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
    tracing_subscriber::fmt().pretty().init();
    let _input_path = env::var("INPUT_PATH");

    let t = tokio::spawn(load_images(
        Path::new("/tmp/watch-test"),
        tokio::time::interval(Duration::from_secs_f32(1.0)),
    ))
    .await;

    match t {
        Ok(_) => println!("Done"),
        Err(e) => eprintln!("{}", e),
    }
}
