//! LAN server for player apps (Android / Windows character sheets).
//!
//! Plain JSON over HTTP plus one WebSocket that announces "something changed,
//! revision N" so clients re-fetch. Reads are open on the LAN; writes need the
//! campaign's join code (header `X-Join-Code` or `?code=`), if one is set.
//! See `docs/API.md` for the route list.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dnd_core::api::*;
use dnd_core::view::Viewer;
use dnd_core::{model::*, World};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;

pub type SharedWorld = Arc<RwLock<World>>;

#[derive(Clone)]
struct AppState {
    world: SharedWorld,
}

/// A running server. Dropping it (or calling `stop`) shuts it down.
pub struct ServerHandle {
    pub addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ServerHandle {
    pub fn stop(mut self) {
        self.shutdown_inner();
    }
    fn shutdown_inner(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.shutdown_inner();
    }
}

/// Start the server on `0.0.0.0:port` (reachable from the LAN) in a background thread.
pub fn start(world: SharedWorld, port: u16) -> anyhow::Result<ServerHandle> {
    start_on(world, "0.0.0.0", port)
}

/// Start the server on this machine only (`127.0.0.1`). Used by the tests,
/// so running them never raises a Windows Firewall prompt.
pub fn start_local(world: SharedWorld, port: u16) -> anyhow::Result<ServerHandle> {
    start_on(world, "127.0.0.1", port)
}

fn start_on(world: SharedWorld, host: &'static str, port: u16) -> anyhow::Result<ServerHandle> {
    let (tx, rx) = oneshot::channel::<()>();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<anyhow::Result<SocketAddr>>();
    let thread = std::thread::Builder::new().name("dnd-server".into()).spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = ready_tx.send(Err(e.into()));
                return;
            }
        };
        rt.block_on(async move {
            let listener = match tokio::net::TcpListener::bind((host, port)).await {
                Ok(l) => l,
                Err(e) => {
                    let _ = ready_tx.send(Err(anyhow::anyhow!("bind port {port}: {e}")));
                    return;
                }
            };
            let addr = listener.local_addr().unwrap();
            let _ = ready_tx.send(Ok(addr));
            let app = router(world);
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await;
        });
    })?;
    let addr = ready_rx.recv()??;
    Ok(ServerHandle { addr, shutdown: Some(tx), thread: Some(thread) })
}

/// IPv4 addresses of this machine, for showing "connect to 192.168.x.x".
pub fn local_ips() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(list) = local_ip_address::list_afinet_netifas() {
        for (name, ip) in list {
            if let std::net::IpAddr::V4(ip4) = ip {
                if ip4.is_loopback() || ip4.is_link_local() {
                    continue;
                }
                v.push(format!("{ip4} ({name})"));
            }
        }
    }
    if v.is_empty() {
        if let Ok(ip) = local_ip_address::local_ip() {
            v.push(ip.to_string());
        }
    }
    v
}

fn router(world: SharedWorld) -> Router {
    let state = AppState { world };
    Router::new()
        .route("/", get(index))
        .route("/api/info", get(info))
        .route("/api/rules", get(rules))
        .route("/api/content", get(content_counts))
        .route("/api/content/{kind}", get(content_list))
        .route("/api/content/{kind}/{id}", get(content_get))
        .route("/api/characters", get(characters).post(character_create))
        .route("/api/characters/{id}", get(character_get).put(character_put))
        .route("/api/characters/{id}/sheet", get(sheet_get))
        .route("/api/characters/{id}/sheet.html", get(sheet_html))
        .route("/api/encounter", get(encounter))
        .route("/api/roll", post(roll))
        // player-safe views: hidden content, hidden items and DM notes removed
        .route("/api/player/content/{kind}", get(player_content_list))
        .route("/api/player/content/{kind}/{id}", get(player_content_get))
        .route("/api/player/characters/{id}", get(player_character_get).patch(player_character_patch))
        .route("/api/player/characters/{id}/sheet", get(player_sheet_get))
        .route("/api/player/encounter", get(encounter))
        // who is at the table: apps register by name, the DM assigns characters
        .route("/api/player/register", post(player_register))
        .route("/api/player/claim", post(player_claim))
        .route("/api/player/builder", get(player_builder))
        .route("/api/player/map", get(player_map))
        .route("/api/player/map/move", post(player_map_move))
        .route("/api/player/map/door", post(player_map_door))
        .route("/api/player/abilities/roll", post(player_roll_abilities))
        .route("/api/player/table", get(player_table))
        .route("/api/player/spells", get(player_spell_list).post(player_spells_choose))
        .route("/api/player/roll", post(player_table_roll))
        .route("/api/player/attack", post(player_attack))
        .route("/api/player/cast", post(player_cast))
        .route("/api/player/do", post(player_do))
        .route("/api/player/characters", post(player_character_create))
        .route("/api/dm/devices", get(dm_devices))
        .route("/api/dm/devices/{id}", axum::routing::put(dm_device_assign).delete(dm_device_forget))
        // DM remote control (needs dm_code)
        .route("/api/dm/characters/{id}/inventory", post(dm_inventory_add))
        .route("/api/dm/characters/{id}/inventory/{index}", axum::routing::delete(dm_inventory_remove).patch(dm_inventory_patch))
        .route("/ws", get(ws_upgrade))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ------------------------------------------------------------------ helpers

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ErrorBody { error: msg.into() })).into_response()
}

