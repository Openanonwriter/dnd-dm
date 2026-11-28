//! Application state and screen routing.

use crate::client::{Client, Cmd, Evt};
use crate::store::{Config, Store};
use crate::theme;
use crate::ui;
use dnd_core::api::*;
use dnd_core::expr::{self, Env};
use dnd_core::{Character, Rng, Sheet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Connect,
    Pick,
    Create,
    Sheet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Stats,
    Combat,
    Spells,
    Items,
    Notes,
    Party,
    Dice,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 8] = [Tab::Stats, Tab::Combat, Tab::Spells, Tab::Items, Tab::Notes, Tab::Party, Tab::Dice, Tab::Settings];
    pub fn icon(self) -> &'static str {
        match self {
            Tab::Stats => "👤",
            Tab::Combat => "⚔",
            Tab::Spells => "✨",
            Tab::Items => "🎒",
            Tab::Notes => "📜",
            Tab::Party => "🗺",
            Tab::Dice => "🎲",
            Tab::Settings => "⚙",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Tab::Stats => "Stats",
            Tab::Combat => "Combat",
            Tab::Spells => "Spells",
            Tab::Items => "Items",
            Tab::Notes => "Notes",
            Tab::Party => "Map",
            Tab::Dice => "Dice",
            Tab::Settings => "More",
        }
    }
}

/// Standard content for the character builder, made by `build.rs`.
const BUNDLED_BUILDER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/builder.json"));

pub struct PlayerApp {
    pub store: Store,
    pub config: Config,
    pub client: Client,
    pub screen: Screen,
    pub tab: Tab,
    pub info: Option<Info>,
    pub online: bool,
    pub busy: bool,
    pub status: String,
    pub status_error: bool,
    pub status_at: Instant,
    pub characters: Vec<CharListed>,
    pub character: Option<Character>,
    pub sheet: Option<Sheet>,
    pub synced: Option<Character>,
    pub encounter: Option<PlayerEncounter>,
    pub map: Option<dnd_core::view::MapView>,
    pub map_ui: ui::map::MapUi,
    pub last_edit: Option<Instant>,
    pub last_poll: Instant,
    pub scan_results: Vec<(String, String)>,
    pub url_input: String,
    pub code_input: String,
    pub name_input: String,
    /// What the server last said about this device: name, assigned character.
    pub seat: Option<Seat>,
    /// What a character can be built from (from the server, cached on disk).
    pub builder: Option<BuilderOptions>,
    /// The same, as a library, so the builder can show real numbers.
    pub builder_lib: Option<dnd_core::Library>,
    /// the builder is still on the content packed into the app, not a DM's list
    pub builder_bundled: bool,
    /// The character being built, and the copy last written to disk.
    pub draft: Character,
    pub draft_saved: Character,
    pub draft_saved_at: Instant,
    pub rng: Rng,
    pub dice_formula: String,
    pub dice_log: Vec<String>,
    pub ui_state: ui::UiState,
    /// What happened at the table lately, as the server tells it.
    pub table: Vec<dnd_core::TableEntry>,
    /// Normal, advantage or disadvantage for the next d20.
    pub mode: dnd_core::play::RollMode,
    /// Combatants picked as targets for attacks and spells.
    pub targets: Vec<String>,
    /// Development aid: save a picture of the window and exit.
    pub shot: crate::shot::Shot,
    /// a finger left the screen last frame; see `raw_input_hook`
    pointer_gone_pending: bool,
    /// Saved game the player tapped "Forget" on once (server address, campaign); a second tap deletes it.
    pub forget_armed: Option<(String, String)>,
    /// Tab to open on once the sheet is there (development aid).
    pub start_tab: Option<Tab>,
    /// Development aid: `DND_PLAYER_CREATE="species,class,background"`
    /// opens the builder with those picks (for screenshots).
    pub dev_create: Option<String>,
}

