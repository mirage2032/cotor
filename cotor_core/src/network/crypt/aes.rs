use aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Key};
use serde::{Deserialize, Serialize};
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AESKey {
    key: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AESEncodedData {
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}

impl AESEncodedData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(12 + self.ciphertext.len());
        data.extend_from_slice(&self.nonce);
        data.extend_from_slice(&self.ciphertext);
        data
    }
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 12 {
            return Err("Data too short");
        }
        let nonce: [u8; 12] = data[0..12].try_into().map_err(|_| "Invalid nonce length")?;
        let ciphertext = data[12..].to_vec();
        Ok(Self { nonce, ciphertext })
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
        Ok(AESEncodedData {
            nonce: nonce.into(),
            ciphertext,
        })
    }

    pub fn decrypt(&self, data: &AESEncodedData) -> Result<Vec<u8>, &'static str> {
        let key: &Key<Aes256Gcm> = (&self.key).into();
        let cipher = Aes256Gcm::new(key);
        let plaintext = cipher
            .decrypt((&data.nonce).into(), data.ciphertext.as_ref())
            .map_err(|_| "Decryption failed")?;
        Ok(plaintext)
    }
}