fn authorized(world: &World, headers: &HeaderMap, q: &HashMap<String, String>) -> bool {
    let code = &world.campaign.settings.join_code;
    if code.is_empty() {
        return true;
    }
    let given = headers
        .get("x-join-code")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| q.get("code").cloned())
        .unwrap_or_default();
    given.trim() == code.trim()
}

fn dm_authorized(world: &World, headers: &HeaderMap, q: &HashMap<String, String>) -> bool {
    let code = world.campaign.settings.dm_code.trim();
    if code.is_empty() {
        return false;
    }
    let given = headers
        .get("x-dm-code")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| q.get("dm_code").cloned())
        .unwrap_or_default();
    given.trim() == code
}

fn parse_kind(s: &str) -> Option<Kind> {
    Kind::from_folder(s)
}

// ------------------------------------------------------------------ handlers

async fn index(State(st): State<AppState>) -> Html<String> {
    let w = st.world.read().unwrap();
    let chars: Vec<String> = w
        .campaign
        .characters
        .iter()
        .map(|c| format!("<li><a href=\"/api/characters/{0}/sheet.html\">{1}</a> (<a href=\"/api/characters/{0}/sheet\">json</a>)</li>", c.id, c.name))
        .collect();
    Html(format!(
        "<!doctype html><html><head><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>dnd-dm</title>\
         <style>body{{font-family:sans-serif;margin:24px;max-width:720px;background:#111;color:#eee}}a{{color:#ff8a00}}code{{background:#222;padding:2px 4px}}</style></head><body>\
         <h1 style=\"color:#ff8a00\">dnd-dm server</h1><p>Campaign: <b>{}</b> &middot; revision {} &middot; join code {}</p>\
         <h2>Characters</h2><ul>{}</ul>\
         <h2>API</h2><ul><li><a href=/api/info>/api/info</a></li><li><a href=/api/content>/api/content</a></li><li><a href=/api/characters>/api/characters</a></li><li><a href=/api/encounter>/api/encounter</a></li><li><a href=/api/rules>/api/rules</a></li></ul>\
         <p>WebSocket at <code>/ws</code> pushes <code>{{\"type\":\"revision\",\"revision\":N}}</code> whenever the DM changes something.</p>\
         </body></html>",
        w.campaign.name,
        w.revision,
        if w.campaign.settings.join_code.is_empty() { "not required".to_string() } else { "required".to_string() },
        chars.join("")
    ))
}

async fn info(State(st): State<AppState>) -> Json<Info> {
    let w = st.world.read().unwrap();
    Json(Info {
        app: "dnd-dm".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        campaign: w.campaign.name.clone(),
        revision: w.revision,
        join_code_required: !w.campaign.settings.join_code.is_empty(),
        characters: w.campaign.characters.len(),
        encounter_active: w.campaign.encounter.active,
        packs: w.library.packs.iter().map(|p| p.name.clone()).collect(),
        game_live: w.campaign.settings.game_live,
    })
}

async fn rules(State(st): State<AppState>) -> Json<Rules> {
    Json(st.world.read().unwrap().library.rules.clone())
}

async fn content_counts(State(st): State<AppState>) -> Json<HashMap<String, usize>> {
    let w = st.world.read().unwrap();
    Json(Kind::ALL.iter().map(|k| (k.folder().to_string(), w.library.count(*k))).collect())
}

async fn content_list(State(st): State<AppState>, Path(kind): Path<String>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let Some(k) = parse_kind(&kind) else { return err(StatusCode::NOT_FOUND, "unknown kind") };
    let w = st.world.read().unwrap();
    let dm = dm_authorized(&w, &headers, &q);
    let list: Vec<Listed> = w
        .library
        .list(k)
        .into_iter()
        .filter(|(id, _)| dm || w.library.get(k, id).map(|e| !e.hidden()).unwrap_or(false))
        .map(|(id, name)| Listed { id, name })
        .collect();
    Json(list).into_response()
}

