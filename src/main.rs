use dis_rs::entity_state::model::EntityState;
use dis_rs::enumerations::PduType;
use dis_rs::model::{Location, PduBody};
use futures::SinkExt;
use map_3d::{ecef2geodetic, rad2deg, Ellipsoid};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::watch; // mpsc
use tokio::task::JoinSet; // for websocket.send()

// This will be sent out as a dictionary of EntityStructures.
// { EntityId : EntityStructure }
#[derive(Debug, Serialize, Deserialize)]
struct EntityStructure {
    entity_id: u16,
    force_id: u8,
    location: [f64; 3],
    orientation: [f32; 3],
    orientation_geodetic: [f64; 3],
    angle: f32, // This is custom
    marking: String,
}

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
        let all_pdu = dis_rs::parse(&buf[..len]).unwrap();
        let pdu = all_pdu.get(0).unwrap();
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
        let packet = create_entity_structure_packet(pdu);
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(packet))
            .await?;
    }
}

fn create_entity_structure_packet(pdu: EntityState) -> String {
    let packet = EntityStructure {
        entity_id: pdu.entity_id.entity_id,
        force_id: pdu.force_id.into(),
        location: [
            pdu.entity_location.x_coordinate,
            pdu.entity_location.y_coordinate,
            pdu.entity_location.z_coordinate,
        ],
        orientation_geodetic: get_geodetic_entity_location(pdu.entity_location), // custom
        orientation: [
            pdu.entity_orientation.psi,
            pdu.entity_orientation.theta,
            pdu.entity_orientation.phi,
        ],
        angle: pdu.entity_orientation.psi, // custom
        marking: pdu.entity_marking.marking_string.clone(),
    };
    serde_json::to_string(&packet).expect("Serialization failed")
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

fn get_geodetic_entity_location(loc: Location) -> [f64; 3] {
    let lla = ecef2geodetic(
        loc.x_coordinate,
        loc.y_coordinate,
        loc.z_coordinate,
        Ellipsoid::default(),
    );

    // We need to output lat & lon as degrees
    (rad2deg(lla.0), rad2deg(lla.1), lla.2).into()
}
