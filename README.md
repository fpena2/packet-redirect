# Approach

This is a simplified version of this program.


```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // Create a new Tokio MPSC channel with a buffer capacity of 10.
    let (tx, mut rx) = mpsc::channel(10);

    // Spawn a new task that sends values to the channel.
    let sender_task = tokio::spawn(async move {
        for i in 0..10 {
            tx.send(i).await.unwrap();
        }
    });

    // Receive values from the channel in the main task.
    while let Some(received) = rx.recv().await {
        println!("Received value: {}", received);
    }

    // Wait for the sender task to finish.
    sender_task.await.unwrap();
}

```

```rust
// #[tokio::main]
// async fn main() -> io::Result<()> {
//     let socket = UdpSocket::bind("0.0.0.0:9000").await?;
//     loop {
//         let mut buf = [0; 1024];
//         let (len, addr) = socket.recv_from(&mut buf).await?;
//         println!("received {:?} bytes from {:?}", len, addr);
//     }
// }
```