async fn content_get(State(st): State<AppState>, Path((kind, id)): Path<(String, String)>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let Some(k) = parse_kind(&kind) else { return err(StatusCode::NOT_FOUND, "unknown kind") };
    let w = st.world.read().unwrap();
    let dm = dm_authorized(&w, &headers, &q);
    match w.library.get(k, &id) {
        Some(e) if dm || !e.hidden() => Json(e).into_response(),
        _ => err(StatusCode::NOT_FOUND, "no such entry"),
    }
}

async fn characters(State(st): State<AppState>) -> Json<Vec<CharListed>> {
    let w = st.world.read().unwrap();
    Json(
        w.campaign
            .characters
            .iter()
            .map(|c| {
                let s = dnd_core::derive(c, &w.library);
                CharListed { id: c.id.clone(), name: c.name.clone(), player: c.player.clone(), level: s.level, class_line: s.class_line }
            })
            .collect(),
    )
}

async fn character_get(State(st): State<AppState>, Path(id): Path<String>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let w = st.world.read().unwrap();
    let dm = dm_authorized(&w, &headers, &q);
    match w.campaign.character(&id) {
        Some(c) => Json(if dm { c.clone() } else { c.player_view() }).into_response(),
        None => err(StatusCode::NOT_FOUND, "no such character"),
    }
}

async fn character_put(
    State(st): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(mut ch): Json<Character>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required; players edit through /api/player/characters");
    }
    ch.id = id.clone();
    match w.campaign.character_mut(&id) {
        Some(slot) => *slot = ch,
        None => return err(StatusCode::NOT_FOUND, "no such character"),
    }
    w.campaign.log(format!("character '{id}' replaced via API"));
    w.touch();
    StatusCode::NO_CONTENT.into_response()
}

async fn character_create(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(mut ch): Json<Character>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required; players upload through /api/player/characters");
    }
    if ch.id.is_empty() {
        ch.id = Character::slug(&ch.name);
    }
    if ch.id.is_empty() {
        return err(StatusCode::BAD_REQUEST, "name required");
    }
    if w.campaign.character(&ch.id).is_some() {
        return err(StatusCode::CONFLICT, "character id already exists");
    }
    let id = ch.id.clone();
    let name = ch.name.clone();
    if ch.hp.current <= 0 {
        // a fresh character starts at full health
        ch.hp.current = dnd_core::derive(&ch, &w.library).hp_max;
    }
    w.campaign.characters.push(ch);
    w.campaign.log(format!("character '{id}' created via the DM API"));
    w.touch();
    (StatusCode::CREATED, Json(Listed { id, name })).into_response()
}

async fn sheet_get(State(st): State<AppState>, Path(id): Path<String>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let w = st.world.read().unwrap();
    let dm = dm_authorized(&w, &headers, &q);
    match w.sheet(&id) {
        Some(s) => Json(if dm { s } else { s.player_view() }).into_response(),
        None => err(StatusCode::NOT_FOUND, "no such character"),
    }
}

async fn sheet_html(State(st): State<AppState>, Path(id): Path<String>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let w = st.world.read().unwrap();
    let dm = dm_authorized(&w, &headers, &q);
    match w.sheet(&id) {
        Some(s) => Html(dnd_core::export::character_sheet_html(&if dm { s } else { s.player_view() }, &w.library)).into_response(),
        None => err(StatusCode::NOT_FOUND, "no such character"),
    }
}

async fn encounter(State(st): State<AppState>) -> Json<PlayerEncounter> {
    let w = st.world.read().unwrap();
    let e = &w.campaign.encounter;
    let show_hp = w.campaign.settings.show_monster_hp_to_players;
    // where things are comes from /api/player/map, which knows what each player can see
    let in_view: Vec<String> = w.map_view(&Viewer::Party).tokens.into_iter().map(|t| t.id).collect();
    let m = &w.campaign.map;
    let map = MapState { id: m.id.clone(), name: m.name.clone(), width: m.width, height: m.height, feet_per_square: m.feet_per_square, ..Default::default() };
    let combatants = e
        .combatants
        .iter()
        .enumerate()
        .filter(|(_, c)| c.visible)
        .map(|(i, c)| {
            let is_pc = matches!(c.kind, CombatantKind::Pc { .. });
            PlayerCombatant {
                id: c.id.clone(),
                name: c.name.clone(),
                is_pc,
                initiative: c.initiative,
                hp: if is_pc || show_hp { Some(c.hp) } else { None },
                hp_max: if is_pc || show_hp { Some(c.hp_max) } else { None },
                ac: if is_pc { Some(c.ac) } else { None },
                size: c.size.clone(),
                pos: c.pos.filter(|_| in_view.contains(&c.id)),
                conditions: c.conditions.clone(),
                is_current: e.active && i == e.turn,
            }
        })
        .collect();
    Json(PlayerEncounter { name: e.name.clone(), active: e.active, round: e.round, map, combatants })
}

