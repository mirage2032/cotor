use tokio::io;
use tokio::io::{AsyncBufReadExt, BufReader};
use cotor_core::network::crypt::KeyChain;
use cotor_core::network::crypt::aes::AESKey;
use cotor_core::network::crypt::rsa::RSAPrivateKey;
use cotor_core::network::packet::aes::AESPacketData;
use cotor_core::network::packet::message::MessageData;
use cotor_core::network::packet::rsa::RSAPacketData;
use cotor_core::network::packet::{NetworkPacket, PacketEncryption};
use tracing_subscriber::filter::dynamic_filter_fn;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod clientconn;
mod handlers;
mod server;

const ROOT_SPAN_NAME: &str = "cotor_server";

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                // print the span name in your log prefix
                .with_target(false)
                .with_thread_names(false)
                .with_span_events(fmt::format::FmtSpan::CLOSE),
        )
        .with(dynamic_filter_fn(move |meta, ctx| {
            // If *this* metadata *is* the root span itself, show it
            if meta.is_span() && meta.name() == ROOT_SPAN_NAME {
                return true;
            }
            // Otherwise, walk up the active span stack to see if we're inside it
            let mut cur = ctx.lookup_current();
            while let Some(sr) = cur {
                if sr.name() == ROOT_SPAN_NAME {
                    return true;
                }
                cur = sr.parent();
            }
            false
        }))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let span = tracing::info_span!(ROOT_SPAN_NAME);
    let _enter = span.enter();

    // let key_chain = KeyChain::new().expect("Failed to create key chain");
    //
    // let message = MessageData::new_debug("Test message".to_string());
    // let aes_packet = AESPacketData::AESKey(*key_chain.aes_key.as_ref().expect("AES key not set"));
    //
    // let network_packet = NetworkPacket::new(&aes_packet, &PacketEncryption::RSA, &key_chain)?;
    // let mut stream = std::io::Cursor::new(Vec::new());
    // network_packet.send(&mut stream).await?;
    // stream.set_position(0); // Reset the cursor to the beginning of the stream
    // let received_packet = NetworkPacket::from_stream(&mut stream).await?;
    // let received_packet_data = received_packet.decrypt(&key_chain)?;
    // println!("Received packet data: {received_packet_data:?}");
    let mut server = server::COTORServer::new().await?;
    server.start().await?;
    //async wait for keyboard new line
    println!("Server is running. Press Enter to stop...");
    let mut input = String::new();
    let mut reader = BufReader::new(io::stdin());
    while input != "stop\n" {
        input.clear();
        reader.read_line(&mut input).await?;
    }
    tracing::info!("Stopping server...");
    server.stop().await;
    Ok(())
}
