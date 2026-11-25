//! Network worker. The UI thread sends [`Cmd`]s; a background thread does
//! blocking HTTP with `ureq`, holds a WebSocket for change notifications and
//! answers with [`Evt`]s. Everything is `http://` on the LAN, so no TLS.

use dnd_core::api::*;
use dnd_core::{Character, Sheet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Cmd {
    /// Point at a server (base URL like `http://192.168.1.10:8420`) and fetch info + character list.
    Connect { url: String, code: String, device_id: String, name: String },
    /// Ask to play this character (only works when the DM allows picking).
    Claim(String),
    /// Upload a character the player built; it becomes this device's character.
    Create { character: Box<Character>, gear: Vec<usize> },
    /// Ask the server to roll this player's ability scores.
    RollAbilities,
    /// Stop following a character (the DM took it away).
    Leave,
    Scan,
    LoadList,
    LoadCharacter(String),
    Patch { id: String, patch: PlayerPatch },
    Refresh,
    Shutdown,
}

#[derive(Debug)]
pub enum Evt {
    Connected(Info),
    /// Who the server says we are and which character we play. `fresh` is
    /// true right after a connect (as opposed to a background refresh).
    Seat { seat: Seat, fresh: bool },
    ClaimFailed(String),
    /// What characters can be built from on this server.
    Builder(Box<BuilderOptions>),
    /// The upload worked; this is our new seat.
    Created(Seat),
    CreateFailed(String),
    Disconnected(String),
    Characters(Vec<CharListed>),
    Character { character: Character, sheet: Sheet },
    Encounter(PlayerEncounter),
    Patched(Character),
    ScanResult(Vec<(String, String)>),
    Error(String),
    Busy(bool),
}

pub struct Client {
    pub tx: Sender<Cmd>,
    pub rx: Receiver<Evt>,
}

struct Worker {
    base: String,
    code: String,
    device_id: String,
    name: String,
    current: Option<String>,
    agent: ureq::Agent,
    evt: Sender<Evt>,
    ctx: egui::Context,
    ws_alive: Arc<Mutex<bool>>,
}

impl Worker {
    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.agent.get(&url);
        if !self.code.is_empty() {
            req = req.set("X-Join-Code", &self.code);
        }
        let resp = req.call().map_err(|e| anyhow::anyhow!("{}", short_err(e)))?;
        Ok(resp.into_json::<T>()?)
    }

    fn patch<T: serde::de::DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.agent.request("PATCH", &url).set("Content-Type", "application/json");
        if !self.code.is_empty() {
            req = req.set("X-Join-Code", &self.code);
        }
        if !self.device_id.is_empty() {
            req = req.set("X-Device-Id", &self.device_id);
        }
        let resp = req.send_string(&serde_json::to_string(body)?).map_err(|e| anyhow::anyhow!("{}", short_err(e)))?;
        Ok(resp.into_json::<T>()?)
    }

    fn post<T: serde::de::DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base, path);
        let mut req = self.agent.post(&url).set("Content-Type", "application/json");
        if !self.code.is_empty() {
            req = req.set("X-Join-Code", &self.code);
        }
        let resp = req.send_string(&serde_json::to_string(body)?).map_err(|e| anyhow::anyhow!("{}", short_err(e)))?;
        Ok(resp.into_json::<T>()?)
    }

    /// Tell the server who we are; the answer is our seat at the table.
    fn register(&self) -> anyhow::Result<Seat> {
        self.post("/api/player/register", &RegisterReq { device_id: self.device_id.clone(), name: self.name.clone() })
    }

    fn send(&self, e: Evt) {
        let _ = self.evt.send(e);
        self.ctx.request_repaint();
    }

    fn load_character(&mut self, id: &str) {
        self.current = Some(id.to_string());
        match (self.get::<Character>(&format!("/api/player/characters/{id}")), self.get::<Sheet>(&format!("/api/player/characters/{id}/sheet"))) {
            (Ok(character), Ok(sheet)) => self.send(Evt::Character { character, sheet }),
            (Err(e), _) | (_, Err(e)) => self.send(Evt::Error(format!("load character: {e}"))),
        }
        match self.get::<PlayerEncounter>("/api/player/encounter") {
            Ok(enc) => self.send(Evt::Encounter(enc)),
            Err(e) => self.send(Evt::Error(format!("encounter: {e}"))),
        }
    }

    fn start_ws(&self, cmd_tx: Sender<Cmd>) {
        *self.ws_alive.lock().unwrap() = false;
        let alive = Arc::new(Mutex::new(true));
        let ws_url = format!("{}/ws?device={}", self.base.replacen("http://", "ws://", 1), self.device_id);
        let evt = self.evt.clone();
        let ctx = self.ctx.clone();
        let flag = alive.clone();
        std::thread::Builder::new()
            .name("dnd-player-ws".into())
            .spawn(move || {
                let (mut socket, _) = match tungstenite::connect(&ws_url) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = evt.send(Evt::Error(format!("live updates unavailable ({e}); polling instead")));
                        ctx.request_repaint();
                        return;
                    }
                };
                if let tungstenite::stream::MaybeTlsStream::Plain(s) = socket.get_mut() {
                    let _ = s.set_read_timeout(Some(Duration::from_millis(1000)));
                }
                let mut last_rev: Option<u64> = None;
                loop {
                    if !*flag.lock().unwrap() {
                        let _ = socket.close(None);
                        break;
                    }
                    match socket.read() {
                        Ok(tungstenite::Message::Text(t)) => {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                                if let Some(rev) = v.get("revision").and_then(|r| r.as_u64()) {
                                    if last_rev != Some(rev) {
                                        last_rev = Some(rev);
                                        let _ = cmd_tx.send(Cmd::Refresh);
                                    }
                                }
                            }
                        }
                        Ok(tungstenite::Message::Close(_)) => break,
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(_) => break,
                    }
                }
            })
            .ok();
        // remember the newest flag so a later connect can stop this thread
        let mut guard = self.ws_alive.lock().unwrap();
        *guard = true;
        drop(guard);
        WS_FLAGS.lock().unwrap().push(alive);
    }
}

