use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
// use crate::network::crypt::packet::{PacketDecrypter, PacketEncrypter};
// use crate::network::packet::PacketData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSAPrivateKey {
    private_key: RsaPrivateKey,
}

impl RSAPrivateKey {
    pub fn new() -> Result<Self, rsa::errors::Error> {
        let bits = 2048; // RSA key size
        let private_key = RsaPrivateKey::new(&mut rand::rng(), bits)?;
        Ok(Self { private_key })
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, rsa::errors::Error> {
        self.private_key.decrypt(Pkcs1v15Encrypt, data)
    }
}

// impl PacketDecrypter for RSAPrivateKey {
//     fn decrypt_packet(&self, data: Vec<u8>) -> Result<PacketData, &'static str> {
//         let decrypted_data = self.decrypt(&data).map_err(|_| "Decryption failed")?;
//         PacketData::decode(&decrypted_data)
//             .map_err(|_| "Failed to decode packet")
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSAPublicKey {
    public_key: RsaPublicKey,
}

impl RSAPublicKey {
    pub fn from_private_key(private_key: &RSAPrivateKey) -> Self {
        Self {
            public_key: RsaPublicKey::from(&private_key.private_key),
        }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, &'static str> {
        self.public_key
            .encrypt(&mut rand::rng(), Pkcs1v15Encrypt, data).map_err(|_| "Encryption failed")
    }
}

impl From<RSAPrivateKey> for RSAPublicKey {
    fn from(private_key: RSAPrivateKey) -> Self {
        RSAPublicKey::from_private_key(&private_key)
    }
}

// impl PacketEncrypter for RSAPublicKey {
//     fn encrypt_packet(&self, packet: &PacketData) -> Result<Vec<u8>, &'static str> {
//         let encoded_data = packet.encode().map_err(|_| "Failed to encode packet")?;
//         self.encrypt(&encoded_data)
//             .map_err(|_| "Encryption failed")
//     }
// }