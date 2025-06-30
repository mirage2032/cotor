use cotor_core::network::crypt::KeyChain;
use cotor_core::network::crypt::aes::AESKey;
use cotor_core::network::crypt::rsa::RSAPrivateKey;
use cotor_core::network::packet::message::MessageData;
use cotor_core::network::packet::{NetworkPacket, PacketEncryption};
use tracing_subscriber::filter::dynamic_filter_fn;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod clientconn;
mod server;
mod handlers;

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

    let mut key_chain = KeyChain::default();
    key_chain.aes_key = Some(AESKey::new()?);
    let rsa_private_key = RSAPrivateKey::new()?;
    let rsa_public_key = rsa_private_key.public_key();
    key_chain.rsa_private_key = Some(rsa_private_key);
    key_chain.rsa_public_key = Some(rsa_public_key);

    let message = MessageData::new_debug("Test message".to_string());
    let network_packet = NetworkPacket::new(&message, PacketEncryption::RSA, &key_chain)?;
    let mut stream = std::io::Cursor::new(Vec::new());
    network_packet.send(&mut stream).await?;
    stream.set_position(0); // Reset the cursor to the beginning of the stream
    let received_packet = NetworkPacket::from_stream(&mut stream).await?;
    let received_packet_data = received_packet.decrypt(&key_chain)?;
    println!("Received packet data: {:?}", received_packet_data);
    // let mut server = server::COTORServer::new().await?;
    // server.start().await?;
    // server.stop().await;
    Ok(())
}
