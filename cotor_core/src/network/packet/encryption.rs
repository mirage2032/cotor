use serde::{Deserialize, Serialize};
use crate::network::crypt;
use crate::network::packet::AnyPacket;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionPacket {
    RSAPublicKey(crypt::rsa::RSAPublicKey),
    AESKey(crypt::aes::AESKey),
}

#[typetag::serde]
impl AnyPacket for EncryptionPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