static WS_FLAGS: Mutex<Vec<Arc<Mutex<bool>>>> = Mutex::new(Vec::new());

fn stop_all_ws() {
    for f in WS_FLAGS.lock().unwrap().drain(..) {
        *f.lock().unwrap() = false;
    }
}

fn short_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let msg = serde_json::from_str::<ErrorBody>(&body).map(|b| b.error).unwrap_or(body);
            format!("HTTP {code}: {msg}")
        }
        ureq::Error::Transport(t) => t.message().map(|m| m.to_string()).unwrap_or_else(|| t.kind().to_string()),
    }
}

impl Client {
    pub fn spawn(ctx: egui::Context) -> Self {
        let (cmd_tx, cmd_rx) = channel::<Cmd>();
        let (evt_tx, evt_rx) = channel::<Evt>();
        let ws_cmd_tx = cmd_tx.clone();
        std::thread::Builder::new()
            .name("dnd-player-net".into())
            .spawn(move || {
                let mut w = Worker {
                    base: String::new(),
                    code: String::new(),
                    device_id: String::new(),
                    name: String::new(),
                    current: None,
                    agent: ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(4)).timeout(Duration::from_secs(12)).build(),
                    evt: evt_tx,
                    ctx,
                    ws_alive: Arc::new(Mutex::new(false)),
                };
                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        Cmd::Shutdown => {
                            stop_all_ws();
                            break;
                        }
                        Cmd::Connect { url, code, device_id, name } => {
                            w.send(Evt::Busy(true));
                            let base = normalize_url(&url);
                            if base != w.base {
                                w.current = None;
                            }
                            w.base = base;
                            w.code = code;
                            w.device_id = device_id;
                            w.name = name;
                            stop_all_ws();
                            match w.get::<Info>("/api/info") {
                                Ok(info) => {
                                    w.send(Evt::Connected(info));
                                    match w.register() {
                                        Ok(seat) => w.send(Evt::Seat { seat, fresh: true }),
                                        Err(e) => w.send(Evt::Error(format!("register: {e}"))),
                                    }
                                    match w.get::<Vec<CharListed>>("/api/characters") {
                                        Ok(list) => w.send(Evt::Characters(list)),
                                        Err(e) => w.send(Evt::Error(format!("characters: {e}"))),
                                    }
                                    if let Ok(o) = w.get::<BuilderOptions>("/api/player/builder") {
                                        w.send(Evt::Builder(Box::new(o)));
                                    }
                                    w.start_ws(ws_cmd_tx.clone());
                                }
                                Err(e) => w.send(Evt::Disconnected(format!("{e}"))),
                            }
                            w.send(Evt::Busy(false));
                        }
                        Cmd::Scan => {
                            w.send(Evt::Busy(true));
                            let found = scan_lan();
                            w.send(Evt::ScanResult(found));
                            w.send(Evt::Busy(false));
                        }
                        Cmd::LoadList => match w.get::<Vec<CharListed>>("/api/characters") {
                            Ok(list) => w.send(Evt::Characters(list)),
                            Err(e) => w.send(Evt::Error(format!("characters: {e}"))),
                        },
                        Cmd::LoadCharacter(id) => {
                            w.send(Evt::Busy(true));
                            w.load_character(&id);
                            w.send(Evt::Busy(false));
                        }
                        Cmd::Patch { id, patch } => match w.patch::<Character>(&format!("/api/player/characters/{id}"), &patch) {
                            Ok(c) => w.send(Evt::Patched(c)),
                            Err(e) => w.send(Evt::Error(format!("save: {e}"))),
                        },
                        Cmd::Claim(id) => {
                            let req = ClaimReq { device_id: w.device_id.clone(), character: id };
                            match w.post::<Seat>("/api/player/claim", &req) {
                                Ok(seat) => w.send(Evt::Seat { seat, fresh: true }),
                                Err(e) => w.send(Evt::ClaimFailed(format!("{e}"))),
                            }
                        }
                        Cmd::RollAbilities => {
                            let req = RollAbilitiesReq { device_id: w.device_id.clone() };
                            match w.post::<Seat>("/api/player/abilities/roll", &req) {
                                Ok(seat) => w.send(Evt::Seat { seat, fresh: false }),
                                Err(e) => w.send(Evt::Error(format!("roll: {e}"))),
                            }
                        }
                        Cmd::Create { character, gear } => {
                            w.send(Evt::Busy(true));
                            let req = CreateReq { device_id: w.device_id.clone(), character: *character, gear };
                            match w.post::<Seat>("/api/player/characters", &req) {
                                Ok(seat) => w.send(Evt::Created(seat)),
                                Err(e) => w.send(Evt::CreateFailed(format!("{e}"))),
                            }
                            w.send(Evt::Busy(false));
                        }
                        Cmd::Leave => w.current = None,
                        Cmd::Refresh => {
                            if !w.base.is_empty() && !w.device_id.is_empty() {
                                // heartbeat + "did the DM give me a (different) character?"
                                if let Ok(seat) = w.register() {
                                    w.send(Evt::Seat { seat, fresh: false });
                                }
                            }
                            if let Some(id) = w.current.clone() {
                                w.load_character(&id);
                            } else if !w.base.is_empty() {
                                if let Ok(list) = w.get::<Vec<CharListed>>("/api/characters") {
                                    w.send(Evt::Characters(list));
                                }
                            }
                        }
                    }
                }
            })
            .expect("spawn network thread");
        Self { tx: cmd_tx, rx: evt_rx }
    }
}