const EDIT_DEBOUNCE: Duration = Duration::from_millis(700);

impl PlayerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, data_dir: PathBuf) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut store = Store::new(data_dir);
        let mut config = store.load_config();
        // carry on in the game this device was last in
        if config.games.iter().any(|g| g.server_url == config.server_url && g.campaign == config.campaign) {
            store.set_scope(&crate::store::game_key(&config.server_url, &config.campaign));
        }
        if config.device_id.is_empty() {
            config.device_id = crate::store::new_device_id();
            store.save_config(&config);
        }
        cc.egui_ctx.set_zoom_factor(config.zoom.clamp(0.7, 1.8));
        let client = Client::spawn(cc.egui_ctx.clone());
        let mut app = Self {
            url_input: config.server_url.clone(),
            code_input: config.join_code.clone(),
            name_input: config.player_name.clone(),
            seat: None,
            builder: None,
            builder_lib: None,
            builder_bundled: false,
            draft: Character::default(),
            draft_saved: Character::default(),
            draft_saved_at: Instant::now(),
            store,
            config,
            client,
            screen: Screen::Connect,
            tab: Tab::Stats,
            info: None,
            online: false,
            busy: false,
            status: String::new(),
            status_error: false,
            status_at: Instant::now(),
            characters: Vec::new(),
            character: None,
            sheet: None,
            synced: None,
            encounter: None,
            map: None,
            map_ui: Default::default(),
            last_edit: None,
            last_poll: Instant::now(),
            scan_results: Vec::new(),
            rng: Rng::from_time(),
            dice_formula: "1d20".into(),
            dice_log: Vec::new(),
            ui_state: Default::default(),
            table: Vec::new(),
            mode: Default::default(),
            targets: std::env::var("DND_PLAYER_TARGET").ok().into_iter().collect(),
            shot: crate::shot::Shot::from_env(),
            pointer_gone_pending: false,
            forget_armed: None,
            dev_create: std::env::var("DND_PLAYER_CREATE").ok(),
            start_tab: std::env::var("DND_PLAYER_TAB").ok().and_then(|t| Tab::ALL.into_iter().find(|x| x.label().eq_ignore_ascii_case(&t))),
        };
        // the list a DM sent last time, or else the standard content packed into the app
        match app.store.load_builder() {
            Some(o) => app.set_builder(o),
            None => match serde_json::from_slice::<BuilderOptions>(BUNDLED_BUILDER) {
                Ok(o) => {
                    app.set_builder(o);
                    app.builder_bundled = true;
                }
                Err(e) => log::error!("built-in content does not load: {e}"),
            },
        }
        if let Some(d) = app.store.load_draft() {
            app.draft = d.clone();
            app.draft_saved = d;
        }
        // offline start: show the cached character straight away
        if !app.config.character_id.is_empty() {
            if let Some((c, s)) = app.store.load_cached(&app.config.character_id) {
                app.character = Some(c.clone());
                app.synced = Some(c);
                app.sheet = Some(s);
                app.screen = Screen::Sheet;
            }
        }
        if !app.config.server_url.is_empty() && !app.config.player_name.trim().is_empty() {
            app.connect();
        }
        app
    }

    pub fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = text.into();
        self.status_error = error;
        self.status_at = Instant::now();
    }

    fn connect_cmd(&self) -> Cmd {
        Cmd::Connect {
            url: self.config.server_url.clone(),
            code: self.config.join_code.clone(),
            device_id: self.config.device_id.clone(),
            name: self.config.player_name.clone(),
        }
    }

    pub fn connect(&mut self) {
        let name = self.name_input.trim().to_string();
        if name.is_empty() {
            self.set_status("Type your name first so the DM knows whose device this is", true);
            return;
        }
        self.name_input = name.clone();
        self.config.player_name = name;
        let url = crate::client::normalize_url(&self.url_input);
        self.url_input = url.clone();
        if url != self.config.server_url {
            // another server: which campaign it runs is not known until it answers
            self.config.campaign.clear();
        }
        self.config.server_url = url.clone();
        self.config.join_code = self.code_input.clone();
        if !self.config.recent_servers.contains(&url) {
            self.config.recent_servers.insert(0, url.clone());
            self.config.recent_servers.truncate(5);
        }
        self.persist();
        let _ = self.client.tx.send(self.connect_cmd());
    }

    /// Write the config, first noting the current character, name and join
    /// code in the saved game this device is in.
    pub fn persist(&mut self) {
        let (url, campaign) = (self.config.server_url.clone(), self.config.campaign.clone());
        if !campaign.is_empty() {
            let seat_name = self.seat.as_ref().filter(|s| s.character.as_deref() == Some(self.config.character_id.as_str())).map(|s| s.character_name.clone());
            let sheet_name = self.character.as_ref().filter(|c| c.id == self.config.character_id).map(|c| c.name.clone());
            if let Some(g) = self.config.games.iter_mut().find(|g| g.server_url == url && g.campaign == campaign) {
                g.join_code = self.config.join_code.clone();
                g.player_name = self.config.player_name.clone();
                if g.character_id != self.config.character_id {
                    g.character_id = self.config.character_id.clone();
                    g.character_name.clear();
                }
                if let Some(n) = seat_name.or(sheet_name).filter(|n| !n.is_empty()) {
                    g.character_name = n;
                }
            }
        }
        self.store.save_config(&self.config);
    }

    /// The server answered and runs `campaign`: switch to the saved game for
    /// this server and campaign, making one the first time.
    fn enter_game(&mut self, campaign: &str) {
        let url = self.config.server_url.clone();
        let key = crate::store::game_key(&url, campaign);
        let known = self.config.games.iter().position(|g| g.server_url == url && g.campaign == campaign);
        let idx = match known {
            Some(i) => i,
            None => {
                // the one game of a version that kept no list keeps its character
                let inherited = if self.config.games.is_empty() { self.config.character_id.clone() } else { String::new() };
                self.config.games.push(crate::store::Game { server_url: url.clone(), campaign: campaign.to_string(), character_id: inherited, ..Default::default() });
                self.config.games.len() - 1
            }
        };
        self.config.games[idx].last_used = crate::store::now_secs();
        let switched = self.store.scope != key || self.config.campaign != campaign;
        self.config.campaign = campaign.to_string();
        if switched {
            self.store.set_scope(&key);
            self.config.character_id = self.config.games[idx].character_id.clone();
            self.show_cached_character();
            if let Some(o) = self.store.load_builder() {
                self.set_builder(o);
                self.builder_bundled = false;
            }
        }
        self.persist();
    }

    /// Show the offline copy of `config.character_id` in the current game,
    /// or nothing when there is none; never another game's character.
    fn show_cached_character(&mut self) {
        let id = self.config.character_id.clone();
        if self.character.as_ref().map(|c| c.id == id).unwrap_or(false) && !id.is_empty() && self.sheet.is_some() {
            return;
        }
        self.last_edit = None;
        self.encounter = None;
        self.map = None;
        self.targets.clear();
        match self.store.load_cached(&id) {
            Some((c, s)) => {
                self.character = Some(c.clone());
                self.synced = Some(c);
                self.sheet = Some(s);
            }
            None => {
                self.character = None;
                self.synced = None;
                self.sheet = None;
            }
        }
    }

    /// Saved games, most recently used first.
    pub fn games_sorted(&self) -> Vec<crate::store::Game> {
        let mut g = self.config.games.clone();
        g.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        g
    }

    /// Make a saved game the current one (address, name, join code, character).
    fn select_game(&mut self, game: &crate::store::Game) {
        self.flush_edits(true);
        self.seat = None;
        self.online = false;
        self.info = None;
        self.config.server_url = game.server_url.clone();
        self.config.campaign = game.campaign.clone();
        self.config.join_code = game.join_code.clone();
        self.config.character_id = game.character_id.clone();
        self.url_input = game.server_url.clone();
        self.code_input = game.join_code.clone();
        if !game.player_name.is_empty() {
            self.name_input = game.player_name.clone();
        }
        self.store.set_scope(&game.key());
        self.show_cached_character();
        if let Some(o) = self.store.load_builder() {
            self.set_builder(o);
            self.builder_bundled = false;
        }
    }

    /// Connect to a saved game.
    pub fn join_game(&mut self, game: &crate::store::Game) {
        self.select_game(game);
        self.connect();
    }

    /// Open a saved game's character from the copy on this device, without the server.
    pub fn open_game_offline(&mut self, game: &crate::store::Game) {
        self.select_game(game);
        self.config.player_name = self.name_input.trim().to_string();
        self.persist();
        if self.character.is_some() {
            self.screen = Screen::Sheet;
            self.tab = Tab::Stats;
        } else {
            self.set_status("No copy of that character on this device yet", true);
        }
    }

    /// Remove a saved game and its offline files from this device.
    pub fn forget_game(&mut self, game: &crate::store::Game) {
        self.config.games.retain(|g| !(g.server_url == game.server_url && g.campaign == game.campaign));
        self.store.forget_game(&game.key());
        if self.config.server_url == game.server_url && self.config.campaign == game.campaign {
            self.config.campaign.clear();
            self.config.character_id.clear();
            self.store.scope.clear();
            self.character = None;
            self.synced = None;
            self.sheet = None;
            self.seat = None;
        }
        self.store.save_config(&self.config);
    }

    /// Ask the server for a character (pick screen, when the DM allows it).
    pub fn claim_character(&mut self, id: &str) {
        let _ = self.client.tx.send(Cmd::Claim(id.to_string()));
    }

    /// True when the player may choose from the list themselves.
    pub fn can_choose(&self) -> bool {
        self.seat.as_ref().map(|s| s.can_choose).unwrap_or(false)
    }

    fn set_builder(&mut self, o: BuilderOptions) {
        self.builder_lib = Some(dnd_core::builder::library_from_options(&o));
        self.builder = Some(o);
    }

    /// True when the server would accept a character from this device now.
    pub fn can_create(&self) -> bool {
        self.online && self.seat.as_ref().map(|s| s.can_create).unwrap_or(false)
    }

    pub fn game_live(&self) -> bool {
        self.seat.as_ref().map(|s| s.game_live).unwrap_or(false)
    }

    /// Open the character builder, starting a fresh draft if there is none.
    pub fn open_builder(&mut self) {
        if self.draft.ability_method.is_empty() {
            self.draft = Character::default();
            self.draft.ability_method = "standard_array".into();
            if let Some(o) = &self.builder {
                for (k, v) in dnd_core::ABILITY_KEYS.iter().zip(o.rules.standard_array.iter()) {
                    self.draft.base_abilities.set(k, *v);
                }
            }
        }
        self.screen = Screen::Create;
    }

    pub fn save_draft(&mut self) {
        if self.draft != self.draft_saved {
            self.store.save_draft(&self.draft);
            self.draft_saved = self.draft.clone();
        }
        self.draft_saved_at = Instant::now();
    }

    pub fn discard_draft(&mut self) {
        self.draft = Character::default();
        self.draft_saved = Character::default();
        self.store.clear_draft();
    }

    pub fn upload_draft(&mut self) {
        self.save_draft();
        let mut character = self.draft.clone();
        let gear = crate::ui::create::gear_choices(&character);
        character.custom.clear();
        let _ = self.client.tx.send(Cmd::Create { character: Box::new(character), gear });
    }

    /// Level that player-made characters start at in this campaign.
    pub fn start_level(&self) -> u32 {
        let from_seat = self.seat.as_ref().map(|s| s.start_level).filter(|l| *l > 0);
        from_seat.or(self.builder.as_ref().map(|b| b.start_level)).unwrap_or(1).clamp(1, 20)
    }

    /// True when the open character is one this player built and may rebuild now.
    pub fn can_rebuild(&self) -> bool {
        self.can_create() && self.seat.as_ref().map(|s| s.made_character).unwrap_or(false)
    }

    /// Open the builder filled in with the current character's choices.
    pub fn rebuild_character(&mut self) {
        let Some(c) = &self.character else { return };
        let mut d = Character::default();
        d.name = c.name.clone();
        d.species = c.species.clone();
        d.background = c.background.clone();
        d.classes = c.classes.iter().take(1).cloned().collect();
        d.base_abilities = c.base_abilities;
        d.ability_method = if c.ability_method.is_empty() { "standard_array".into() } else { c.ability_method.clone() };
        d.skill_proficiencies = c.skill_proficiencies.clone();
        d.advancements = c.advancements.clone();
        d.alignment = c.alignment.clone();
        d.appearance = c.appearance.clone();
        d.personality = c.personality.clone();
        d.ideals = c.ideals.clone();
        d.bonds = c.bonds.clone();
        d.flaws = c.flaws.clone();
        d.backstory = c.backstory.clone();
        d.notes = c.notes.clone();
        d.spells.known = c.spells.known.clone();
        d.spells.prepared = c.spells.prepared.clone();
        self.draft = d;
        self.screen = Screen::Create;
    }

    fn drop_character(&mut self) {
        self.config.character_id.clear();
        self.persist();
        self.character = None;
        self.synced = None;
        self.sheet = None;
        self.last_edit = None;
        let _ = self.client.tx.send(Cmd::Leave);
    }

    fn on_seat(&mut self, seat: Seat, fresh: bool) {
        self.online = true;
        let assigned = seat.character.clone();
        let assigned_name = seat.character_name.clone();
        self.seat = Some(seat);
        match assigned {
            Some(id) => {
                let changed = id != self.config.character_id;
                if changed || self.screen == Screen::Pick || (fresh && self.screen == Screen::Connect) {
                    if changed {
                        self.last_edit = None;
                        self.set_status(format!("You are playing {assigned_name}"), false);
                    }
                    self.open_character(&id);
                }
            }
            None => {
                if !self.config.character_id.is_empty() || self.character.is_some() {
                    self.drop_character();
                }
                if (fresh && self.screen != Screen::Create) || self.screen == Screen::Sheet {
                    self.screen = Screen::Pick;
                }
            }
        }
    }

    pub fn open_character(&mut self, id: &str) {
        self.config.character_id = id.to_string();
        self.persist();
        if let Some((c, s)) = self.store.load_cached(id) {
            self.character = Some(c.clone());
            self.synced = Some(c);
            self.sheet = Some(s);
        } else if self.character.as_ref().map(|c| c.id != id).unwrap_or(false) {
            // no cached copy: don't keep showing the previous character
            self.character = None;
            self.synced = None;
            self.sheet = None;
        }
        self.screen = Screen::Sheet;
        self.tab = Tab::Stats;
        let _ = self.client.tx.send(Cmd::LoadCharacter(id.to_string()));
    }

    /// Re-derive numbers locally is not possible without the library, so the
    /// sheet comes from the server; but HP/slots live on the character, so
    /// mirror those into the cached sheet for instant feedback.
    fn mirror_state_into_sheet(&mut self) {
        if let (Some(c), Some(s)) = (&self.character, self.sheet.as_mut()) {
            s.hp_current = c.hp.current.min(s.hp_max);
            s.hp_temp = c.hp.temp;
            let mut used = c.spells.slots_used.clone();
            used.resize(9, 0);
            s.spell_slots_used = used;
            s.conditions = c.conditions.clone();
            s.exhaustion = c.exhaustion;
            s.inspiration = c.inspiration;
            s.death_saves = c.death_saves.clone();
            s.currency = c.currency.clone();
            s.spells_known = c.spells.known.clone();
            s.spells_prepared = c.spells.prepared.clone();
            s.hit_dice_used = c.hp.hit_dice_used.clone();
        }
    }

    fn pump_events(&mut self) {
        while let Ok(evt) = self.client.rx.try_recv() {
            match evt {
                Evt::Busy(b) => self.busy = b,
                Evt::Connected(info) => {
                    self.online = true;
                    self.set_status(format!("Connected to '{}'", info.campaign), false);
                    self.enter_game(&info.campaign);
                    self.info = Some(info);
                    // which screen comes next is decided by the Seat that follows
                    if self.screen == Screen::Sheet && !self.config.character_id.is_empty() {
                        let _ = self.client.tx.send(Cmd::LoadCharacter(self.config.character_id.clone()));
                    }
                }
                Evt::Disconnected(e) => {
                    self.online = false;
                    self.set_status(format!("Can't reach server: {e}"), true);
                }
                Evt::Seat { seat, fresh } => self.on_seat(seat, fresh),
                Evt::ClaimFailed(e) => {
                    self.set_status(format!("Can't take that character: {e}"), true);
                    let _ = self.client.tx.send(Cmd::Refresh);
                }
                Evt::Builder(o) => {
                    self.store.save_builder(&o);
                    self.set_builder(*o);
                    self.builder_bundled = false;
                }
                Evt::Created(seat) => {
                    self.discard_draft();
                    self.screen = Screen::Pick;
                    self.on_seat(seat, true);
                }
                Evt::CreateFailed(e) => {
                    self.set_status(format!("Upload failed: {e}"), true);
                    let _ = self.client.tx.send(Cmd::Refresh);
                }
                Evt::Characters(list) => self.characters = list,
                Evt::Character { character, sheet } => {
                    // don't clobber edits the player is still typing
                    if self.last_edit.is_none() {
                        self.character = Some(character.clone());
                        self.synced = Some(character.clone());
                        self.store.cache_character(&character, &sheet);
                    } else {
                        self.synced = Some(character.clone());
                    }
                    self.sheet = Some(sheet);
                    self.mirror_state_into_sheet();
                    self.online = true;
                }
                Evt::Encounter(e) => self.encounter = Some(e),
                Evt::Map(m) => self.map = Some(*m),
                Evt::Walked(feet) => self.map_ui.last_walk = format!("You walked {feet} ft."),
                Evt::MapRefused(why) => {
                    // "409 Conflict: you cannot get there from here" -> the reason only
                    let why = why.rsplit(": ").next().unwrap_or(&why).to_string();
                    self.map_ui.last_walk = format!("Not possible: {why}");
                }
                Evt::Table(t) => self.table = t,
                Evt::Action(Ok(r)) => {
                    self.dice_log.insert(0, r.text.clone());
                    self.dice_log.truncate(30);
                    self.set_status(r.text, false);
                    self.mode = Default::default();
                }
                Evt::Action(Err(e)) => {
                    // "HTTP 409: it is not your turn" -> the reason only
                    let why = e.split_once(": ").map(|x| x.1.to_string()).unwrap_or(e);
                    self.set_status(why, true);
                }
                Evt::SpellsChosen(c) => {
                    if let Some(mine) = self.character.as_mut() {
                        mine.spells.known = c.spells.known.clone();
                        mine.spells.prepared.retain(|p| c.spells.prepared.contains(p));
                    }
                    if let Some(s) = self.synced.as_mut() {
                        s.spells.known = c.spells.known.clone();
                        s.spells.prepared.retain(|p| c.spells.prepared.contains(p));
                    }
                }
                Evt::Patched(c) => {
                    self.synced = Some(c);
                    self.set_status("Saved to DM", false);
                    let _ = self.client.tx.send(Cmd::Refresh);
                }
                Evt::ScanResult(list) => {
                    if list.is_empty() {
                        self.set_status("No dnd-dm server found on this Wi-Fi", true);
                    } else {
                        self.set_status(format!("Found {} server(s)", list.len()), false);
                        if list.len() == 1 {
                            self.url_input = list[0].0.clone();
                        }
                    }
                    self.scan_results = list;
                }
                Evt::Error(e) => {
                    if e.contains("polling") {
                        self.set_status(e, false);
                    } else {
                        if e.contains("load character") || e.contains("characters:") {
                            self.online = false;
                        }
                        self.set_status(e, true);
                    }
                }
            }
        }
    }

    fn flush_edits(&mut self, force: bool) {
        let (Some(c), Some(s)) = (&self.character, &self.synced) else { return };
        if c == s {
            self.last_edit = None;
            return;
        }
        if force || self.last_edit.map(|t| t.elapsed() >= EDIT_DEBOUNCE).unwrap_or(false) {
            let patch = PlayerPatch::diff(s, c);
            let id = c.id.clone();
            if let Some(sheet) = &self.sheet {
                self.store.cache_character(c, sheet);
            }
            if self.online {
                let _ = self.client.tx.send(Cmd::Patch { id, patch });
            } else {
                self.set_status("Offline: changes kept on this device", false);
            }
            self.synced = self.character.clone();
            self.last_edit = None;
        }
    }

    /// Do something through the server (attack, cast, rest...). Edits still
    /// waiting on this device go first so the two cannot cross.
    pub fn act(&mut self, cmd: Cmd) {
        if !self.online {
            self.set_status("Not connected to the DM", true);
            return;
        }
        self.flush_edits(true);
        let _ = self.client.tx.send(cmd);
    }

    /// The id of our own token in the fight, if we are in it.
    pub fn my_token(&self) -> Option<String> {
        if let Some(t) = self.map.as_ref().and_then(|m| m.tokens.iter().find(|t| t.kind == "pc" && t.id == self.config.character_id)) {
            return Some(t.id.clone());
        }
        self.encounter.as_ref().and_then(|e| e.combatants.iter().find(|c| c.is_pc && c.id == self.config.character_id)).map(|c| c.id.clone())
    }

    /// True while a fight is on and it is our character's turn.
    pub fn my_turn(&self) -> bool {
        let Some(me) = self.my_token() else { return false };
        self.encounter.as_ref().map(|e| e.active && e.combatants.iter().any(|c| c.is_current && c.id == me)).unwrap_or(false)
    }

    pub fn roll(&mut self, formula: &str, label: &str) {
        // at the table the DM's machine rolls, so everyone sees the same dice
        if self.online && self.character.is_some() && self.seat.as_ref().map(|s| s.character.is_some()).unwrap_or(false) {
            let mode = self.mode;
            self.act(Cmd::Roll { formula: formula.to_string(), label: label.to_string(), mode });
            return;
        }
        match expr::roll(formula, &Env::new(), &mut self.rng) {
            Ok(r) => {
                let detail: Vec<String> = r.rolls.iter().map(|d| d.to_string()).collect();
                let line = if label.is_empty() { format!("{formula} = {}", r.int()) } else { format!("{label}: {formula} = {}", r.int()) };
                let line = if detail.is_empty() { line } else { format!("{line}  [{}]", detail.join(", ")) };
                self.dice_log.insert(0, line);
                self.dice_log.truncate(30);
            }
            Err(e) => self.dice_log.insert(0, format!("{formula}: {e}")),
        }
    }
}

