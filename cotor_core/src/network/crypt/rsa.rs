use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
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
    
    pub fn public_key(&self) -> RSAPublicKey {
        RSAPublicKey::from_private_key(self)
    }
}

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