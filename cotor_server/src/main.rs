use tracing::event;
use tracing_subscriber::filter::dynamic_filter_fn;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod server;
mod clientconn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut server = server::COTORServer::new().await?;
    server.start().await?;
    server.stop().await;
    Ok(())
}
