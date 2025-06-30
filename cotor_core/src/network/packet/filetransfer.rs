use std::path::PathBuf;
use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferInitData {
    pub file_location: PathBuf,
    pub total_chunks: u32,
    pub file_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferProgressData {
    pub chunk_number: u32,
    pub total_chunks: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTransferAction {
    Request(String),
    StartSend(FileTransferInitData),
    Progress(FileTransferProgressData),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferPacketData {
    pub transfer_id: Uuid,
    pub action: FileTransferAction,
}

#[typetag::serde]
impl AnyPacketData for FileTransferPacketData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
