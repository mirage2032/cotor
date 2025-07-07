use crate::network::packet::AnyPacket;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
pub struct FileTransferPacket {
    pub transfer_id: Uuid,
    pub action: FileTransferAction,
}

#[typetag::serde]
impl AnyPacket for FileTransferPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
