use crate::network::crypt;
use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RSAPacketData {
    PublicKey(crypt::rsa::RSAPublicKey),
    Error(String),
    Ok,
}

#[typetag::serde]
impl AnyPacketData for RSAPacketData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
