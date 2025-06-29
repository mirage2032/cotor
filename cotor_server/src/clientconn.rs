use arti_client::DataStream;
use cotor_core::network::packet::NetworkPacket;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{event, instrument};
use uuid::Uuid;

struct ClientConnTasks {
    cancel_token: CancellationToken,
    packet_receiver: tokio::task::JoinHandle<Result<(), String>>,
    packet_sender: tokio::task::JoinHandle<Result<(), String>>,
    sender_queue_tx: tokio::sync::mpsc::Sender<NetworkPacket>,
}
type ConnKillCb = Option<Box<dyn FnOnce(&Uuid) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>>;

pub struct ClientConnection {
    uuid: Uuid,
    tasks: Option<ClientConnTasks>,
    kill_cb: ConnKillCb,
}

impl ClientConnection {
    #[instrument(name="new_conn", skip(stream, cancel_token, kill_cb))]
    pub fn new<F,Fut>(stream: DataStream, cancel_token: CancellationToken, kill_cb: F) -> Self
    where
        F: FnOnce(&Uuid) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let uuid = Uuid::new_v4();
        event!(tracing::Level::INFO, "Starting connection with UUID: {}", uuid);
        let stream = Arc::new(Mutex::new(stream));
        let receiver_task = tokio::spawn(Self::packet_receiver_task(stream.clone(), cancel_token.clone()));
        let (sender_queue_tx, sender_queue_rx) = tokio::sync::mpsc::channel(100); // Adjust the buffer size as needed it might be too big
        let sender_task = tokio::spawn(Self::packet_sender_task(stream.clone(), cancel_token.clone(), sender_queue_rx));
        let tasks = Some(ClientConnTasks {
            cancel_token,
            packet_receiver: receiver_task,
            packet_sender: sender_task,
            sender_queue_tx,
        });
        Self { uuid, tasks, kill_cb: Some(Box::new(move |uuid|Box::pin(kill_cb(uuid)))) }
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    async fn task_kill_cb(&mut self, message: &str) {
        self.stop();
        self.kill_cb.take().map(async |cb| {
            event!(tracing::Level::INFO, "Killing connection with UUID: {} due to: {}", self.uuid, message);
            cb(&self.uuid).await;
        });
    }

    #[instrument]
    async fn packet_receiver_task(
        stream: Arc<Mutex<DataStream>>,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        while !cancel_token.is_cancelled() {
            match NetworkPacket::from_stream(stream.lock().await.deref_mut()).await {
                Ok(packet) => {
                    event!(tracing::Level::INFO, "Received packet: {:?}", packet.header);
                }
                Err(e) => {
                    event!(tracing::Level::ERROR, "Failed to read packet,closing connection: {}", e);
                    cancel_token.cancel();
                    return Err(e)
                }
            }
        }
        Ok(())
    }

    #[instrument]
    async fn packet_sender_task(
        stream: Arc<Mutex<DataStream>>,
        cancel_token: CancellationToken,
        mut sender_queue_rx: tokio::sync::mpsc::Receiver<NetworkPacket>,
    ) -> Result<(), String> {
        while !cancel_token.is_cancelled() {
            if let Some(packet) = sender_queue_rx.recv().await {
                event!(tracing::Level::INFO, "Sending packet: {:?}", packet.header);
                let mut stream_lock = stream.lock().await;
                packet.send(stream_lock.deref_mut()).await?;
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