use crate::handlers::EncryptionData;
use async_compression::tokio::write::GzipEncoder;
use cotor_core::network::crypt::KeyChain;
use cotor_core::network::packet::NetworkPacket;
use cotor_core::network::packet::filetransfer::{
    FileTransferAction, FileTransferInitData, FileTransferPacketData, FileTransferProgressData,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::RwLock;
use tokio_tar::Builder;
use tokio_util::sync::CancellationToken;
use tracing::event;
use uuid::Uuid;

#[derive(Debug, Copy, Clone)]
pub struct TransferProgress {
    chunk_number: u32,
    total_chunks: u32,
}
#[derive(Debug, Clone)]
pub struct FileTransferData {
    pub destination: PathBuf,
    pub source: PathBuf,
}
const CHUNK_SIZE: u32 = 1024 * 16;

#[derive(Debug)]
pub struct UploadTask {
    pub data: FileTransferData,
    pub cancel_token: CancellationToken,
    pub progress: Arc<RwLock<TransferProgress>>,
    pub task: tokio::task::JoinHandle<Result<(), String>>,
}

impl UploadTask {
    pub async fn new(
        data: FileTransferData,
        cancel_token: CancellationToken,
        sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
        enc_data: Weak<RwLock<EncryptionData>>,
    ) -> Result<(Self, Uuid), String> {
        //combine as path source and name
        let transfer_id = Uuid::new_v4();
        let temp_file =
            NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;
        let file = tokio::fs::File::open(temp_file.path())
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let buf = BufWriter::new(file);
        let gz = GzipEncoder::new(buf);
        let mut builder = Builder::new(gz);
        builder
            .append_dir_all("", &data.source)
            .await
            .map_err(|e| format!("Failed to append directory: {}", e))?;
        let file = builder
            .into_inner()
            .await
            .map_err(|e| format!("Failed to finalize tar: {}", e))?
            .into_inner()
            .into_inner();
        let file_size: u32 = file
            .metadata()
            .await
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .len() as u32;
        let total_chunks = file_size.div_ceil(CHUNK_SIZE);
        let mut buf = BufReader::new(file);
        let upload_init = FileTransferPacketData {
            transfer_id,
            action: FileTransferAction::StartSend(FileTransferInitData {
                file_location: data.destination.clone(),
                total_chunks,
                file_size,
            }),
        };
        let enc_data = enc_data.upgrade().ok_or("Encryption data not available")?;
        let init_packet = NetworkPacket::new(
            &upload_init,
            &enc_data.read().await.encryption_type,
            &enc_data.read().await.key_chain,
        )?;
        sender_queue_tx
            .send(init_packet)
            .await
            .map_err(|e| format!("Failed to send upload init packet: {}", e))?;
        let cancel_token_clone = cancel_token.clone();
        let progress = Arc::new(RwLock::new(TransferProgress {
            chunk_number: 0,
            total_chunks,
        }));
        let progress_clone = progress.clone();
        let task = tokio::spawn(async move {
            let mut chunk_number = 0;
            while !cancel_token_clone.is_cancelled() {
                let mut data = vec![0; CHUNK_SIZE as usize];
                let bytes_read = buf
                    .read(&mut data)
                    .await
                    .map_err(|e| format!("Failed to read from file: {}", e))?;
                if bytes_read == 0 {
                    break; // EOF
                }
                let progress = FileTransferPacketData {
                    transfer_id,
                    action: FileTransferAction::Progress(FileTransferProgressData {
                        chunk_number,
                        total_chunks,
                        data,
                    }),
                };
                let progress_packet = NetworkPacket::new(
                    &progress,
                    &enc_data.read().await.encryption_type,
                    &enc_data.read().await.key_chain,
                )?;
                sender_queue_tx
                    .send(progress_packet)
                    .await
                    .map_err(|e| format!("Failed to send progress packet: {}", e))?;
                progress_clone.write().await.chunk_number = chunk_number;
                chunk_number += 1;
            }
            Ok(())
        });
        Ok((
            UploadTask {
                data,
                cancel_token,
                progress,
                task,
            },
            transfer_id,
        ))
    }
}

#[derive(Debug)]
pub struct DownloadTask {
    pub data: FileTransferData,
    pub progress: Option<TransferProgress>,
    pub buf: BufWriter<tokio::fs::File>,
}

impl DownloadTask {
    pub async fn new(
        data: FileTransferData,
        sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
        enc_data: Weak<RwLock<EncryptionData>>,
    ) -> Result<(Self, Uuid), String> {
        let transfer_id = Uuid::new_v4();
        let temp_file =
            NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;
        let file = tokio::fs::File::create(temp_file.path())
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;
        let buf = BufWriter::new(file);
        let download_request = FileTransferPacketData {
            transfer_id,
            action: FileTransferAction::Request(
                data.source
                    .to_str()
                    .ok_or("Invalid source path")?
                    .to_string(),
            ),
        };
        let enc_data = enc_data.upgrade().ok_or("Encryption data not available")?;
        let packet = NetworkPacket::new(
            &download_request,
            &enc_data.read().await.encryption_type,
            &enc_data.read().await.key_chain,
        )
        .map_err(|e| format!("Failed to create network packet: {}", e))?;
        sender_queue_tx
            .send(packet)
            .await
            .map_err(|e| format!("Failed to send download request packet: {}", e))?;
        let progress = None;
        Ok((
            DownloadTask {
                data,
                progress,
                buf,
            },
            transfer_id,
        ))
    }

    pub async fn save(mut self) -> Result<(), String> {
        self.buf
            .flush()
            .await
            .map_err(|e| format!("Failed to flush buffer: {}", e))?;
        let file = self.buf.into_inner();
        let reader = BufReader::new(file);
        let mut archive = tokio_tar::Archive::new(reader);
        archive
            .unpack(&self.data.destination)
            .await
            .map_err(|e| format!("Failed to unpack tar: {}", e))
    }

    pub async fn receive_progress(
        mut self,
        progress_data: &FileTransferProgressData,
    ) -> Result<Option<Self>, String> {
        let progress = TransferProgress {
            chunk_number: progress_data.chunk_number,
            total_chunks: progress_data.total_chunks,
        };
        if let Some(current_progress) = &self.progress
            && current_progress.chunk_number != progress.chunk_number - 1
        {
            return Err(format!(
                "Received out of order chunk: expected {}, got {}",
                current_progress.chunk_number + 1,
                progress.chunk_number
            ));
        }
        self.progress = Some(progress);
        self.buf
            .write_all(&progress_data.data)
            .await
            .map_err(|e| format!("Failed to write to file: {e}"))?;
        if progress.chunk_number == progress.total_chunks {
            event!(
                tracing::Level::INFO,
                "Download complete. Saving file to {}",
                self.data.destination.display()
            );
            self.save().await?;
            Ok(None) // Indicate that the download is complete
        } else {
            Ok(Some(self))
        }
    }
}

#[derive(Default, Debug)]
pub struct FileTransferTasks {
    pub upload_tasks: HashMap<Uuid, UploadTask>,
    pub download_tasks: HashMap<Uuid, DownloadTask>,
}

impl FileTransferTasks {}

pub struct FileTransferHandler {
    cancel_token: CancellationToken,
    tasks: FileTransferTasks,
    sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
    pub enc_data: Weak<RwLock<EncryptionData>>,
    // pub download_entries: HashMap<Uuid, FileTransferData>,
}

impl FileTransferHandler {
    pub fn new(
        cancel_token: CancellationToken,
        sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
        enc_data: Weak<RwLock<EncryptionData>>,
    ) -> Self {
        FileTransferHandler {
            cancel_token,
            tasks: FileTransferTasks::default(),
            sender_queue_tx,
            enc_data,
        }
    }
    pub async fn handle(&mut self, file: &FileTransferPacketData) -> Result<(), String> {
        match &file.action {
            FileTransferAction::Progress(progress_data) => {
                let download = self
                    .tasks
                    .download_tasks
                    .remove(&file.transfer_id)
                    .ok_or_else(|| {
                        format!(
                            "No download task found for transfer ID: {}",
                            file.transfer_id
                        )
                    })?;
                let download = download.receive_progress(progress_data).await?;
                if let Some(download) = download {
                    self.tasks.download_tasks.insert(file.transfer_id, download);
                }
            }
            FileTransferAction::Error(message) => {
                return Err(format!("File transfer error: {}", message));
            }
            _ => {}
        }
        Ok(())
    }
    pub async fn upload(&mut self, data: FileTransferData) -> Result<Uuid, String> {
        let (task, transfer_id) = UploadTask::new(
            data,
            self.cancel_token.child_token(),
            self.sender_queue_tx.clone(),
            self.enc_data.clone(),
        )
        .await?;
        self.tasks.upload_tasks.insert(transfer_id, task);
        Ok(transfer_id)
    }

    pub async fn download(&mut self, data: FileTransferData) -> Result<Uuid, String> {
        let (task, transfer_id) =
            DownloadTask::new(data, self.sender_queue_tx.clone(), self.enc_data.clone()).await?;
        self.tasks.download_tasks.insert(transfer_id, task);
        Ok(transfer_id)
    }
}