async fn roll(State(st): State<AppState>, Json(req): Json<RollReq>) -> Response {
    let mut w = st.world.write().unwrap();
    match w.roll(&req.formula) {
        Ok(r) => Json(RollResp { formula: req.formula, total: r.int(), rolls: r.rolls.iter().map(|d| d.to_string()).collect() }).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

// ------------------------------------------------------------------ player views

async fn player_content_list(State(st): State<AppState>, Path(kind): Path<String>) -> Response {
    let Some(k) = parse_kind(&kind) else { return err(StatusCode::NOT_FOUND, "unknown kind") };
    let w = st.world.read().unwrap();
    let list: Vec<Listed> = w
        .library
        .list(k)
        .into_iter()
        .filter(|(id, _)| w.library.get(k, id).map(|e| !e.hidden()).unwrap_or(false))
        .map(|(id, name)| Listed { id, name })
        .collect();
    Json(list).into_response()
}

async fn player_content_get(State(st): State<AppState>, Path((kind, id)): Path<(String, String)>) -> Response {
    let Some(k) = parse_kind(&kind) else { return err(StatusCode::NOT_FOUND, "unknown kind") };
    let w = st.world.read().unwrap();
    match w.library.get(k, &id) {
        Some(e) if !e.hidden() => Json(e).into_response(),
        _ => err(StatusCode::NOT_FOUND, "no such entry"),
    }
}

async fn player_character_get(State(st): State<AppState>, Path(id): Path<String>) -> Response {
    let w = st.world.read().unwrap();
    match w.campaign.character(&id) {
        Some(c) => Json(c.player_view()).into_response(),
        None => err(StatusCode::NOT_FOUND, "no such character"),
    }
}

async fn player_character_patch(
    State(st): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(patch): Json<PlayerPatch>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "join code required");
    }
    // a registered device may only edit its own character, unless the DM lets players pick
    let dev = headers.get("x-device-id").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    if let Some(d) = &dev {
        w.seen(d);
    }
    let own = dev.as_deref().and_then(|d| w.campaign.device_character(d)).map(|c| c.id == id).unwrap_or(false);
    if !own && !w.campaign.settings.players_choose_character {
        return err(StatusCode::FORBIDDEN, "the DM has not assigned this character to your device");
    }
    let Some(ch) = w.campaign.character_mut(&id) else { return err(StatusCode::NOT_FOUND, "no such character") };
    ch.apply_player_patch(patch);
    let view = ch.player_view();
    w.sync_combatant_from_pc(&id);
    w.touch();
    Json(view).into_response()
}

async fn player_sheet_get(State(st): State<AppState>, Path(id): Path<String>) -> Response {
    let w = st.world.read().unwrap();
    match w.sheet(&id) {
        Some(s) => Json(s.player_view()).into_response(),
        None => err(StatusCode::NOT_FOUND, "no such character"),
    }
}

// ------------------------------------------------------------------ DM remote

#[derive(Deserialize)]
struct InventoryAdd {
    item: String,
    #[serde(default)]
    name: String,
    #[serde(default = "one")]
    qty: u32,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    custom: bool,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    dm_notes: String,
}

fn one() -> u32 {
    1
}

async fn dm_inventory_add(
    State(st): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<InventoryAdd>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required");
    }
    if !body.custom && !w.library.items.contains_key(&body.item) {
        return err(StatusCode::BAD_REQUEST, "unknown item id (set custom=true for free text)");
    }
    let item_name = body.item.clone();
    let Some(ch) = w.campaign.character_mut(&id) else { return err(StatusCode::NOT_FOUND, "no such character") };
    ch.inventory.push(InventoryItem {
        item: body.item,
        name: body.name,
        qty: body.qty.max(1),
        hidden: body.hidden,
        custom: body.custom,
        notes: body.notes,
        dm_notes: body.dm_notes,
        ..Default::default()
    });
    let index = ch.inventory.len() - 1;
    w.campaign.log(format!("DM gave {item_name} to '{id}'"));
    w.touch();
    (StatusCode::CREATED, Json(serde_json::json!({ "index": index }))).into_response()
}

async fn dm_inventory_remove(
    State(st): State<AppState>,
    Path((id, index)): Path<(String, usize)>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required");
    }
    let Some(ch) = w.campaign.character_mut(&id) else { return err(StatusCode::NOT_FOUND, "no such character") };
    if index >= ch.inventory.len() {
        return err(StatusCode::NOT_FOUND, "no such inventory slot");
    }
    let removed = ch.inventory.remove(index);
    w.campaign.log(format!("DM removed {} from '{id}'", removed.item));
    w.touch();
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
struct InventoryPatch {
    hidden: Option<bool>,
    qty: Option<u32>,
    equipped: Option<bool>,
    attuned: Option<bool>,
    notes: Option<String>,
    dm_notes: Option<String>,
}