pub fn normalize_url(s: &str) -> String {
    let mut u = s.trim().trim_end_matches('/').to_string();
    if !u.starts_with("http://") && !u.starts_with("https://") {
        u = format!("http://{u}");
    }
    // add the default port if none given
    let after_scheme = u.splitn(2, "://").nth(1).unwrap_or("");
    if !after_scheme.contains(':') {
        u.push_str(":8420");
    }
    u
}

/// Our own LAN IPv4 (no packet is sent; the OS just picks the route).
pub fn local_ipv4() -> Option<Ipv4Addr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("10.254.254.254:1").ok()?;
    match s.local_addr().ok()?.ip() {
        IpAddr::V4(v4) if !v4.is_loopback() => Some(v4),
        _ => None,
    }
}

/// Probe every host on our /24 for a dnd-dm server on port 8420.
/// Returns (url, campaign name).
pub fn scan_lan() -> Vec<(String, String)> {
    let Some(me) = local_ipv4() else { return Vec::new() };
    let o = me.octets();
    let hosts: Vec<Ipv4Addr> = (1..=254u8).map(|h| Ipv4Addr::new(o[0], o[1], o[2], h)).collect();
    let hosts = Arc::new(Mutex::new(hosts));
    let found = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for _ in 0..48 {
        let hosts = hosts.clone();
        let found = found.clone();
        handles.push(std::thread::spawn(move || loop {
            let ip = match hosts.lock().unwrap().pop() {
                Some(ip) => ip,
                None => break,
            };
            let addr = SocketAddr::new(IpAddr::V4(ip), 8420);
            if TcpStream::connect_timeout(&addr, Duration::from_millis(350)).is_ok() {
                let url = format!("http://{ip}:8420");
                let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(2)).build();
                if let Ok(resp) = agent.get(&format!("{url}/api/info")).call() {
                    if let Ok(info) = resp.into_json::<Info>() {
                        if info.app == "dnd-dm" {
                            found.lock().unwrap().push((url, info.campaign));
                        }
                    }
                }
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let mut v = found.lock().unwrap().clone();
    v.sort();
    v
}
