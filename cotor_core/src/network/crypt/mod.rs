pub mod aes;
pub mod rsa;

pub struct KeyChain {
    pub aes_key: Option<aes::AESKey>,
    pub rsa_public_key: Option<rsa::RSAPublicKey>,
    pub rsa_private_key: Option<rsa::RSAPrivateKey>,
}

impl KeyChain {
    pub fn encrypt_aes(&self, data: &[u8]) -> Result<Vec<u8>, &'static str> {
        if let Some(aes_key) = &self.aes_key {
            aes_key.encrypt(data).map(|encoded_data| encoded_data.into())
        } else {
            Err("AES key not set")
        }
    }
    pub fn decrypt_aes(&self, data: impl Into<aes::AESEncodedData>) -> Result<Vec<u8>, &'static str> {
        if let Some(aes_key) = &self.aes_key {
            aes_key.decrypt(data.into())
        } else {
            Err("AES key not set")
        }
    }
    
    pub fn encrypt_rsa(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(rsa_public_key) = &self.rsa_public_key {
            rsa_public_key.encrypt(data).map_err(|_| "RSA encryption failed".to_string())
        } else {
            Err("RSA public key not set".to_string())
        }
    }
    
    pub fn decrypt_rsa(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(rsa_private_key) = &self.rsa_private_key {
            rsa_private_key.decrypt(data).map_err(|_| "RSA decryption failed".to_string())
        } else {
            Err("RSA private key not set".to_string())
        }
    }
}