async fn dm_inventory_patch(
    State(st): State<AppState>,
    Path((id, index)): Path<(String, usize)>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(p): Json<InventoryPatch>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required");
    }
    let Some(ch) = w.campaign.character_mut(&id) else { return err(StatusCode::NOT_FOUND, "no such character") };
    let Some(slot) = ch.inventory.get_mut(index) else { return err(StatusCode::NOT_FOUND, "no such inventory slot") };
    if let Some(v) = p.hidden {
        slot.hidden = v;
    }
    if let Some(v) = p.qty {
        slot.qty = v;
    }
    if let Some(v) = p.equipped {
        slot.equipped = v;
    }
    if let Some(v) = p.attuned {
        slot.attuned = v;
    }
    if let Some(v) = p.notes {
        slot.notes = v;
    }
    if let Some(v) = p.dm_notes {
        slot.dm_notes = v;
    }
    let item = slot.clone();
    w.touch();
    Json(item).into_response()
}

// ------------------------------------------------------------------ devices

fn seat_of(w: &World, device_id: &str) -> Seat {
    let name = w.campaign.device(device_id).map(|d| d.name.clone()).unwrap_or_default();
    let ch = w.campaign.device_character(device_id);
    let dev = w.campaign.device(device_id);
    let made = match (dev, ch) {
        (Some(d), Some(c)) => !d.made.is_empty() && d.made == c.id,
        _ => false,
    };
    Seat {
        device_id: device_id.to_string(),
        name,
        character: ch.map(|c| c.id.clone()),
        character_name: ch.map(|c| c.name.clone()).unwrap_or_default(),
        can_choose: w.campaign.settings.players_choose_character,
        can_create: !w.campaign.settings.game_live && (ch.is_none() || made),
        game_live: w.campaign.settings.game_live,
        made_character: made,
        rolled: dev.map(|d| d.rolled.clone()).unwrap_or_default(),
        start_level: w.campaign.settings.player_start_level.clamp(1, 20),
    }
}

fn valid_device_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A player app says "I am <name>". Called on connect and again on every
/// refresh, so it doubles as the heartbeat; the answer says which character
/// the DM has given this device.
async fn player_register(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(req): Json<RegisterReq>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "join code required");
    }
    let name: String = req.name.trim().chars().filter(|c| !c.is_control()).take(40).collect();
    if name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "enter your name first");
    }
    if !valid_device_id(&req.device_id) {
        return err(StatusCode::BAD_REQUEST, "bad device id");
    }
    if w.campaign.device(&req.device_id).is_none() && w.campaign.devices.len() >= 64 {
        return err(StatusCode::CONFLICT, "too many devices registered; the DM can forget old ones");
    }
    let changed = w.campaign.register_device(&req.device_id, &name);
    let was_online = w.device_online(&req.device_id);
    w.seen(&req.device_id);
    if changed || !was_online {
        w.touch();
    }
    Json(seat_of(&w, &req.device_id)).into_response()
}

/// A player picks their own character; only when the DM allows it.
async fn player_claim(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(req): Json<ClaimReq>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "join code required");
    }
    if w.campaign.device(&req.device_id).is_none() {
        return err(StatusCode::NOT_FOUND, "register first");
    }
    if !w.campaign.settings.players_choose_character {
        return err(StatusCode::FORBIDDEN, "the DM assigns characters in this campaign");
    }
    if w.campaign.character(&req.character).is_none() {
        return err(StatusCode::NOT_FOUND, "no such character");
    }
    if w.campaign.devices.iter().any(|d| d.id != req.device_id && d.character == req.character) {
        return err(StatusCode::CONFLICT, "someone else is already playing that character");
    }
    w.campaign.assign_device(&req.device_id, Some(&req.character));
    w.seen(&req.device_id);
    w.touch();
    Json(seat_of(&w, &req.device_id)).into_response()
}

// ------------------------------------------------------------------ the map

/// Who is looking: the device's character, or the whole party for a device
/// without one (and for anyone who does not say).
fn viewer_of(w: &World, device_id: Option<&str>) -> Viewer {
    match device_id.and_then(|d| w.campaign.device_character(d)) {
        Some(c) => Viewer::Character(c.id.clone()),
        None => Viewer::Party,
    }
}

/// The map as this player may see it: only explored squares, and tokens only
/// where their character can see them right now.
async fn player_map(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let w = st.world.read().unwrap();
    let dev = q.get("device").map(|s| s.as_str()).or_else(|| headers.get("x-device-id").and_then(|v| v.to_str().ok()));
    Json(w.map_view(&viewer_of(&w, dev))).into_response()
}

