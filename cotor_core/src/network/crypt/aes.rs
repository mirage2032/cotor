// use crate::network::crypt::packet::{PacketDecrypter, PacketEncrypter};
// use crate::network::packet::PacketData;
use aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Key};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AESKey {
    key: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AESEncodedData {
    data: Vec<u8>,
}

impl AESEncodedData {
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 12 {
            return Err("Data too short for nonce");
        }
        Ok(Self {
            data: data.to_vec(),
        })
    }

    pub fn nonce(&self) -> Result<&[u8;12], &'static str> {
        if self.data.len() < 12 {
            return Err("Data too short for nonce");
        }
        self.data[0..12].try_into().map_err(|_|"Invalid nonce length")
    }

    pub fn ciphertext(&self) -> Result<&[u8], &'static str> {
        if self.data.len() < 12 {
            return Err("Data too short for nonce");
        }
        self.data[12..].try_into().map_err(|_|"Invalid nonce length")
    }
}

impl From<Vec<u8>> for AESEncodedData {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Into<Vec<u8>> for AESEncodedData {
    fn into(self) -> Vec<u8> {
        self.data
    }
}
impl AESKey {
    pub fn new() -> Result<Self, &'static str> {
        let key = Aes256Gcm::generate_key().map_err(|_| "Failed to generate key")?;
        let key = key.to_vec();
        Ok(Self {
            key: key.try_into().map_err(|_| "Invalid key length")?,
        })
    }

    pub fn from_bytes(key: &[u8; 32]) -> Result<Self, &'static str> {
        if key.len() != 32 {
            return Err("Invalid key length");
        }
        Ok(Self { key: *key })
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<AESEncodedData, &'static str> {
        let key: &Key<Aes256Gcm> = (&self.key).into();
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce().map_err(|_| "Failed to generate nonce")?;
        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|_| "Encryption failed")?;
        let mut encoded_data = Vec::with_capacity(12 + ciphertext.len());
        encoded_data.extend_from_slice(nonce.as_ref());
        encoded_data.extend_from_slice(&ciphertext);
        Ok(AESEncodedData {
            data: encoded_data,
        })
    }

    pub fn decrypt(&self, data: impl Into<AESEncodedData>) -> Result<Vec<u8>, &'static str> {
        let data = data.into();
        let key: &Key<Aes256Gcm> = (&self.key).into();
        let cipher = Aes256Gcm::new(key);
        let plaintext = cipher
            .decrypt(data.nonce()?.into(), data.ciphertext()?)
            .map_err(|_| "Decryption failed")?;
        Ok(plaintext)
    }
}

// impl PacketEncrypter for AESKey {
//     fn encrypt_packet(&self, packet: &PacketData) -> Result<Vec<u8>, &'static str> {
//         let data = bincode::serde::encode_to_vec(&packet, bincode::config::standard())
//             .map_err(|_| "Failed to encode packet")?;
//         Ok(self.encrypt(&data)?.into())
//     }
// }
// impl PacketDecrypter for AESKey {
//     fn decrypt_packet(&self, data: Vec<u8>) -> Result<PacketData, &'static str> {
//         let plaintext = self.decrypt(data)?;
//         PacketData::decode(&plaintext)
//             .map_err(|_| "Failed to decode packet")
//     }
// }
