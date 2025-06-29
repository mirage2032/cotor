use arti_client::DataStream;
use cotor_core::prelude::uuid::Uuid;
use tokio_util::sync::CancellationToken;

pub struct ClientConnection{
    uuid: Uuid,
    stream: DataStream,
    cancel_token: CancellationToken
}

impl ClientConnection {
    pub fn new(stream: DataStream) -> Self {
        let uuid = Uuid::new_v4();
        let cancel_token = CancellationToken::new();
        Self { uuid, stream, cancel_token }
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    async fn packet_receiver(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tokio::select! {
            _ = self.cancel_token.cancelled() => {
                println!("Packet receiver cancelled for connection: {}", self.uuid);
                Ok(())
            }
            result = self.stream.rea => {
                match result {
                    Some(Ok(packet)) => {
                        // Process the packet
                        println!("Received packet: {:?}", packet);
                        Ok(())
                    },
                    Some(Err(e)) => {
                        eprintln!("Error receiving packet: {}", e);
                        Err(Box::new(e))
                    },
                    None => {
                        println!("Stream ended for connection: {}", self.uuid);
                        Ok(())
                    }
                }
            }
        }
    }

    pub async fn handle(&mut self) {

    }
}