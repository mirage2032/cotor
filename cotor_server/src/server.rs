use crate::clientconn::ClientConnection;
use arti_client::{TorClient, TorClientConfig};
use futures_util::{Stream, StreamExt};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::config::OnionServiceConfigBuilder;
use tor_hsservice::{RendRequest, RunningOnionService};
use tor_rtcompat::PreferredRuntime;
use tracing::{Instrument, event, info_span, instrument};
use uuid::Uuid;

pub struct COTORServer {
    tor_client: TorClient<PreferredRuntime>,
    service: Option<Arc<RunningOnionService>>,
    cancel_token: CancellationToken,
    acceptor_handle: Option<tokio::task::JoinHandle<()>>,
    client_connections: Arc<Mutex<HashMap<Uuid, ClientConnection>>>,
}

impl COTORServer {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let cfg = TorClientConfig::default();
        let tor_client = TorClient::create_bootstrapped(cfg).await?;
        Ok(COTORServer {
            tor_client,
            service: None,
            cancel_token: CancellationToken::new(),
            acceptor_handle: None,
            client_connections: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn start_service(
        &mut self,
    ) -> Result<impl Stream<Item = RendRequest> + Send + 'static, Box<dyn Error>> {
        if self.service.is_some() {
            return Err("Service is already running".into());
        }
        let (onion_service, rend_requests) = self.tor_client.launch_onion_service(
            OnionServiceConfigBuilder::default()
                .nickname("my-service".to_owned().try_into().unwrap())
                .build()?,
        )?;
        if let Some(onion_address) = onion_service.onion_address() {
            event!(
                tracing::Level::INFO,
                "Onion service launched with address: {}",
                onion_address
            );
        } else {
            return Err("Failed to get onion address".into());
        }
        self.service = Some(onion_service);
        Ok(rend_requests)
    }

    async fn start_acceptor(
        &mut self,
        rend_request: impl Stream<Item = RendRequest> + Send + Unpin + 'static,
        connected_clients: Arc<Mutex<HashMap<Uuid, ClientConnection>>>,
    ) {
        let mut stream_request = tor_hsservice::handle_rend_requests(rend_request);
        let cancel_token = self.cancel_token.clone();
        //spawn task to handle rend requests and stop on self.cancel_token.cancelled();
        self.acceptor_handle = Some(tokio::spawn(async move {
            loop {
                tokio::select! {
                _ = cancel_token.cancelled() => {
                    event!(tracing::Level::INFO, "Acceptor received cancel signal, stopping...");
                    break;
                }
                maybe_request = stream_request.next() => {
                    match maybe_request {
                        Some(request) => {
                                match request.accept(Connected::new_empty()).await{
                                    Ok(stream) => {
                                        let connected_clients_clone = connected_clients.clone();
                                        let client_conn = ClientConnection::new(stream, cancel_token.child_token(),
                                            move |id| {
                                                let id = *id;
                                                let connected_clients_clone = connected_clients_clone.clone();
                                                Box::pin(async move {
                                                    if let Some(conn) = connected_clients_clone.lock().await.get_mut(&id){
                                                        conn.close().await;
                                                    }
                                                    connected_clients_clone.lock().await.remove(&id);
                                                    event!(tracing::Level::INFO, "Removed client");
                                                })
                                            }
                                        );
                                        match client_conn {
                                            Ok(client_conn) => {
                                                event!(tracing::Level::INFO, "Client connection established: {:?}", client_conn.uuid());
                                                connected_clients.lock().await.insert(client_conn.uuid(), client_conn);
                                            }
                                            Err(e) => {
                                                event!(tracing::Level::WARN, "Failed to create client connection: {}", e);
                                                continue;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        event!(tracing::Level::WARN, "Failed to connect to client: {}", e);
                                    }
                                }
                            }
                        None => {
                            event!(tracing::Level::INFO, "No more rend requests, stopping acceptor...");
                            break;
                            }
                        }
                    }
                }
            }
        }.instrument(info_span!("acceptor_task"))));
    }

    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        event!(tracing::Level::INFO, "Starting COTOR Server...");
        let rend_requests = self.start_service().await?;
        event!(tracing::Level::INFO, "Starting accepting connections...");
        self.start_acceptor(rend_requests, self.client_connections.clone())
            .await;
        event!(tracing::Level::INFO, "Waiting for connections...");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn stop(&mut self) {
        self.cancel_token.cancel();
        if let Some(handle) = self.acceptor_handle.take()
            && let Err(e) = handle.await
        {
            event!(
                tracing::Level::ERROR,
                "Error while stopping acceptor: {}",
                e
            );
        }
        for (_, client) in self.client_connections.lock().await.iter_mut() {
            client.close().await;
        }
        event!(tracing::Level::INFO, "COTOR Server stopped.");
    }
}

impl Drop for COTORServer {
    fn drop(&mut self) {
        self.cancel_token.cancel();
        if let Some(handle) = self.acceptor_handle.take() {
            handle.abort();
        }
    }
}
