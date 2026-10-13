//! The websocket at `/app/<key>`, speaking the Pusher protocol 7:
//!
//! - On connect `pusher:connection_established` with a socket id (two
//!   random numbers) and the activity timeout; an unknown key gets error
//!   4001 and the connection is dropped.
//! - `pusher:ping` is answered `{"event":"pusher:pong"}`; `pusher:subscribe`
//!   checks a private channel's signature (error 4009 when it does not
//!   match), takes a public channel as it is and refuses presence
//!   channels; `pusher:unsubscribe` is silent.
//! - A message that is not JSON with an `event`, an unknown `pusher:`
//!   event and a malformed client event get error 4200; a client event
//!   from a socket not on its channel gets 4009 and one from a member is
//!   not relayed. A message over the size limit closes the
//!   connection with 1009.
//! - A subscribed connection silent for the ping interval is pinged on the
//!   next tick, and dropped with error 4201 if it is still silent at the
//!   tick after; one that never subscribed is left alone.

use std::collections::HashSet;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use rand::RngExt;
use serde_json::{Value, json};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;

use super::auth::{channel_auth, sign};
use crate::config::PusherConfig;
use crate::server::AppState;

/// The largest frame the websocket layer reads; the protocol's own limit
/// (`PUSHER_MAX_MESSAGE_SIZE`) is checked on top.
const READ_LIMIT: usize = 1024 * 1024;

pub async fn upgrade(
    State(state): State<AppState>,
    Path(key): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade
        .max_message_size(READ_LIMIT)
        .max_frame_size(READ_LIMIT)
        .on_upgrade(move |socket| async move {
            if key == state.config.pusher.key {
                Connection::new(socket, &state).run().await;
            } else {
                reject(socket).await;
            }
        })
}

/// An unknown app key: the error, then the connection is dropped.
async fn reject(mut socket: WebSocket) {
    let frame = error_frame(4001, "Application does not exist");
    let _ = socket.send(Message::Text(frame.into())).await;
}

/// `pusher:error` with its JSON string data.
fn error_frame(code: u16, message: &str) -> String {
    json!({
        "event": "pusher:error",
        "data": json!({ "code": code, "message": message }).to_string(),
    })
    .to_string()
}

/// A socket id: two random numbers up to a billion.
fn new_socket_id() -> String {
    let mut random = rand::rng();
    format!(
        "{}.{}",
        random.random_range(1..=1_000_000_000u32),
        random.random_range(1..=1_000_000_000u32)
    )
}

struct Connection<'a> {
    socket: WebSocket,
    state: &'a AppState,
    config: &'a PusherConfig,
    socket_id: String,
    channels: HashSet<String>,
    last_seen: Instant,
    pinged: bool,
    /// The client answered with websocket pings or pongs: the server
    /// pings it the same way.
    control_frames: bool,
}

/// How a message ends the connection, if it does.
enum Outcome {
    Continue,
    Close(CloseFrame),
    Drop,
}

impl<'a> Connection<'a> {
    fn new(socket: WebSocket, state: &'a AppState) -> Self {
        Self {
            socket,
            state,
            config: &state.config.pusher,
            socket_id: new_socket_id(),
            channels: HashSet::new(),
            last_seen: Instant::now(),
            pinged: false,
            control_frames: false,
        }
    }

    async fn send(&mut self, frame: String) -> bool {
        self.socket.send(Message::Text(frame.into())).await.is_ok()
    }

    fn touch(&mut self) {
        self.last_seen = Instant::now();
        self.pinged = false;
    }

    fn inactive(&self) -> bool {
        self.last_seen.elapsed() >= self.config.ping_interval
    }

