use crate::network::crypt::aes::AESKey;
use crate::network::crypt::rsa::{RSAPrivateKey, RSAPublicKey};

pub mod aes;
pub mod rsa;

#[derive(Debug, Clone)]
pub struct KeyChain {
    pub aes_key: Option<AESKey>,
    pub rsa_public_key: Option<RSAPublicKey>,
    pub rsa_private_key: Option<RSAPrivateKey>,
}

impl KeyChain {
    pub fn new() -> Result<Self, String> {
        let aes_key = AESKey::new()?;
        let rsa_private_key = RSAPrivateKey::new()?;
        let rsa_public_key = rsa_private_key.public_key();
        let key_chain = KeyChain {
            aes_key: Some(aes_key),
            rsa_public_key: Some(rsa_public_key),
            rsa_private_key: Some(rsa_private_key),
        };
        Ok(key_chain)
    }

    pub fn new_aes() -> Result<Self, String> {
        let aes_key = AESKey::new()?;
        let key_chain = KeyChain {
            aes_key: Some(aes_key),
            rsa_public_key: None,
            rsa_private_key: None,
        };
        Ok(key_chain)
    }

    pub fn new_rsa() -> Result<Self, String> {
        let rsa_private_key = RSAPrivateKey::new()?;
        let rsa_public_key = rsa_private_key.public_key();
        let key_chain = KeyChain {
            aes_key: None,
            rsa_public_key: Some(rsa_public_key),
            rsa_private_key: Some(rsa_private_key),
        };
        Ok(key_chain)
    }
}
