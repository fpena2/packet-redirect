use tokio::sync::watch;
use tokio::try_join; // mpsc

mod producers;
mod subscribers;
use producers::ProducerHandler;
use subscribers::SubscriberHandler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, rx) = watch::channel(String::new());

    let producers = ProducerHandler::new(tx).await;
    let subscribers = SubscriberHandler::new(rx).await;

    // monitor and publish concurrently
    try_join!(producers.monitor(), subscribers.serve())?;
    Ok(())
}