/// The token of the character this device plays, if it is on the map.
fn own_token(w: &World, device_id: &str) -> Result<(String, String), Response> {
    let Some(ch) = w.campaign.device_character(device_id) else {
        return Err(err(StatusCode::FORBIDDEN, "the DM has not given you a character yet"));
    };
    let token = w.campaign.encounter.combatants.iter().find(|c| matches!(&c.kind, CombatantKind::Pc { id } if id == &ch.id) && c.pos.is_some());
    match token {
        Some(t) => Ok((t.id.clone(), ch.id.clone())),
        None => Err(err(StatusCode::CONFLICT, "the DM has not put you on the map yet")),
    }
}

/// A player walks their own token. The way must be clear and within reach;
/// in a fight it must be their turn.
async fn player_map_move(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, Json(req): Json<MapMoveReq>) -> Response {
    let mut w = st.world.write().unwrap();
    if !authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "join code required");
    }
    let (token, ch) = match own_token(&w, &req.device_id) {
        Ok(t) => t,
        Err(r) => return r,
    };
    w.seen(&req.device_id);
    match w.player_walk(&token, dnd_core::grid::Pos::new(req.x, req.y)) {
        Ok(feet) => Json(MapMoved { feet, map: w.map_view(&Viewer::Character(ch)) }).into_response(),
        Err(e) => err(StatusCode::CONFLICT, e),
    }
}

/// A player opens or closes a door their character is standing next to.
async fn player_map_door(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, Json(req): Json<MapDoorReq>) -> Response {
    let mut w = st.world.write().unwrap();
    if !authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "join code required");
    }
    let (token, ch) = match own_token(&w, &req.device_id) {
        Ok(t) => t,
        Err(r) => return r,
    };
    w.seen(&req.device_id);
    if !w.campaign.settings.players_move_tokens {
        return err(StatusCode::FORBIDDEN, "ask the DM to open it");
    }
    let Some(i) = w.campaign.map.door_near(req.x, req.y, 0.45) else { return err(StatusCode::NOT_FOUND, "no door there") };
    let door = w.campaign.map.walls[i].clone();
    if door.kind == WallKind::SecretDoor && !door.open {
        return err(StatusCode::NOT_FOUND, "no door there");
    }
    let side = w.campaign.encounter.find(&token).map(|c| w.library.rules.size_squares(&c.size)).unwrap_or(1) as f32;
    let at = w.campaign.encounter.find(&token).and_then(|c| c.pos).unwrap_or_default();
    let (cx, cy) = (at.x as f32 + side / 2.0, at.y as f32 + side / 2.0);
    let (mx, my) = door.mid();
    if (cx - mx).abs().max((cy - my).abs()) > side / 2.0 + 0.6 {
        return err(StatusCode::CONFLICT, "you are too far from that door");
    }
    match w.toggle_door(i, false) {
        Ok(open) => {
            let who = w.campaign.character(&ch).map(|c| c.name.clone()).unwrap_or_default();
            w.campaign.log(format!("{who} {} a door", if open { "opens" } else { "closes" }));
            Json(MapMoved { feet: 0, map: w.map_view(&Viewer::Character(ch)) }).into_response()
        }
        Err(e) => err(StatusCode::CONFLICT, e),
    }
}

/// Species, classes, backgrounds and rules tables for the app's character builder.
// ------------------------------------------------------------------ playing

/// The id of the character this device plays, after the usual checks.
fn seated(w: &mut World, headers: &HeaderMap, q: &HashMap<String, String>, device_id: &str) -> Result<String, Response> {
    if !authorized(w, headers, q) {
        return Err(err(StatusCode::UNAUTHORIZED, "join code required"));
    }
    w.seen(device_id);
    match w.campaign.device_character(device_id) {
        Some(c) => Ok(c.id.clone()),
        None => Err(err(StatusCode::FORBIDDEN, "the DM has not given you a character yet")),
    }
}

fn played(r: Result<dnd_core::play::ActionResult, String>) -> Response {
    match r {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(StatusCode::CONFLICT, e),
    }
}

/// What has happened at the table lately (rolls, attacks, turns).
async fn player_table(State(st): State<AppState>) -> Json<Vec<TableEntry>> {
    let w = st.world.read().unwrap();
    let t = &w.campaign.table;
    Json(t[t.len().saturating_sub(60)..].to_vec())
}

/// Every spell players may read.
async fn player_spell_list(State(st): State<AppState>) -> Json<Vec<Spell>> {
    let w = st.world.read().unwrap();
    Json(w.library.spells.values().filter(|s| !s.hidden).cloned().collect())
}

