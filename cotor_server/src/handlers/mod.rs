mod filetransfer;

use crate::handlers::filetransfer::FileTransferHandler;
use cotor_core::network::crypt::KeyChain;
use cotor_core::network::crypt::rsa::RSAPublicKey;
use cotor_core::network::packet::aes::AESPacketData;
use cotor_core::network::packet::rsa::RSAPacketData;
use cotor_core::network::packet::{NetworkPacket, PacketEncryption};
use tokio_util::sync::CancellationToken;
use cotor_core::network::packet::filetransfer::{FileTransferAction, FileTransferPacketData};

pub struct ClientHandlers {
    cancel_token: CancellationToken,
    file_transfer: FileTransferHandler,
    // encryption
    key_chain: KeyChain, // Assuming KeyChain is defined elsewhere in your codebase
    encryption_type: PacketEncryption,
    // callback method send_packet that receives as parameter a single impl EncodablePacket, should get async move closures
    sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
}

impl ClientHandlers {
    pub fn new(
        cancel_token: CancellationToken,
        sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
    ) -> Self {
        Self {
            cancel_token,
            file_transfer: FileTransferHandler::default(),
            key_chain: KeyChain::new_aes().expect("Failed to create KeyChain"),
            encryption_type: PacketEncryption::Plain,
            sender_queue_tx,
        }
    }

    async fn handle_rsa_packet(&mut self, packet: RSAPacketData) -> Result<(), String> {
        if let RSAPacketData::PublicKey(rsa_public_key) = packet {
            self.key_chain.rsa_public_key = Some(rsa_public_key);
            let aes_packet =
                AESPacketData::AESKey(self.key_chain.aes_key.ok_or("AES key not set")?);
            let packet = NetworkPacket::new(&aes_packet, &PacketEncryption::RSA, &self.key_chain)?;
            self.sender_queue_tx
                .send(packet)
                .await
                .map_err(|e| format!("Failed to send AES key packet: {}", e))?;
            self.encryption_type = PacketEncryption::AES;
        }
        Ok(())
    }

    pub async fn handle_packet(&mut self, packet: &NetworkPacket) -> Result<(), String> {
        let packet_data = packet.decrypt(&self.key_chain)?;
        if let Some(rsa_packet) = packet_data.as_any().downcast_ref::<RSAPacketData>() {
            self.handle_rsa_packet(rsa_packet.clone()).await?;
        }
        if let Some(file_packet) = packet_data.as_any().downcast_ref::<FileTransferPacketData>() {
            self.file_transfer.handle(file_packet).await?;
        }
        Ok(())
    }
}
