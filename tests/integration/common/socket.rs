//! A websocket client speaking the Pusher protocol to the server under
//! test, the way the desktop clients connect: `/app/<key>?protocol=7&...`.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// How long a test waits for a frame it expects.
pub const FRAME_TIMEOUT: Duration = Duration::from_secs(5);

pub struct PusherClient {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    pub socket_id: Option<String>,
}

/// What the server did next.
#[derive(Debug, PartialEq)]
pub enum Next {
    Text(String),
    /// A close frame with its code and reason.
    Closed(Option<(u16, String)>),
    /// The connection ended without a close frame.
    Dropped,
    Silent,
}

impl PusherClient {
    /// Connects to `/app/<key>` with the query the v0.5 client sends.
    pub async fn connect(address: SocketAddr, key: &str) -> Self {
        Self::connect_path(
            address,
            &format!("/app/{key}?protocol=7&client=telescope-rust&version=0.5.1&flash=false"),
        )
        .await
    }

    pub async fn connect_path(address: SocketAddr, path: &str) -> Self {
        let url = format!("ws://{address}{path}");
        let (socket, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("the websocket connects");
        Self {
            socket,
            socket_id: None,
        }
    }

    /// Connects and reads `pusher:connection_established`, keeping the
    /// socket id.
    pub async fn established(address: SocketAddr, key: &str) -> (Self, String) {
        let mut client = Self::connect(address, key).await;
        let frame = client.text().await;
        let parsed: Value = serde_json::from_str(&frame).expect("a JSON frame");
        assert_eq!(parsed["event"], "pusher:connection_established", "{frame}");
        let data: Value =
            serde_json::from_str(parsed["data"].as_str().expect("string data")).expect("JSON data");
        let socket_id = data["socket_id"].as_str().expect("a socket id").to_owned();
        client.socket_id = Some(socket_id.clone());
        (client, frame)
    }

    pub async fn send(&mut self, text: &str) {
        self.socket
            .send(Message::text(text.to_owned()))
            .await
            .expect("the frame sends");
    }

    pub async fn send_json(&mut self, value: &Value) {
        self.send(&value.to_string()).await;
    }

    /// What comes next within `wait`.
    pub async fn next_within(&mut self, wait: Duration) -> Next {
        loop {
            match tokio::time::timeout(wait, self.socket.next()).await {
                Err(_) => return Next::Silent,
                Ok(None) => return Next::Dropped,
                Ok(Some(Err(_))) => return Next::Dropped,
                Ok(Some(Ok(Message::Text(text)))) => return Next::Text(text.to_string()),
                Ok(Some(Ok(Message::Close(frame)))) => {
                    return Next::Closed(
                        frame.map(|frame| (u16::from(frame.code), frame.reason.to_string())),
                    );
                }
                // Protocol pings and pongs are not Pusher frames.
                Ok(Some(Ok(_))) => continue,
            }
        }
    }

    /// The next text frame.
    pub async fn text(&mut self) -> String {
        match self.next_within(FRAME_TIMEOUT).await {
            Next::Text(text) => text,
            other => panic!("expected a frame, got {other:?}"),
        }
    }

    /// Nothing arrives within `wait`.
    pub async fn assert_silent(&mut self, wait: Duration) {
        let next = self.next_within(wait).await;
        assert_eq!(next, Next::Silent, "expected silence");
    }

    pub async fn close(mut self) {
        self.socket.close(None).await.ok();
    }
}
