use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferInitData {
    pub file_location: String,
    pub file_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferProgressData {
    pub chunk_number: usize,
    pub total_chunks: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTransferAction {
    Request(FileTransferInitData),
    StartSend(FileTransferInitData),
    Progress(FileTransferProgressData),
    Error(String),
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferPacket {
    pub transfer_id: Uuid,
    pub action: FileTransferAction,
}

#[typetag::serde]
impl AnyPacketData for FileTransferPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