/// A player picks the spells their character knows.
async fn player_spells_choose(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, Json(req): Json<SpellsReq>) -> Response {
    let mut w = st.world.write().unwrap();
    let id = match seated(&mut w, &headers, &q, &req.device_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let w = &mut *w;
    let live = w.campaign.settings.game_live;
    let Some(ch) = w.campaign.characters.iter_mut().find(|c| c.id == id) else { return err(StatusCode::NOT_FOUND, "no such character") };
    let before = ch.spells.known.len();
    if let Err(e) = dnd_core::play::choose_spells(&w.library, ch, &req.known, live) {
        return err(StatusCode::CONFLICT, e);
    }
    let view = ch.player_view();
    let (name, now) = (ch.name.clone(), ch.spells.known.len());
    if now != before {
        w.campaign.log(format!("{name} now knows {now} spells (was {before})"));
    }
    w.touch();
    Json(view).into_response()
}

async fn player_table_roll(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, Json(req): Json<TableRollReq>) -> Response {
    let mut w = st.world.write().unwrap();
    let id = match seated(&mut w, &headers, &q, &req.device_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    if req.formula.len() > 80 || req.label.len() > 60 {
        return err(StatusCode::BAD_REQUEST, "that roll is too long");
    }
    let name = w.campaign.character(&id).map(|c| c.name.clone()).unwrap_or_default();
    played(w.table_roll(&name, &req.formula, &req.label, req.mode))
}

async fn player_attack(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, Json(req): Json<AttackReq>) -> Response {
    let mut w = st.world.write().unwrap();
    let id = match seated(&mut w, &headers, &q, &req.device_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    played(w.pc_attack(&id, req.attack, &req.target, req.mode))
}

async fn player_cast(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, Json(req): Json<CastReq>) -> Response {
    let mut w = st.world.write().unwrap();
    let id = match seated(&mut w, &headers, &q, &req.device_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    played(w.pc_cast(&id, &req.spell, req.slot, &req.targets, req.mode))
}

/// End of turn, rests, hit dice, death saves.
async fn player_do(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>, Json(req): Json<DoReq>) -> Response {
    let mut w = st.world.write().unwrap();
    let id = match seated(&mut w, &headers, &q, &req.device_id) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let done = |text: &str| Ok(dnd_core::play::ActionResult { text: text.to_string(), ..Default::default() });
    let r = match req.action.as_str() {
        "end_turn" => w.end_turn(&id).and_then(|_| done("turn ended")),
        "short_rest" => w.rest(&id, false).and_then(|_| done("short rest")),
        "long_rest" => w.rest(&id, true).and_then(|_| done("long rest")),
        "hit_die" => w.spend_hit_die(&id, &req.arg),
        "death_save" => w.death_save(&id),
        "dash" => w.dash(&id).and_then(|_| done("dash")),
        _ => Err(format!("unknown action '{}'", req.action)),
    };
    played(r)
}

async fn player_builder(State(st): State<AppState>) -> Json<BuilderOptions> {
    let w = st.world.read().unwrap();
    Json(dnd_core::builder::builder_options(&w.library, w.campaign.settings.player_start_level))
}

/// A player uploads the character they built. Allowed between sessions only;
/// the new character goes straight to the device that made it.
async fn player_character_create(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(req): Json<CreateReq>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "join code required");
    }
    let Some(player) = w.campaign.device(&req.device_id).map(|d| d.name.clone()) else {
        return err(StatusCode::NOT_FOUND, "register first");
    };
    if w.campaign.settings.game_live {
        return err(StatusCode::FORBIDDEN, "the game has started; the DM will give you a character");
    }
    let dev = w.campaign.device(&req.device_id).cloned().unwrap_or_default();
    // a player may rebuild the character they made, but not swap out one the DM gave them
    let replacing = match w.campaign.device_character(&req.device_id).map(|c| c.id.clone()) {
        Some(id) if id == dev.made => Some(id),
        Some(_) => return err(StatusCode::CONFLICT, "the DM gave you this character; ask the DM if you want to make your own"),
        None => None,
    };
    if replacing.is_none() && w.campaign.characters.len() >= 200 {
        return err(StatusCode::CONFLICT, "this campaign is full of characters; ask the DM");
    }
    let cx = dnd_core::builder::UploadContext {
        start_level: w.campaign.settings.player_start_level,
        rolled: &dev.rolled,
        gear: &req.gear,
        replacing: replacing.as_deref(),
    };
    let mut ch = match dnd_core::builder::prepare_player_character(&w.library, &w.campaign, &player, &req.character, &cx) {
        Ok(ch) => ch,
        Err(e) => return err(StatusCode::BAD_REQUEST, e),
    };
    // a rebuild keeps what the DM put on the old character
    if let Some(old) = replacing.as_deref().and_then(|id| w.campaign.character(id)) {
        ch.dm_notes = old.dm_notes.clone();
        ch.modifiers = old.modifiers.clone();
        ch.inventory.extend(old.inventory.iter().filter(|i| i.hidden || !i.dm_notes.is_empty()).cloned());
    }
    let (id, name) = (ch.id.clone(), ch.name.clone());
    if let Some(old) = &replacing {
        w.campaign.characters.retain(|c| c.id != *old);
        w.campaign.log(format!("'{player}' rebuilt their character as {name}"));
    } else {
        w.campaign.log(format!("'{player}' made the character {name}"));
    }
    w.campaign.characters.push(ch);
    w.campaign.assign_device(&req.device_id, Some(&id));
    if let Some(d) = w.campaign.devices.iter_mut().find(|d| d.id == req.device_id) {
        d.made = id.clone();
    }
    w.seen(&req.device_id);
    w.touch();
    (StatusCode::CREATED, Json(seat_of(&w, &req.device_id))).into_response()
}

/// The server rolls a player's ability scores (4d6, drop the lowest, six
/// times) so the DM can see them; one set per device until the DM clears it.
async fn player_roll_abilities(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(req): Json<RollAbilitiesReq>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "join code required");
    }
    let Some(dev) = w.campaign.device(&req.device_id).cloned() else {
        return err(StatusCode::NOT_FOUND, "register first");
    };
    if w.campaign.settings.game_live {
        return err(StatusCode::FORBIDDEN, "the game has started; the DM will give you a character");
    }
    if dev.rolled.is_empty() {
        let mut rolls = Vec::new();
        for _ in 0..6 {
            match w.roll("4d6kh3") {
                Ok(r) => rolls.push(r.int() as i32),
                Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            }
        }
        w.campaign.log(format!("'{}' rolled ability scores: {rolls:?}", dev.name));
        if let Some(d) = w.campaign.devices.iter_mut().find(|d| d.id == req.device_id) {
            d.rolled = rolls;
        }
        w.touch();
    }
    w.seen(&req.device_id);
    Json(seat_of(&w, &req.device_id)).into_response()
}

