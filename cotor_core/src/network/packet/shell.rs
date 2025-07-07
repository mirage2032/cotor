use crate::network::packet::AnyPacket;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellPacketAction {
    Start,
    StartConfirm,
    Stdin(String),
    Stdout(String),
    Stderr(String),
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellPacket {
    pub shell_id: Uuid,
    pub action: ShellPacketAction,
}
#[typetag::serde]
impl AnyPacket for ShellPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
