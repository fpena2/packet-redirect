use dis_rs::entity_state::model::EntityState;
use dis_rs::enumerations::PduType;
use dis_rs::model::PduBody;
use futures::SinkExt;
use tokio::net::UdpSocket;
use tokio::sync::watch; // mpsc
use tokio::task::JoinSet; // for websocket.send()

#[tokio::main]
async fn main() {
    let (tx, rx) = watch::channel(EntityState::default());

    let mut tasks = JoinSet::new();
    tasks.spawn(receive_packets(tx));
    tasks.spawn(service_clients(rx));
    loop {}
}

async fn receive_packets(tx: watch::Sender<EntityState>) {
    let socket = UdpSocket::bind("0.0.0.0:9000").await.unwrap();
    loop {
        // Receive packets
        let mut buf = [0; 1024];
        let (len, _addr) = socket.recv_from(&mut buf).await.unwrap();

        // Process the PDU
        let pdus = dis_rs::parse(&buf[..len]).unwrap();
        let pdu = pdus.get(0).unwrap();
        if pdu.header.pdu_type == PduType::EntityState {
            if let PduBody::EntityState(entity_state) = &pdu.body {
                tx.send(entity_state.clone()).unwrap();
            }
        }
    }
}

async fn service_clients(rx: watch::Receiver<EntityState>) {
    let socket = tokio::net::TcpListener::bind("0.0.0.0:9001").await;
    let listener = socket.expect("Failed to bind");
    while let Ok((stream, address)) = listener.accept().await {
        tokio::spawn(handle_client_connection(stream, address, rx.clone()));
    }
}

async fn handle_client_connection(
    stream: tokio::net::TcpStream,
    address: std::net::SocketAddr,
    mut rx: watch::Receiver<EntityState>,
) -> tokio_tungstenite::tungstenite::Result<()> {
    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Failed to accept");

    print_new_connection(&address);

    loop {
        // Wait for the rx to change
        rx.changed().await.unwrap();

        // Send data to client
        let pdu = rx.borrow_and_update().clone();
        let received_string = pdu.entity_marking.marking_string;
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                received_string,
            ))
            .await?;
    }
}

fn print_new_connection(address: &std::net::SocketAddr) {
    let current_time = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.3f")
        .to_string();
    println!(
        "New WebSocket connection at {}: {:?}",
        current_time, address
    );
}
