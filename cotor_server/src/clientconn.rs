use crate::handlers::ClientHandlers;
use arti_client::DataStream;
use cotor_core::network::crypt::KeyChain;
use cotor_core::network::packet::{NetworkPacket, PacketEncryption};
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{event, instrument};
use uuid::Uuid;

struct ClientConnData {
    cancel_token: CancellationToken,
    packet_receiver: tokio::task::JoinHandle<Result<(), String>>,
    packet_sender: tokio::task::JoinHandle<Result<(), String>>,
    sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
}

pub struct ClientConnection {
    uuid: Uuid,
    tasks: Option<ClientConnData>,
    handlers: Arc<RwLock<ClientHandlers>>,
}

impl ClientConnection {
    #[instrument(name = "new_conn", skip(stream, cancel_token, kill_cb))]
    pub fn new<F, Fut>(
        stream: DataStream,
        cancel_token: CancellationToken,
        kill_cb: F,
    ) -> Result<Self, String>
    where
        F: FnOnce(&Uuid) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let uuid = Uuid::new_v4();
        event!(
            tracing::Level::INFO,
            "Starting connection with UUID: {}",
            uuid
        );
        let kill_cb = Arc::new(Mutex::new(Some(kill_cb)));
        let cancel_token_clone = cancel_token.clone();
        let con_kill_cb = async move |message| {
            event!(
                tracing::Level::INFO,
                "Killing connection with UUID: {} due to: {}",
                uuid,
                message
            );
            cancel_token_clone.cancel();
            match kill_cb.lock().await.take() {
                None => {
                    event!(
                        tracing::Level::ERROR,
                        "Could not call kill callback for connection with UUID: {}. Might've already been called",
                        uuid
                    );
                }
                Some(cb) => {
                    cb(&uuid).await;
                    event!(
                        tracing::Level::INFO,
                        "Kill callback for connection with UUID: {} called successfully",
                        uuid
                    );
                }
            }
        };
        let (sender_queue_tx, sender_queue_rx) = tokio::sync::mpsc::channel(100); // Adjust the buffer size as needed it might be too big
        let stream = Arc::new(Mutex::new(stream));
        let handlers = Arc::new(RwLock::new(ClientHandlers::new(
            uuid,
            cancel_token.clone(),
            sender_queue_tx.clone(),
        )?));
        let receiver_task = tokio::spawn(Self::packet_receiver_task(
            stream.clone(),
            handlers.clone(),
            cancel_token.clone(),
            con_kill_cb.clone(),
        ));
        let sender_task = tokio::spawn(Self::packet_sender_task(
            stream.clone(),
            cancel_token.clone(),
            sender_queue_rx,
            con_kill_cb,
        ));
        let tasks = Some(ClientConnData {
            cancel_token,
            packet_receiver: receiver_task,
            packet_sender: sender_task,
            sender_queue_tx,
        });
        Ok(Self {
            uuid,
            tasks,
            handlers,
        })
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    #[instrument(skip_all)]
    async fn packet_receiver_task<F, Fut>(
        stream: Arc<Mutex<DataStream>>,
        handlers: Arc<RwLock<ClientHandlers>>,
        cancel_token: CancellationToken,
        kill_cb: F,
    ) -> Result<(), String>
    where
        F: FnOnce(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        while !cancel_token.is_cancelled() {
            match NetworkPacket::from_stream(stream.lock().await.deref_mut()).await {
                Ok(packet) => {
                    event!(tracing::Level::INFO, "Received packet: {:?}", packet.header);
                    handlers.write().await.handle_packet(&packet).await?;
                }
                Err(e) => {
                    let error = format!("Failed to read packet: {}", e);
                    kill_cb(error).await;
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn packet_sender_task<F, Fut>(
        stream: Arc<Mutex<DataStream>>,
        cancel_token: CancellationToken,
        mut sender_queue_rx: tokio::sync::mpsc::Receiver<NetworkPacket>,
        kill_cb: F,
    ) -> Result<(), String>
    where
        F: FnOnce(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        while !cancel_token.is_cancelled() {
            if let Some(packet) = sender_queue_rx.recv().await {
                event!(tracing::Level::INFO, "Sending packet: {:?}", packet.header);
                let mut stream_lock = stream.lock().await;
                match packet.send(stream_lock.deref_mut()).await {
                    Ok(()) => {
                        event!(tracing::Level::TRACE, "Packet sent successfully");
                    }
                    Err(e) => {
                        let error = format!("Failed to send packet: {}", e);
                        kill_cb(error).await;
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(tasks) = self.tasks.take() {
            tasks.cancel_token.cancel();
        }
    }
}

impl Drop for ClientConnection {
    fn drop(&mut self) {
        self.stop();
    }
}
