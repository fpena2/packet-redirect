use dis_rs::entity_state::model::{EntityAppearance, EntityState};
use dis_rs::enumerations::{AppearanceEntityorObjectState, EntityKind, PduType, PlatformDomain};
use dis_rs::model::{Location, Orientation, PduBody};
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
    orientation_euler: [f64; 3],
    location_geodetic: [f64; 3],
    state: u8,
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
                if entity_state.entity_type.kind == EntityKind::Platform
                    && entity_state.entity_type.domain == PlatformDomain::Air
                {
                    tx.send(entity_state.clone()).unwrap();
                }
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
    let entity_appearance = match pdu.entity_appearance {
        EntityAppearance::AirPlatform(ref air_platform_appearance) => air_platform_appearance.state,
        _ => AppearanceEntityorObjectState::Deactivated,
    };
    let packet = EntityStructure {
        entity_id: pdu.entity_id.entity_id,
        force_id: pdu.force_id.into(),
        location_geodetic: get_geodetic_entity_location(pdu.entity_location),
        orientation_euler: get_euler_entity_orientation(pdu.entity_orientation),
        marking: pdu.entity_marking.marking_string.clone(),
        state: entity_appearance.into(), // custom
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

fn get_geodetic_entity_location(location: Location) -> [f64; 3] {
    let lla = ecef2geodetic(
        location.x_coordinate,
        location.y_coordinate,
        location.z_coordinate,
        Ellipsoid::default(),
    );
    [rad2deg(lla.0), rad2deg(lla.1), lla.2]
}

fn get_euler_entity_orientation(orientation: Orientation) -> [f64; 3] {
    [
        rad2deg(orientation.psi as f64),
        rad2deg(orientation.theta as f64),
        rad2deg(orientation.phi as f64),
    ]
}
