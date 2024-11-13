use tokio::net::UdpSocket;
use tokio::sync::watch::Sender;

pub struct ProducerHandler {
    tx: Sender<String>,
    socket: UdpSocket,
}

impl ProducerHandler {
    pub async fn new(tx: Sender<String>) -> ProducerHandler {
        let socket = UdpSocket::bind("127.0.0.1:3000").await.unwrap();
        ProducerHandler { tx, socket }
    }

    pub async fn monitor(&self) -> anyhow::Result<()> {
        loop {
            let mut buf = [0; 1024];
            let (len, _addr) = self.socket.recv_from(&mut buf).await?;
            let msg = String::from_utf8(buf[..len].to_vec())?;
            self.tx.send(msg)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::watch;

    #[tokio::test]
    async fn test_producer_handler() {
        let (tx, mut rx) = watch::channel(String::new());

        // Init ProducerHandler
        let handler = ProducerHandler::new(tx).await;
        let monitor_task = tokio::spawn(async move {
            handler.monitor().await.unwrap();
        });

        // Send a test message
        let test_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let test_message = "Hello, World!";
        test_socket
            .send_to(test_message.as_bytes(), "127.0.0.1:3000")
            .await
            .unwrap();

        // Check if the message was distributed to the channel receivers
        rx.changed().await.unwrap();
        let received_message: String = rx.borrow_and_update().clone();
        assert_eq!(received_message, test_message);

        monitor_task.abort();
    }
}
