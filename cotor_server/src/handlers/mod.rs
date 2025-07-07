mod filetransfer;
mod screenshot;

use std::any::TypeId;
use crate::handlers::filetransfer::FileTransferHandler;
use crate::handlers::screenshot::ScreenshotHandler;
use cotor_core::network::crypt::KeyChain;
use cotor_core::network::crypt::rsa::RSAPublicKey;
use cotor_core::network::packet::cotor::CotorPacket;
use cotor_core::network::packet::filetransfer::{FileTransferAction, FileTransferPacket};
use cotor_core::network::packet::screenshot::ScreenShotPacket;
use cotor_core::network::packet::{NetworkPacket, PacketEncryption};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::event;
use uuid::Uuid;
use cotor_core::network::packet::encryption::EncryptionPacket;

#[derive(Debug)]

struct EncryptionData {
    key_chain: KeyChain, // Assuming KeyChain is defined elsewhere in your codebase
    encryption_type: PacketEncryption,
}

#[derive(Debug)]
pub struct ClientHandlers {
    cancel_token: CancellationToken,

    file_transfer: FileTransferHandler,
    screenshot: ScreenshotHandler,
    // encryption
    encryption_data: Arc<RwLock<EncryptionData>>,
    // callback method send_packet that receives as parameter a single impl EncodablePacket, should get async move closures
    sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
}

impl ClientHandlers {
    pub fn new(
        uuid: Uuid,
        cancel_token: CancellationToken,
        sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
    ) -> Result<Self, String> {
        let encryption_data = Arc::new(RwLock::new(EncryptionData {
            key_chain: KeyChain::new_aes().expect("Failed to create KeyChain"),
            encryption_type: PacketEncryption::Plain,
        }));
        //get part of uuid for filename
        let uuid_part = uuid.to_string().replace('-', "");
        let uuid_a_part = uuid_part.get(0..8).unwrap_or("default");
        let uuid_b_part = uuid_part.get(8..16).unwrap_or("default");
        let screenshot = ScreenshotHandler::new_named_temp(uuid_a_part, uuid_b_part.to_string())?;
        Ok(ClientHandlers {
            cancel_token: cancel_token.clone(),
            file_transfer: FileTransferHandler::new(
                cancel_token,
                sender_queue_tx.clone(),
                Arc::downgrade(&encryption_data),
            ),

            screenshot,
            encryption_data,
            sender_queue_tx,
        })
    }

    async fn handle_rsa_packet(&mut self, rsa_public_key: RSAPublicKey) -> Result<(), String> {
        self.encryption_data.write().await.key_chain.rsa_public_key = Some(rsa_public_key);
        let aes_packet = EncryptionPacket::AESKey(
            self.encryption_data
                .read()
                .await
                .key_chain
                .aes_key
                .ok_or("AES key not set")?,
        );
        let packet = NetworkPacket::new(
            &aes_packet,
            &PacketEncryption::RSA,
            &self.encryption_data.read().await.key_chain,
        )?;
        self.sender_queue_tx
            .send(packet)
            .await
            .map_err(|e| format!("Failed to send AES key packet: {e}"))?;
        self.encryption_data.write().await.encryption_type = PacketEncryption::AES;
        Ok(())
    }

    pub async fn handle_packet(&mut self, packet: &NetworkPacket) -> Result<(), String> {
        let packet_data = packet.decrypt(&self.encryption_data.read().await.key_chain)?;
        let any_box = packet_data.as_any_box();

        match any_box.type_id() {
            id if id == TypeId::of::<EncryptionPacket>() => {
                let encryption_packet = any_box.downcast::<EncryptionPacket>().unwrap();
                match *encryption_packet {
                    EncryptionPacket::RSAPublicKey(rsa_public_key) => {
                        self.handle_rsa_packet(rsa_public_key).await?;
                    }
                    _ => {
                        event!(tracing::Level::WARN, "Received unexpected encryption packet type: {:?}", encryption_packet);
                    }
                }
            }
            id if id == TypeId::of::<FileTransferPacket>() => {
                let file_packet = any_box.downcast::<FileTransferPacket>().unwrap();
                self.file_transfer.handle(&file_packet).await?;
            }
            id if id == TypeId::of::<CotorPacket>() => {
                let cotor_packet = any_box.downcast::<CotorPacket>().unwrap();
                match *cotor_packet {
                    CotorPacket::Debug(message) => {
                        event!(tracing::Level::DEBUG, "Received Cotor debug message: {:?}",message);
                    }
                    _ => {}
                }
            }
            id if id == TypeId::of::<ScreenShotPacket>() => {
                let screenshot_packet = any_box.downcast::<ScreenShotPacket>().unwrap();
                self.screenshot.handle(*screenshot_packet).await?;
            }
            _ => { event!(tracing::Level::WARN,"Received unknown packet type: {:?}",any_box.type_id());
            }
        }
        Ok(())
    }
}
