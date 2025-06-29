use crate::network::crypt;
use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType{
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub message: String,
    pub level: MessageType,
}

#[typetag::serde]
impl AnyPacketData for MessageData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
