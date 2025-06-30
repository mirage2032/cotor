mod filetransfer;

use cotor_core::network::packet::NetworkPacket;
use crate::handlers::filetransfer::FileTransferHandler;

struct PacketHandlers {
    file_transfer: FileTransferHandler,
    sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
}