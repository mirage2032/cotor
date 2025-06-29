use cotor_core::network::packet::NetworkPacket;
use cotor_core::network::packet::AnyPacketData;
use cotor_core::network::packet::PacketData;
use tracing::event;
use tracing_subscriber::filter::dynamic_filter_fn;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use cotor_core::network::packet::message::MessageData;
use cotor_core::network::crypt::aes::AESKey;
use cotor_core::network::crypt::KeyChain;

mod server;
mod clientconn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let _ = std::any::TypeId::of::<MessageData>();
    //configure tracing to show all logs of all levels. Allow only coto
    const ROOT_SPAN_NAME: &str = "cotor_server";
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                // print the span name in your log prefix
                .with_target(false)
                .with_thread_names(false)
                .with_span_events(fmt::format::FmtSpan::CLOSE)
        )
        .with(
            dynamic_filter_fn(move |meta, ctx| {
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
            })
        )
        .init();
    let span = tracing::info_span!(ROOT_SPAN_NAME);
    let _enter = span.enter();

    let aes_key = AESKey::new()?;
    let mut key_chain = KeyChain::default();
    key_chain.aes_key = Some(aes_key.clone());

    let message:Box<dyn AnyPacketData> = Box::new(MessageData::new_debug("Test message".to_string()));
    let packet = PacketData::from(message);
    let network_packet = packet.plain_encode()?;
    // let received_packet_data = PacketData::bin_deserialize(&network_packet.data)?;
    //create a stream to write to and then be able to read from
    let mut stream = std::io::Cursor::new(Vec::new());
    network_packet.send(&mut stream).await?;
    stream.set_position(0); // Reset the cursor to the beginning of the stream
    let received_packet = NetworkPacket::from_stream(&mut stream).await?;
    let received_packet_data = PacketData::bin_deserialize(received_packet.data.as_slice())?;
    println!("Received packet data: {:?}", received_packet_data);
    // let mut server = server::COTORServer::new().await?;
    // server.start().await?;
    // server.stop().await;
    Ok(())
}
