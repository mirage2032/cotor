pub mod aes;
pub mod rsa;

pub struct KeyChain {
    pub aes_key: Option<aes::AESKey>,
    pub rsa_public_key: Option<rsa::RSAPublicKey>,
    pub rsa_private_key: Option<rsa::RSAPrivateKey>,
}

impl Default for KeyChain {
    fn default() -> Self {
        KeyChain {
            aes_key: None,
            rsa_public_key: None,
            rsa_private_key: None,
        }
    }
}