use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch::Receiver;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message::Text;

// for websocket.send()
use futures::SinkExt;

pub struct SubscriberHandler {
    rx: Receiver<String>,
    listener: TcpListener,
}

impl SubscriberHandler {
    pub async fn new(rx: Receiver<String>) -> SubscriberHandler {
        let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
        SubscriberHandler { rx, listener }
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        loop {
            if let Ok((stream, address)) = self.listener.accept().await {
                println!("Subscriber {:?} has connected", address);
                let rx = self.rx.clone();
                tokio::spawn(Self::publish(stream, rx));
            }
        }
    }

    async fn publish(stream: TcpStream, mut rx: Receiver<String>) -> anyhow::Result<()> {
        // Open the websocket for this subscriber
        let mut ws_stream = accept_async(stream).await?;

        // Wait for the rx to change and send data to the subscriber
        loop {
            rx.changed().await?;

            let msg: String = rx.borrow_and_update().clone();

            println!("Sent message to subscriber: {}", msg);

            ws_stream.send(Text(msg)).await?;
        }
    }
}
