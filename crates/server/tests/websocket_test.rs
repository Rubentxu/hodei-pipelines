use futures::StreamExt;
use hodei_adapters::bus::InMemoryBus;
use hodei_core::{JobId, WorkerId};
use hodei_ports::event_bus::{EventPublisher, SystemEvent};
use hodei_server::{bootstrap::ServerComponents, create_api_router};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

#[tokio::test]
async fn test_websocket_status_updates() {
    // 1. Setup Server Components
    let event_bus = Arc::new(InMemoryBus::new(100));
    let components = ServerComponents {
        config: hodei_adapters::config::AppConfig::default(),
        event_subscriber: event_bus.clone(),
        event_publisher: event_bus.clone(),
        status: "ready",
    };

    // 2. Create Router
    let app = create_api_router(components);

    // 3. Start Test Server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 4. Connect WebSocket
    let ws_url = format!("ws://{}/ws", addr);
    let (mut ws_stream, _) = connect_async(ws_url).await.expect("Failed to connect");

    // 5. Publish Event
    let job_id = JobId::from(Uuid::new_v4());
    let event = SystemEvent::JobStarted {
        job_id: job_id.clone(),
        worker_id: WorkerId(Uuid::new_v4()),
    };

    // Give some time for connection to be established and subscription to happen
    tokio::time::sleep(Duration::from_millis(100)).await;

    event_bus
        .publish(event)
        .await
        .expect("Failed to publish event");

    // 6. Verify Event Received
    let timeout = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(msg) = ws_stream.next().await {
            let msg = msg.expect("Error receiving message");
            if let Message::Text(text) = msg {
                if text.contains(&job_id.to_string()) {
                    return Some(text);
                }
            }
        }
        None
    })
    .await;

    assert!(timeout.is_ok(), "Timed out waiting for event");
    let received_text = timeout.unwrap();
    assert!(received_text.is_some(), "Did not receive expected event");

    println!("Received event: {}", received_text.unwrap());
}