impl eframe::App for PlayerApp {
    /// Keeps flick scrolling alive on touch screens. When a finger lifts,
    /// egui-winit sends "button released" and "pointer gone" in the same
    /// frame; egui 0.31 forgets the pointer's speed on "pointer gone", so the
    /// scroll area sees speed zero at the moment of release and the page
    /// stops dead. Holding "pointer gone" back by one frame lets the scroll
    /// area read the speed first.
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        use egui::Event;
        if std::mem::take(&mut self.pointer_gone_pending) {
            let touching_again = raw_input.events.iter().any(|e| matches!(e, Event::PointerButton { pressed: true, .. } | Event::PointerMoved(_)));
            if !touching_again {
                raw_input.events.insert(0, Event::PointerGone);
            }
        }
        let released = raw_input.events.iter().rposition(|e| matches!(e, Event::PointerButton { pressed: false, .. }));
        if let Some(released) = released {
            let before = raw_input.events.len();
            let mut i = 0;
            raw_input.events.retain(|e| {
                i += 1;
                !(i - 1 > released && matches!(e, Event::PointerGone))
            });
            if raw_input.events.len() != before {
                self.pointer_gone_pending = true;
                ctx.request_repaint();
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));
        self.pump_events();

        // detect edits made this frame cycle
        if let (Some(c), Some(s)) = (&self.character, &self.synced) {
            if c != s && self.last_edit.is_none() {
                self.last_edit = Some(Instant::now());
            }
        }
        self.flush_edits(false);
        self.mirror_state_into_sheet();

