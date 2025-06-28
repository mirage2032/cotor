use crate::network::packet::AnyPacketData;
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
pub struct ShellPacketData{
    pub shell_id: Uuid,
    pub action: ShellPacketAction,
}
#[typetag::serde]
impl AnyPacketData for ShellPacketData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
