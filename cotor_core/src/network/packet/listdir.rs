use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ListDirPacketData {
    Request(String),
    Response(Vec<FileEntry>),
    Error(String),
}

#[typetag::serde]
impl AnyPacketData for ListDirPacketData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
