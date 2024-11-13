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
