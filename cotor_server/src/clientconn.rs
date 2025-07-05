use crate::handlers::ClientHandlers;
use arti_client::DataStream;
use cotor_core::network::packet::NetworkPacket;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{event, instrument, span, Instrument};
use uuid::Uuid;

#[derive(Debug)]
struct ClientConnData {
    cancel_token: CancellationToken,
    stream: Arc<Mutex<DataStream>>,
    packet_receiver: tokio::task::JoinHandle<Result<(), String>>,
    packet_sender: tokio::task::JoinHandle<Result<(), String>>,
    sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
}

#[derive(Debug)]
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
                    tokio::spawn(cb(&uuid).instrument(
                        span!(tracing::Level::INFO, "kill_callback"),
                    ));
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
        ).instrument(
            span!(tracing::Level::INFO, "packet_receiver_task", uuid = %uuid),
        ));
        let sender_task = tokio::spawn(Self::packet_sender_task(
            stream.clone(),
            cancel_token.clone(),
            sender_queue_rx,
            con_kill_cb,
        ).instrument(
            span!(tracing::Level::INFO, "packet_sender_task", uuid = %uuid),
        ));
        let tasks = Some(ClientConnData {
            cancel_token,
            stream,
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
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    event!(tracing::Level::INFO, "Cancellation token triggered, stopping receiver task");
                    return Ok(());
                }
                result = NetworkPacket::from_stream_with_cancel(stream.clone(),cancel_token.clone()) => {
                    match result {
                        Ok(packet) => {
                            event!(tracing::Level::INFO, "Received packet: {:?}", packet.header);
                            if let Err(err) = handlers.write().await.handle_packet(&packet).await {
                                let error = format!("Failed to handle packet: {err}");
                                kill_cb(error).await;
                                return Err(err.to_string());
                            }
                        }
                        Err(e) => {
                            let error = format!("Failed to read packet: {e}");
                            kill_cb(error).await;
                            return Err(e);
                        }
                    }
                }
            }
        }
    }

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
        loop {
            tokio::select! {
            _ = cancel_token.cancelled() => {
                event!(tracing::Level::INFO, "Cancellation token triggered, stopping sender task");
                return Ok(());
            }
            maybe_packet = sender_queue_rx.recv() => {
                match maybe_packet {
                    Some(packet) => {
                        event!(tracing::Level::INFO, "Sending packet: {:?}", packet.header);
                        let mut stream_lock = stream.lock().await;
                        match packet.send(stream_lock.deref_mut()).await {
                            Ok(()) => {
                                event!(tracing::Level::TRACE, "Packet sent successfully");
                            }
                            Err(e) => {
                                let error = format!("Failed to send packet: {e}");
                                kill_cb(error).await;
                                return Err(e);
                            }
                        }
                    }
                    None => {
                        event!(tracing::Level::INFO, "Sender queue closed, stopping sender task");
                        return Ok(());
                    }
                }
            }
        }
        }
    }

    #[instrument(skip(self), name = "close_connection")]
    pub async fn close(&mut self) {
        if let Some(tasks) = self.tasks.take() {
            event!(
                tracing::Level::INFO,
                "Closing connection with UUID: {}",
                self.uuid
            );
            tasks.cancel_token.cancel();
            if let Err(err) = tasks.stream.lock().await.deref_mut().shutdown().await{
                event!(
                    tracing::Level::ERROR,
                    "Failed to close stream for connection with UUID: {}: {}",
                    self.uuid,
                    err
                );
            }
            let receiver_result = tasks.packet_receiver.await;
            let sender_result = tasks.packet_sender.await;
            if let Err(e) = receiver_result {
                event!(tracing::Level::ERROR, "Receiver task failed: {}", e);
            }
            if let Err(e) = sender_result {
                event!(tracing::Level::ERROR, "Sender task failed: {}", e);
            }
            event!(
                tracing::Level::INFO,
                "Connection with UUID: {} closed",
                self.uuid
            );
        }
    }
}

impl Drop for ClientConnection {
    fn drop(&mut self) {
        if let Some(tasks) = self.tasks.take() {
            event!(
                tracing::Level::INFO,
                "Dropping connection with UUID: {}",
                self.uuid
            );
            tasks.cancel_token.cancel();
            tasks.packet_receiver.abort();
            tasks.packet_sender.abort();
            event!(
                tracing::Level::INFO,
                "Connection with UUID: {} dropped",
                self.uuid
            );
        }
    }
}
