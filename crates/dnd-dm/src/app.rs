use crate::paths::Paths;
use crate::ui;
use crate::ui::theme;
use dnd_core::{Kind, World};
use dnd_server::{ServerHandle, SharedWorld};
use eframe::egui;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Content,
    Characters,
    Encounter,
    Map,
    Campaign,
}

impl View {
    pub const ALL: [View; 6] = [View::Dashboard, View::Content, View::Characters, View::Encounter, View::Map, View::Campaign];
    pub fn label(self) -> &'static str {
        match self {
            View::Dashboard => "🏠  Dashboard",
            View::Content => "📚  Content",
            View::Characters => "🧙  Characters",
            View::Encounter => "⚔  Encounter",
            View::Map => "🗺  Map",
            View::Campaign => "💾  Campaign",
        }
    }
}

pub struct Status {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

pub struct DmApp {
    pub world: SharedWorld,
    pub paths: Paths,
    pub view: View,
    pub server: Option<ServerHandle>,
    pub server_error: String,
    pub ips: Vec<String>,
    pub status: Option<Status>,
    pub dice_formula: String,
    pub dice_log: Vec<String>,
    pub content: ui::content::ContentState,
    pub characters: ui::characters::CharState,
    pub encounter: ui::encounter::EncounterState,
    pub map: ui::map::MapViewState,
    pub campaign: ui::campaign::CampaignState,
    pub dashboard: ui::dashboard::DashState,
    // autosave
    pub seen_revision: u64,
    pub saved_revision: u64,
    pub last_change: Instant,
    pub autosave: bool,
}

pub const AUTOSAVE_DELAY: Duration = Duration::from_millis(1200);

impl DmApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Paths) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut world = World::new(paths.content_dirs.clone());
        world.campaign.settings.port = paths.config.port;
        // reopen the most recent campaign, if any
        if let Some((_, dir, _)) = dnd_core::save::list_campaigns(&paths.saves_dir).into_iter().next() {
            if let Ok(c) = dnd_core::save::load_campaign(&dir) {
                let port = world.campaign.settings.port;
                world.campaign = c;
                if world.campaign.settings.port == 0 {
                    world.campaign.settings.port = port;
                }
            }
        }
        let rev = world.revision;
        let world: SharedWorld = Arc::new(RwLock::new(world));
        let mut app = Self {
            world,
            paths,
            view: View::Dashboard,
            server: None,
            server_error: String::new(),
            ips: dnd_server::local_ips(),
            status: None,
            dice_formula: "1d20".into(),
            dice_log: Vec::new(),
            content: Default::default(),
            characters: Default::default(),
            encounter: Default::default(),
            map: Default::default(),
            campaign: Default::default(),
            dashboard: Default::default(),
            seen_revision: rev,
            saved_revision: rev,
            last_change: Instant::now(),
            autosave: true,
        };
        if app.paths.config.autostart_server {
            app.start_server();
        }
        app
    }

    pub fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = Some(Status { text: text.into(), error, at: Instant::now() });
    }

    pub fn start_server(&mut self) {
        if self.server.is_some() {
            return;
        }
        let port = self.world.read().unwrap().campaign.settings.port;
        match dnd_server::start(self.world.clone(), port) {
            Ok(h) => {
                self.server_error.clear();
                self.set_status(format!("Server listening on port {}", h.addr.port()), false);
                self.server = Some(h);
                self.ips = dnd_server::local_ips();
            }
            Err(e) => {
                self.server_error = format!("{e:#}");
                self.set_status(format!("Server failed: {e:#}"), true);
            }
        }
    }

    pub fn stop_server(&mut self) {
        if let Some(h) = self.server.take() {
            h.stop();
            self.set_status("Server stopped", false);
        }
    }

    pub fn roll_dice(&mut self) {
        let formula = self.dice_formula.trim().to_string();
        if formula.is_empty() {
            return;
        }
        let mut w = self.world.write().unwrap();
        match w.roll(&formula) {
            Ok(r) => {
                let detail: Vec<String> = r.rolls.iter().map(|d| d.to_string()).collect();
                let line = if detail.is_empty() { format!("{formula} = {}", r.int()) } else { format!("{formula} = {}   [{}]", r.int(), detail.join(", ")) };
                w.campaign.log(format!("DM rolled {line}"));
                self.dice_log.insert(0, line);
                self.dice_log.truncate(12);
            }
            Err(e) => {
                self.dice_log.insert(0, format!("{formula}: {e}"));
            }
        }
    }

    /// Save the campaign a moment after the last change.
    fn maybe_autosave(&mut self, force: bool) {
        let rev = self.world.read().unwrap().revision;
        if rev != self.seen_revision {
            self.seen_revision = rev;
            self.last_change = Instant::now();
        }
        if rev == self.saved_revision {
            return;
        }
        if force || (self.autosave && self.last_change.elapsed() >= AUTOSAVE_DELAY) {
            match ui::campaign::save_quiet(self) {
                Ok(_) => self.saved_revision = rev,
                Err(e) => {
                    self.set_status(format!("Autosave failed: {e:#}"), true);
                    self.saved_revision = rev; // don't retry every frame
                }
            }
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(10, 6))).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⬢ dnd-dm").color(theme::ORANGE).strong().size(18.0));
                ui.separator();
                let (name, rev, pcs) = {
                    let w = self.world.read().unwrap();
                    (w.campaign.name.clone(), w.revision, w.campaign.characters.len())
                };
                ui.label(egui::RichText::new(&name).strong().size(16.0));
                ui.label(theme::muted(format!("{pcs} PCs")));
                let unsaved = rev != self.saved_revision;
                if unsaved {
                    ui.label(egui::RichText::new("● saving…").color(theme::ORANGE).small());
                } else {
                    ui.label(egui::RichText::new("● saved").color(theme::GOOD).small());
                }
                ui.separator();
                match &self.server {
                    Some(h) => {
                        ui.colored_label(theme::GOOD, format!("● server :{}", h.addr.port()));
                        if ui.small_button("stop").clicked() {
                            self.stop_server();
                        }
                    }
                    None => {
                        ui.colored_label(theme::MUTED, "○ server off");
                        if ui.small_button("start").clicked() {
                            self.start_server();
                        }
                    }
                }
                ui.separator();
                ui.label(theme::accent("🎲"));
                let r = ui.add(egui::TextEdit::singleline(&mut self.dice_formula).desired_width(110.0).hint_text("1d20+5"));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.add(theme::primary("Roll")).clicked() || enter {
                    self.roll_dice();
                }
                if let Some(last) = self.dice_log.first() {
                    ui.label(egui::RichText::new(last).monospace());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(s) = &self.status {
                        if s.at.elapsed() < Duration::from_secs(8) {
                            let c = if s.error { theme::BAD } else { theme::GOOD };
                            ui.colored_label(c, &s.text);
                        }
                    }
                });
            });
        });
    }

    fn nav(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(160.0)
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(8, 10)))
            .show(ctx, |ui| {
                for v in View::ALL {
                    let selected = self.view == v;
                    let text = egui::RichText::new(v.label()).size(15.0).color(if selected { egui::Color32::BLACK } else { theme::TEXT });
                    let btn = egui::Button::new(text)
                        .fill(if selected { theme::ORANGE } else { egui::Color32::TRANSPARENT })
                        .min_size(egui::vec2(144.0, 34.0));
                    if ui.add(btn).clicked() {
                        self.view = v;
                    }
                }
                ui.add_space(12.0);
                theme::section(ui, "Library");
                let counts: Vec<(Kind, usize)> = {
                    let w = self.world.read().unwrap();
                    Kind::ALL.iter().map(|k| (*k, w.library.count(*k))).collect()
                };
                for (k, n) in counts {
                    ui.horizontal(|ui| {
                        ui.label(theme::muted(k.label()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(theme::accent(n.to_string()));
                        });
                    });
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(6.0);
                    ui.label(theme::muted(format!("v{}", env!("CARGO_PKG_VERSION"))).small());
                    ui.checkbox(&mut self.autosave, "Autosave").on_hover_text("Save the campaign ~1 s after every change");
                    if !self.dice_log.is_empty() {
                        ui.collapsing("Roll history", |ui| {
                            for l in &self.dice_log {
                                ui.label(egui::RichText::new(l).small().monospace());
                            }
                        });
                    }
                });
            });
    }
}

impl eframe::App for DmApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(400));
        self.maybe_autosave(false);
        self.top_bar(ctx);
        self.nav(ctx);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(12.0)).show(ctx, |ui| match self.view {
            View::Dashboard => ui::dashboard::show(self, ui),
            View::Content => ui::content::show(self, ui),
            View::Characters => ui::characters::show(self, ui),
            View::Encounter => ui::encounter::show(self, ui),
            View::Map => ui::map::show(self, ui),
            View::Campaign => ui::campaign::show(self, ui),
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.maybe_autosave(true);
        ui::content::flush(self);
        self.stop_server();
    }
}