async fn dm_devices(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let w = st.world.read().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required");
    }
    let list: Vec<DeviceListed> = w
        .campaign
        .devices
        .iter()
        .map(|d| DeviceListed {
            id: d.id.clone(),
            name: d.name.clone(),
            character: w.campaign.device_character(&d.id).map(|c| c.id.clone()),
            online: w.device_online(&d.id),
            last_seen: d.last_seen.clone(),
        })
        .collect();
    Json(list).into_response()
}

#[derive(Deserialize)]
struct DeviceAssign {
    #[serde(default)]
    character: Option<String>,
}

async fn dm_device_assign(
    State(st): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<DeviceAssign>,
) -> Response {
    let mut w = st.world.write().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required");
    }
    let character = body.character.as_deref().filter(|c| !c.is_empty());
    if !w.campaign.assign_device(&id, character) {
        return err(StatusCode::NOT_FOUND, "no such device or character");
    }
    w.touch();
    Json(seat_of(&w, &id)).into_response()
}

async fn dm_device_forget(State(st): State<AppState>, Path(id): Path<String>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let mut w = st.world.write().unwrap();
    if !dm_authorized(&w, &headers, &q) {
        return err(StatusCode::UNAUTHORIZED, "dm code required");
    }
    if !w.campaign.forget_device(&id) {
        return err(StatusCode::NOT_FOUND, "no such device");
    }
    w.presence.remove(&id);
    w.touch();
    StatusCode::NO_CONTENT.into_response()
}

// ------------------------------------------------------------------ websocket

async fn ws_upgrade(ws: WebSocketUpgrade, State(st): State<AppState>, Query(q): Query<HashMap<String, String>>) -> Response {
    let device = q.get("device").cloned().filter(|d| !d.is_empty());
    ws.on_upgrade(move |socket| ws_loop(socket, st, device))
}

async fn ws_loop(mut socket: WebSocket, st: AppState, device: Option<String>) {
    let mut last = 0u64;
    let mut ticks = 0u32;
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(400));
    loop {
        tokio::select! {
            _ = tick.tick() => {
                // an open socket is the heartbeat for a registered device
                if let Some(d) = &device {
                    if ticks % 12 == 0 {
                        let mut w = st.world.write().unwrap();
                        if w.campaign.device(d).is_some() {
                            w.seen(d);
                        }
                    }
                    ticks = ticks.wrapping_add(1);
                }
                let rev = st.world.read().unwrap().revision;
                if rev != last {
                    last = rev;
                    let msg = format!("{{\"type\":\"revision\",\"revision\":{rev}}}");
                    if socket.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(t))) => {
                        if t.as_str() == "ping" {
                            let _ = socket.send(Message::Text("pong".into())).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}
