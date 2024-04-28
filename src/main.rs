use dis_rs::entity_state::model::EntityState;
use dis_rs::enumerations::PduType;
use dis_rs::model::{Location, Orientation, PduBody, VectorF32};
use futures::SinkExt;
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use tokio::net::UdpSocket;
use tokio::sync::watch; // mpsc
use tokio::task::JoinSet; // for websocket.send()

// This will be sent out as a dictionary of EntityStructures.
// { EntityId : EntityStructure }
#[derive(Debug, Serialize, Deserialize)]
struct EntityStructure {
    marking: String,
    position: [f64; 3],
    angle: f32,
    id: u16,
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
        let packet = create_entity_structure_packet(pdu);
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(packet))
            .await?;
    }
}

fn create_entity_structure_packet(pdu: EntityState) -> String {
    let packet = EntityStructure {
        marking: pdu.entity_marking.marking_string.clone(),
        position: [
            pdu.entity_location.x_coordinate,
            pdu.entity_location.y_coordinate,
            pdu.entity_location.z_coordinate,
        ],
        angle: calculate_angle(
            pdu.entity_location,
            pdu.entity_orientation,
            pdu.entity_linear_velocity,
        ),
        id: pdu.entity_id.entity_id,
    };
    serde_json::to_string(&packet).expect("Serialization failed")
}

fn calculate_angle(_location: Location, orientation: Orientation, _velocity: VectorF32) -> f32 {
    // let loc = Vector3::new(
    //     location.x_coordinate,
    //     location.y_coordinate,
    //     location.z_coordinate,
    // );
    // let euler = Vector3::new(orientation.phi, orientation.phi, orientation.phi);
    // let vel = Vector3::new(
    //     velocity.first_vector_component as f64,
    //     velocity.second_vector_component as f64,
    //     velocity.third_vector_component as f64,
    // );
    //
    // let linear_velocity = vel.cross(&loc);
    // let linear_velocity_magnitude = linear_velocity.magnitude();
    //
    // // Calculate angle in xy-plane
    // let theta = linear_velocity.y.atan2(linear_velocity.x);
    // // Calculate angle with respect to z-axis
    // let phi = linear_velocity.z.acos() / PI;
    orientation.psi
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