    async fn run(mut self) {
        let established = json!({
            "event": "pusher:connection_established",
            "data": json!({
                "socket_id": self.socket_id,
                "activity_timeout": self.config.activity_timeout,
            })
            .to_string(),
        })
        .to_string();
        if !self.send(established).await {
            return;
        }
        let mut bus = self.state.pusher.subscribe();
        let mut ticks = tokio::time::interval_at(
            Instant::now() + self.config.ping_interval,
            self.config.ping_interval.max(Duration::from_millis(10)),
        );
        loop {
            tokio::select! {
                message = self.socket.recv() => {
                    let Some(Ok(message)) = message else { return };
                    match self.receive(message).await {
                        Outcome::Continue => {}
                        Outcome::Close(frame) => {
                            let _ = self.socket.send(Message::Close(Some(frame))).await;
                            return;
                        }
                        Outcome::Drop => return,
                    }
                }
                published = bus.recv() => match published {
                    Ok(published) => {
                        if self.channels.contains(&published.channel)
                            && !self.send(published.frame.clone()).await
                        {
                            return;
                        }
                    }
                    Err(RecvError::Lagged(missed)) => {
                        tracing::warn!(missed, "a slow websocket missed events");
                    }
                    Err(RecvError::Closed) => return,
                },
                _ = ticks.tick() => {
                    // Only connections on a channel are watched.
                    if self.channels.is_empty() {
                        continue;
                    }
                    if self.pinged && self.inactive() {
                        let _ = self.send(error_frame(4201, "Pong reply not received in time")).await;
                        return;
                    }
                    if self.inactive() {
                        let sent = if self.control_frames {
                            self.socket.send(Message::Ping(Vec::new().into())).await.is_ok()
                        } else {
                            self.send(r#"{"event":"pusher:ping"}"#.to_owned()).await
                        };
                        if !sent {
                            return;
                        }
                        self.pinged = true;
                    }
                }
            }
        }
    }

    async fn receive(&mut self, message: Message) -> Outcome {
        let text = match message {
            Message::Text(text) => text,
            Message::Binary(bytes) => Utf8Bytes::from(String::from_utf8_lossy(&bytes).into_owned()),
            Message::Ping(_) | Message::Pong(_) => {
                self.control_frames = true;
                self.touch();
                return Outcome::Continue;
            }
            Message::Close(_) => return Outcome::Drop,
        };
        self.touch();
        if text.len() > self.config.max_message_size {
            return Outcome::Close(CloseFrame {
                code: 1009,
                reason: "Maximum message size exceeded".into(),
            });
        }
        let reply = match self.handle(text.as_str()) {
            Ok(reply) => reply,
            Err(Invalid) => Some(error_frame(4200, "Invalid message format")),
        };
        if let Some(reply) = reply
            && !self.send(reply).await
        {
            return Outcome::Drop;
        }
        Outcome::Continue
    }

    /// One message; the frame to answer, if any. `Err` is "Invalid
    /// message format".
    fn handle(&mut self, text: &str) -> Result<Option<String>, Invalid> {
        let mut message: Value = serde_json::from_str(text).map_err(|_| Invalid)?;
        let object = message.as_object_mut().ok_or(Invalid)?;
        // `data` may arrive as a JSON string.
        if let Some(Value::String(data)) = object.get("data")
            && let Ok(parsed @ (Value::Object(_) | Value::Array(_))) =
                serde_json::from_str::<Value>(data)
        {
            object.insert("data".to_owned(), parsed);
        }
        let event = object
            .get("event")
            .and_then(Value::as_str)
            .ok_or(Invalid)?
            .to_owned();
        let empty = Value::Object(Default::default());
        let data = match object.get("data") {
            None | Some(Value::Null) => &empty,
            Some(Value::String(text)) if text.is_empty() => &empty,
            Some(data) => data,
        };
        match event.strip_prefix("pusher:") {
            Some("ping") => Ok(Some(r#"{"event":"pusher:pong"}"#.to_owned())),
            Some("pong") => Ok(None),
            Some("connection_established") => Ok(Some(
                json!({
                    "event": "pusher:connection_established",
                    "data": json!({
                        "socket_id": self.socket_id,
                        "activity_timeout": self.config.activity_timeout,
                    })
                    .to_string(),
                })
                .to_string(),
            )),
            Some("subscribe") => self.subscribe(data),
            Some("unsubscribe") => {
                let channel = data.get("channel").and_then(Value::as_str).ok_or(Invalid)?;
                self.channels.remove(channel);
                Ok(None)
            }
            Some(_) => Err(Invalid),
            None => self.client_event(&event, object),
        }
    }

    fn subscribe(&mut self, data: &Value) -> Result<Option<String>, Invalid> {
        let channel = data.get("channel").and_then(Value::as_str).ok_or(Invalid)?;
        let auth = optional_string(data.get("auth"))?;
        let channel_data = optional_string(data.get("channel_data"))?;
        if channel_data.is_some_and(|text| serde_json::from_str::<Value>(text).is_err()) {
            return Err(Invalid);
        }
        let unauthorized = || Ok(Some(error_frame(4009, "Connection is unauthorized")));
        if channel.starts_with("presence-") {
            return unauthorized();
        }
        if channel.starts_with("private-") {
            let expected = match channel_data {
                Some(channel_data) => format!(
                    "{}:{}",
                    self.config.key,
                    sign(
                        &self.config.secret,
                        &format!("{}:{channel}:{channel_data}", self.socket_id)
                    )
                ),
                None => channel_auth(
                    &self.config.key,
                    &self.config.secret,
                    &self.socket_id,
                    channel,
                ),
            };
            if !auth.is_some_and(|auth| constant_time_eq(auth, &expected)) {
                return unauthorized();
            }
        }
        self.channels.insert(channel.to_owned());
        Ok(Some(
            json!({
                "event": "pusher_internal:subscription_succeeded",
                "data": "{}",
                "channel": channel,
            })
            .to_string(),
        ))
    }

    /// An event without the `pusher:` prefix: it needs a channel; a
    /// `client-` event from a socket on that channel is dropped, from one
    /// that is not answered with 4009; anything else is ignored.
    fn client_event(
        &self,
        event: &str,
        message: &serde_json::Map<String, Value>,
    ) -> Result<Option<String>, Invalid> {
        let channel = message
            .get("channel")
            .and_then(Value::as_str)
            .ok_or(Invalid)?;
        if !matches!(
            message.get("data"),
            None | Some(Value::Null | Value::Object(_) | Value::Array(_))
        ) {
            return Err(Invalid);
        }
        if !event.starts_with("client-") || self.channels.contains(channel) {
            return Ok(None);
        }
        Ok(Some(error_frame(
            4009,
            "The client is not a member of the specified channel.",
        )))
    }
}

struct Invalid;

/// A field that must be a string when it is there.
fn optional_string(value: Option<&Value>) -> Result<Option<&str>, Invalid> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text)),
        Some(_) => Err(Invalid),
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |difference, (x, y)| difference | (x ^ y))
            == 0
}
