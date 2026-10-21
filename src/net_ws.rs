//! WebSocket transport for online play in the web build.
//!
//! The native build matchmakes and plays over iroh (QUIC/UDP), which browsers
//! cannot use. For the web build this module talks to the **jumpy-relay**
//! server instead (see the `server/` directory of this repo): matchmaking
//! uses small JSON messages and, once matched, game traffic is forwarded by
//! the relay as base64 payloads inside `proxy` messages.
//!
//! The socket produced here plugs into the normal bones networking flow:
//! it implements [`NetworkSocket`] + [`ggrs::NonBlockingSocket`], is wrapped
//! in [`AnyGgrsSocket::Custom`] and handed to the game as a
//! [`NetworkMatchSocket`], exactly like the sockets created by the native
//! matchmakers. GGRS messages keep the same [`GameMessage`] postcard framing
//! as the native socket, and the `reliable` flag of the relay protocol is
//! used to separate GGRS traffic (`false`) from reliable messages (`true`).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

use bones_framework::networking::{
    AnyGgrsSocket, GameMessage, NetworkMatchSocket, NetworkSocket, SocketTarget,
};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::WebSocket;

use crate::prelude::*;

/// Default relay server (deployed on Railway).
pub const DEFAULT_RELAY_URL: &str = "wss://jumpy-production.up.railway.app/ws";

/// The relay URL to use: a `?relay=` query parameter on the page URL
/// overrides the default, which is handy for testing a local server.
pub fn relay_url() -> String {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
                if let Some(relay) = params.get("relay") {
                    if !relay.is_empty() {
                        return relay;
                    }
                }
            }
        }
    }
    DEFAULT_RELAY_URL.to_string()
}

/// Result of a successful matchmaking, as assigned by the relay.
#[derive(Clone, Copy, Debug)]
pub struct MatchedInfo {
    /// Random seed both players must use for the deterministic simulation.
    pub seed: u64,
    /// This client's player index within the match.
    pub player_idx: u32,
    /// Total number of players in the match.
    pub player_count: u32,
}

struct Shared {
    ws: WebSocket,
    accepted: bool,
    matched: Option<MatchedInfo>,
    closed: bool,
    /// Incoming GGRS messages: (sender player idx, message).
    ggrs_in: VecDeque<(u32, GameMessage)>,
    /// Incoming reliable messages: (sender player idx, payload).
    reliable_in: VecDeque<(u32, Vec<u8>)>,
    match_id: u8,
    player_idx: u32,
    player_count: u32,
    // Kept alive for as long as the connection is used.
    _on_open: Option<Closure<dyn FnMut()>>,
    _on_message: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
    _on_close: Option<Closure<dyn FnMut(web_sys::CloseEvent)>>,
    _on_error: Option<Closure<dyn FnMut(web_sys::ErrorEvent)>>,
}

impl Shared {
    fn send_json(&self, value: &Value) {
        if let Err(err) = self.ws.send_with_str(&value.to_string()) {
            warn!("relay send failed: {err:?}");
        }
    }

    fn send_proxy(&self, to: Option<u32>, reliable: bool, bytes: &[u8]) {
        let mut msg = json!({
            "type": "proxy",
            "reliable": reliable,
            "data": base64_encode(bytes),
        });
        match to {
            Some(idx) => msg["to"] = json!(idx),
            None => msg["to"] = json!("all"),
        }
        self.send_json(&msg);
    }
}

/// Handle for an in-progress matchmaking connection to the relay.
///
/// The same connection is later reused as the game socket via
/// [`WsOnlineClient::into_match_socket`].
#[derive(Clone)]
pub struct WsOnlineClient {
    shared: Rc<RefCell<Shared>>,
}

// The web build is single-threaded; the WebSocket handle never leaves it.
unsafe impl Send for WsOnlineClient {}
unsafe impl Sync for WsOnlineClient {}

