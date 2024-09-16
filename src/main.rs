use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;
use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;

// Messages for communication between threads
#[derive(Debug, Clone)]
enum Message {
    WriteRequest(String, String), // (timestamp, commit_number)
    PrepareRequest(String, String), // (timestamp, commit_number)
    PrepareResponse(bool),
    CommitRequest,
    AbortRequest,
    Ack,
}

// Participant struct
struct Participant {
    id: u32,
    coordinator_tx: mpsc::Sender<Message>,
    buffer: Option<(String, String)>, // (timestamp, commit_number)
    rng: SmallRng,
}

impl Participant {
    async fn run(mut self, mut rx: mpsc::Receiver<Message>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::PrepareRequest(timestamp, commit_number) => {
                    // Simulate some work and random failures
                    sleep(Duration::from_millis(100)).await;
                    let is_ready = self.rng.gen::<f32>() > 0.2;

                    if is_ready {
                        self.buffer = Some((timestamp, commit_number));
                    }

                    self.coordinator_tx.send(Message::PrepareResponse(is_ready)).await.unwrap();
                }
                Message::CommitRequest => {
                    if let Some((timestamp, commit_number)) = self.buffer.take() {
                        let log_entry = format!("{} commit {}\n", timestamp, commit_number);
                        let file_name = format!("participant_{}.txt", self.id);

                        let mut file = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(file_name)
                            .await
                            .unwrap();

                        file.write_all(log_entry.as_bytes()).await.unwrap();
                    }
                    self.coordinator_tx.send(Message::Ack).await.unwrap();
                }
                Message::AbortRequest => {
                    // Clear the buffer in case of abort
                    self.buffer = None;
                    self.coordinator_tx.send(Message::Ack).await.unwrap();
                }
                _ => {}
            }
        }
    }
}

// Coordinator struct
struct Coordinator {
    participants: Vec<mpsc::Sender<Message>>,
    client_tx: mpsc::Sender<Message>,
}

impl Coordinator {
    async fn run(mut self, mut rx: mpsc::Receiver<Message>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::WriteRequest(timestamp, commit_number) => {
                    println!("Coordinator: Received write request: {} at {}", commit_number, timestamp);

                    // Phase 1: Prepare
                    for participant in &self.participants {
                        participant.send(Message::PrepareRequest(timestamp.clone(), commit_number.clone())).await.unwrap();
                    }

                    let mut all_prepared = true;
                    for _ in 0..self.participants.len() {
                        if let Some(Message::PrepareResponse(is_ready)) = rx.recv().await {
                            all_prepared &= is_ready;
                        }
                    }

                    // Phase 2: Commit or Abort
                    let commit_msg = if all_prepared {
                        Message::CommitRequest
                    } else {
                        Message::AbortRequest
                    };

                    for participant in &self.participants {
                        participant.send(commit_msg.clone()).await.unwrap();
                    }

                    // Wait for acknowledgments
                    for _ in 0..self.participants.len() {
                        rx.recv().await;
                    }

                    // Inform the client of the result
                    self.client_tx.send(commit_msg).await.unwrap();
                }
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (client_tx, mut client_rx) = mpsc::channel(32);
    let (coordinator_tx, coordinator_rx) = mpsc::channel(32);

    // Create participants
    let mut participant_channels = vec![];
    for i in 0..2 {
        let (tx, rx) = mpsc::channel(32);
        participant_channels.push(tx.clone());
        let participant = Participant {
            id: i,
            coordinator_tx: coordinator_tx.clone(),
            buffer: None,
            rng: SmallRng::from_entropy(),
        };
        tokio::spawn(async move {
            participant.run(rx).await;
        });
    }

    // Create coordinator
    let coordinator = Coordinator {
        participants: participant_channels,
        client_tx: client_tx.clone(),
    };
    tokio::spawn(async move {
        coordinator.run(coordinator_rx).await;
    });

    // Client thread
    tokio::spawn(async move {
        for i in 1..=5 {
            let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
            let commit_number = i.to_string();
            coordinator_tx.send(Message::WriteRequest(timestamp.clone(), commit_number.clone())).await.unwrap();

            // Wait for the result
            match client_rx.recv().await {
                Some(Message::CommitRequest) => println!("Client: Transaction {} committed", i),
                Some(Message::AbortRequest) => println!("Client: Transaction {} aborted", i),
                _ => {}
            }

            sleep(Duration::from_secs(1)).await;
        }
    });

    // Let the simulation run for a while
    sleep(Duration::from_secs(10)).await;
}