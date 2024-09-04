use std::{env, fs, io::Read, path::PathBuf, sync::Arc};

use configuration::ImageEncoding;
use image_processing::{
    transcoder::{Encoder, Transcoder},
    ImageFormat,
};
use notify::{Config, RecommendedWatcher, Watcher};
use storage::disk::{DiskStorage, File};
use tracing::{error, info};

use crate::APIState;

#[derive(Debug)]
pub struct ImageWatcher<'a> {
    path: PathBuf,
    watcher: RecommendedWatcher,
    receiver_channel: tokio::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>,
    state: Arc<APIState<'a>>,
}

impl<'a> ImageWatcher<'a> {
    pub fn new(path: PathBuf, app_state: Arc<APIState<'a>>) -> anyhow::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
                // TODO: handle error
            },
            Config::default(),
        )?;

        Ok(Self {
            watcher,
            path,
            receiver_channel: rx,
            state: app_state,
        })
    }

    pub async fn watch(mut self) -> anyhow::Result<()> {
        // let (_, rx) = tokio::sync::oneshot::channel::<()>();

        self.watcher
            .watch(&self.path, notify::RecursiveMode::NonRecursive)?;

        while let Some(event_result) = self.receiver_channel.recv().await {
            match event_result {
                Ok(event) => {
                    let _ = self.handle_fs_event(event).await;
                }
                Err(e) => {
                    error!(
                        "Error while tryint to receive events from file watcher: {}",
                        e
                    );
                    continue;
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument]
    async fn handle_fs_event(&self, event: notify::Event) -> anyhow::Result<()> {
        match event.kind {
            notify::EventKind::Create(create_kind) => match create_kind {
                notify::event::CreateKind::File => {
                    if let Some(p) = event.paths.into_iter().next() {
                        let _ = self.load_file(p, &self.state.transcoder);
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
    pub fn load_file(&self, path: PathBuf, transcoder: &Transcoder) -> anyhow::Result<()> {
        let content_result = fs::OpenOptions::new().read(true).open(path.clone());

        match content_result {
            Ok(mut f) => {
                let filename = path
                    .file_stem()
                    .expect("file has no stem (filename)")
                    .to_str()
                    .expect("filename is not valid UTF8");

                let extension = path
                    .extension()
                    .expect("file has no stem (filename)")
                    .to_str()
                    .expect("filename is not valid UTF8");

                let mut buf: Vec<u8> = Vec::new();

                while buf.is_empty() {
                    f.read_to_end(&mut buf)?;
                }

                let storage_format = match self.state.configuration.image.storage_format {
                    ImageEncoding::PNG => ImageFormat::Png,
                    ImageEncoding::JPEG => ImageFormat::Jpeg,
                    ImageEncoding::AVIF => ImageFormat::Avif,
                };

                let _new_format = transcoder.transcode(
                    &buf,
                    extension.to_owned(),
                    storage_format,
                    // image_processing::ImageFormat::Avif,
                    None,
                )?;

                let storage = DiskStorage::new("/tmp/watch-out")?;
                let new_path = storage.add_new_file(File::new(filename, "avif"), &_new_format);

                // TODO: make a global settings struct for env vars
                if env::var("DELETE_ORIGINAL_FILE").is_ok()
                    && env::var("DELETE_ORIGINAL_FILE")? == "1"
                {
                    fs::remove_file(path.clone())?;
                }

                info!("'{:?}' stored at '{:?}'", &path, new_path);

                return Ok(());
            }
            Err(e) => anyhow::bail!("failed to read file '{}'", e),
        }
    }
}
