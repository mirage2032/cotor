use crate::network::crypt;
use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AESPacket {
    AESKey(crypt::aes::AESKey),
    Error(String),
    Ok,
}

#[typetag::serde]
impl AnyPacketData for AESPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
