use crate::network::crypt;
use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RSAPacket {
    PublicKey(crypt::rsa::RSAPublicKey),
    Error(String),
    Ok,
}

#[typetag::serde]
impl AnyPacketData for RSAPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
