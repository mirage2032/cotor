use arti_client::{TorClient, TorClientConfig};
use futures_util::{Stream, StreamExt};
use std::error::Error;
use std::sync::Arc;
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::config::OnionServiceConfigBuilder;
use tor_hsservice::{RendRequest, RunningOnionService};
use tor_rtcompat::PreferredRuntime;

pub struct ConnectionAcceptor {
    stop_tx: tokio::sync::oneshot::Sender<()>,
    task_handle: tokio::task::JoinHandle<()>,
}

impl ConnectionAcceptor {
    pub fn new(rend_request: impl Stream<Item = RendRequest> + Unpin + Send + 'static) -> Self {
        let mut stream_request = tor_hsservice::handle_rend_requests(rend_request);
        let (stop_tx,mut stop_rx) = tokio::sync::oneshot::channel();
        let task_handle = tokio::spawn(async move {
            tokio::select! {
                _ = &mut stop_rx => {
                    println!("Connection acceptor stop signal received.");
                    return;
                }
                maybe_request = stream_request.next() => {
                    match maybe_request {
                        Some(stream_req) => {
                            println!("Received rend request: {:?}", stream_req);
                            //accept connection
                            match stream_req.accept(Connected::new_empty()).await{
                                Ok(tor_stream) => {
                                },
                                Err(e) => eprintln!("Failed to accept connection: {}", e),
                            }
                        }
                        None => return, // Stream ended
                    }
                }
            }
        });
        ConnectionAcceptor {
            stop_tx,
            task_handle,
        }
    }
}

pub struct COTORServer {
    tor_client: TorClient<PreferredRuntime>,
    service: Option<Arc<RunningOnionService>>,
    acceptor: Option<ConnectionAcceptor>,
}

impl COTORServer {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let cfg = TorClientConfig::default();
        let tor_client = TorClient::create_bootstrapped(cfg).await?;
        Ok(COTORServer {
            tor_client,
            service: None,
            acceptor: None,
        })
    }

    async fn start_service(
        &mut self,
    ) -> Result<impl Stream<Item = RendRequest> + Send + 'static, Box<dyn Error>>
    {
        if self.service.is_some() {
            return Err("Service is already running".into());
        }
        let (onion_service, rend_requests) = self.tor_client.launch_onion_service(
            OnionServiceConfigBuilder::default()
                .nickname("my-service".to_owned().try_into().unwrap())
                .build()?,
        )?;
        if let Some(onion_address) = onion_service.onion_address() {
            println!("Onion service launched with address: {}", onion_address);
        } else {
            return Err("Failed to get onion address".into());
        }
        self.service = Some(onion_service);
        Ok(rend_requests)
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Starting COTOR (Control Over Tor) Service...");
        let rend_requests = self.start_service().await?;
        println!("Starting accepting connections...");
        self.acceptor = Some(ConnectionAcceptor::new(rend_requests));
        println!("Waiting for connections...");
        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.stop_tx.send(());
            let _ = acceptor.task_handle.await;
            println!("Connection acceptor stopped.");
        }
        self.service = None;
        println!("COTOR Service stopped.");
    }
}