        // polling fallback when the websocket isn't delivering, and automatic
        // reconnect when the DM's server went away and came back
        if self.screen == Screen::Create && self.draft != self.draft_saved && self.draft_saved_at.elapsed() > Duration::from_secs(2) {
            self.save_draft();
        }
        if self.screen != Screen::Connect && !self.config.server_url.is_empty() {
            let every = if self.online { Duration::from_secs(20) } else { Duration::from_secs(8) };
            if self.last_poll.elapsed() > every && !self.busy {
                self.last_poll = Instant::now();
                if self.online {
                    let _ = self.client.tx.send(Cmd::Refresh);
                } else {
                    let _ = self.client.tx.send(self.connect_cmd());
                }
            }
        }

        // DND_PLAYER_CREATE: once past the connect screen, or straight away on a device that has never had a server
        if self.builder.is_some() && (self.screen != Screen::Connect || self.config.server_url.is_empty()) {
            if let Some(v) = self.dev_create.take() {
                self.open_builder();
                let p: Vec<&str> = v.split(',').map(|s| s.trim()).collect();
                self.draft.name = "New hero".into();
                self.draft.species = p.first().map(|s| s.to_string()).unwrap_or_default();
                self.draft.classes = vec![dnd_core::ClassLevel { class: p.get(1).map(|s| s.to_string()).unwrap_or_default(), level: 1, ..Default::default() }];
                self.draft.background = p.get(2).map(|s| s.to_string()).unwrap_or_default();
            }
        }
        if self.screen == Screen::Sheet && self.sheet.is_some() && self.online {
            if let Some(t) = self.start_tab.take() {
                self.tab = t;
            }
        }
        self.shot.tick(ctx);
        match self.screen {
            Screen::Connect => ui::connect::show(self, ctx),
            Screen::Pick => ui::pick::show(self, ctx),
            Screen::Create => ui::create::show(self, ctx),
            Screen::Sheet => ui::sheet::show(self, ctx),
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush_edits(true);
        self.save_draft();
        self.persist();
        let _ = self.client.tx.send(Cmd::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device that has never reached a DM builds characters from this.
    #[test]
    fn built_in_content_loads() {
        let o: BuilderOptions = serde_json::from_slice(BUNDLED_BUILDER).expect("built-in builder content parses");
        assert!(!o.species.is_empty() && !o.classes.is_empty() && !o.backgrounds.is_empty());
        assert!(!o.rules.standard_array.is_empty());
        let lib = dnd_core::builder::library_from_options(&o);
        assert_eq!(lib.classes.len(), o.classes.len());
    }
}