impl WsOnlineClient {
    /// Connect to the relay and immediately request a match.
    pub fn connect_and_search(url: &str, game_id: &str, player_count: u32) -> Result<Self, String> {
        let ws = WebSocket::new(url).map_err(|e| format!("websocket error: {e:?}"))?;

        let shared = Rc::new(RefCell::new(Shared {
            ws: ws.clone(),
            accepted: false,
            matched: None,
            closed: false,
            ggrs_in: VecDeque::new(),
            reliable_in: VecDeque::new(),
            match_id: 0,
            player_idx: 0,
            player_count,
            _on_open: None,
            _on_message: None,
            _on_close: None,
            _on_error: None,
        }));

        // On open: send the matchmaking request.
        {
            let game_id = game_id.to_string();
            let shared2 = shared.clone();
            let on_open = Closure::wrap(Box::new(move || {
                shared2.borrow().send_json(&json!({
                    "type": "request_matchmaking",
                    "game_id": game_id,
                    "player_count": player_count,
                    "match_data": "",
                }));
                info!("Sent matchmaking request to relay");
            }) as Box<dyn FnMut()>);
            ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            shared.borrow_mut()._on_open = Some(on_open);
        }

        // On message: route relay messages into the shared queues.
        {
            let shared2 = shared.clone();
            let on_message = Closure::wrap(Box::new(move |ev: web_sys::MessageEvent| {
                let Some(text) = ev.data().as_string() else {
                    return;
                };
                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                    return;
                };
                let mut shared = shared2.borrow_mut();
                match value.get("type").and_then(Value::as_str) {
                    Some("accepted") => {
                        shared.accepted = true;
                    }
                    Some("success") => {
                        let seed = value
                            .get("seed")
                            .and_then(Value::as_str)
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0);
                        let player_idx =
                            value.get("player_idx").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let player_count = value
                            .get("player_count")
                            .and_then(Value::as_u64)
                            .unwrap_or(2) as u32;
                        shared.player_idx = player_idx;
                        shared.player_count = player_count;
                        shared.matched = Some(MatchedInfo {
                            seed,
                            player_idx,
                            player_count,
                        });
                        info!("Relay matched us: idx {player_idx} of {player_count}, seed {seed}");
                    }
                    Some("proxy_msg") => {
                        let from = value.get("from").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let reliable = value
                            .get("reliable")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                        let Some(data) = value
                            .get("data")
                            .and_then(Value::as_str)
                            .and_then(base64_decode)
                        else {
                            return;
                        };
                        if reliable {
                            shared.reliable_in.push_back((from, data));
                        } else if let Ok(game_message) = postcard::from_bytes::<GameMessage>(&data)
                        {
                            shared.ggrs_in.push_back((from, game_message));
                        }
                    }
                    Some("peer_disconnected") => {
                        info!("Peer disconnected from relay match");
                    }
                    _ => {}
                }
            }) as Box<dyn FnMut(web_sys::MessageEvent)>);
            ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            shared.borrow_mut()._on_message = Some(on_message);
        }

        // Close / error tracking.
        {
            let shared2 = shared.clone();
            let on_close = Closure::wrap(Box::new(move |_ev: web_sys::CloseEvent| {
                shared2.borrow_mut().closed = true;
            }) as Box<dyn FnMut(web_sys::CloseEvent)>);
            ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            shared.borrow_mut()._on_close = Some(on_close);

            let shared2 = shared.clone();
            let on_error = Closure::wrap(Box::new(move |_ev: web_sys::ErrorEvent| {
                shared2.borrow_mut().closed = true;
            }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
            ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            shared.borrow_mut()._on_error = Some(on_error);
        }

        Ok(Self { shared })
    }

    /// Whether the relay has accepted our matchmaking request.
    pub fn is_accepted(&self) -> bool {
        self.shared.borrow().accepted
    }

    /// Whether the connection has closed or errored.
    pub fn is_closed(&self) -> bool {
        self.shared.borrow().closed
    }

    /// The match assignment, once the relay has found enough players.
    pub fn matched(&self) -> Option<MatchedInfo> {
        self.shared.borrow().matched
    }

    /// Cancel matchmaking and close the connection.
    pub fn cancel(&self) {
        {
            let shared = self.shared.borrow();
            if shared.matched.is_none() {
                shared.send_json(&json!({ "type": "stop_matchmaking" }));
            }
        }
        self.shared.borrow().ws.close().ok();
        self.shared.borrow_mut().closed = true;
    }

    /// Convert this connection into the match socket used by the game.
    pub fn into_match_socket(self) -> NetworkMatchSocket {
        let socket = WsSocket {
            shared: self.shared,
        };
        NetworkMatchSocket::new(Arc::new(AnyGgrsSocket::Custom(Box::new(socket))))
    }
}

/// The game socket running over the relay WebSocket connection.
#[derive(Clone)]
pub struct WsSocket {
    shared: Rc<RefCell<Shared>>,
}

unsafe impl Send for WsSocket {}
unsafe impl Sync for WsSocket {}

impl NetworkSocket for WsSocket {
    fn ggrs_socket(&self) -> AnyGgrsSocket {
        AnyGgrsSocket::Custom(Box::new(self.clone()))
    }

    fn send_reliable(&self, target: SocketTarget, message: &[u8]) {
        let shared = self.shared.borrow();
        match target {
            SocketTarget::Player(idx) => shared.send_proxy(Some(idx), true, message),
            SocketTarget::All => shared.send_proxy(None, true, message),
        }
    }

    fn recv_reliable(&self) -> Vec<(u32, Vec<u8>)> {
        self.shared.borrow_mut().reliable_in.drain(..).collect()
    }

    fn close(&self) {
        self.shared.borrow().ws.close().ok();
        self.shared.borrow_mut().closed = true;
    }

    fn player_idx(&self) -> u32 {
        self.shared.borrow().player_idx
    }

    fn player_count(&self) -> u32 {
        self.shared.borrow().player_count
    }

    fn increment_match_id(&mut self) {
        let mut shared = self.shared.borrow_mut();
        shared.match_id = shared.match_id.wrapping_add(1);
    }
}

impl ggrs::NonBlockingSocket<usize> for WsSocket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &usize) {
        let shared = self.shared.borrow();
        let game_message = GameMessage {
            match_id: shared.match_id,
            message: msg.clone(),
        };
        if let Ok(bytes) = postcard::to_allocvec(&game_message) {
            shared.send_proxy(Some(*addr as u32), false, &bytes);
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(usize, ggrs::Message)> {
        let mut shared = self.shared.borrow_mut();
        let match_id = shared.match_id;
        let mut messages = Vec::new();
        while let Some((from, game_message)) = shared.ggrs_in.pop_front() {
            if game_message.match_id == match_id {
                messages.push((from as usize, game_message.message));
            }
        }
        messages
    }
}

// ---------------------------------------------------------------------------
// Minimal base64 codec (standard alphabet, with padding), just big enough
// for the relay payloads and avoids an extra dependency.
// ---------------------------------------------------------------------------

const B64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64_ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(B64_ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64_ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64_ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    fn value_of(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = text.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        let mut vals = [0u32; 4];
        for (i, &c) in chunk.iter().enumerate() {
            vals[i] = if c == b'=' { 0 } else { value_of(c)? };
        }
        let n = (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3];
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for data in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"hello-ggrs-frame",
            &[0u8, 1, 2, 250, 251, 252, 253, 254, 255][..],
        ] {
            let encoded = base64_encode(data);
            assert_eq!(base64_decode(&encoded).as_deref(), Some(data));
        }
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_decode("Zm9vZA==").unwrap(), b"food");
    }
}
