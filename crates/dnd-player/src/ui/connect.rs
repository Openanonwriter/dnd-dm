use crate::app::{PlayerApp, Screen};
use crate::client::Cmd;
use crate::theme;

pub fn show(app: &mut PlayerApp, ctx: &egui::Context) {
    let (inset_top, inset_bottom) = crate::system_insets(ctx);
    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin { left: 18, right: 0, top: 18 + inset_top as i8, bottom: 18 + inset_bottom as i8 })).show(ctx, |ui| {
        theme::page_scroll(ui, egui::ScrollArea::vertical(), 18, |ui| {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("⬣").color(theme::ORANGE).size(48.0));
                ui.heading(theme::accent("dnd-player"));
                ui.label(theme::muted("Your character sheet, live from the DM's table."));
            });
            ui.add_space(20.0);

            let games = app.games_sorted();
            if !games.is_empty() {
                theme::section(ui, "Your games");
                for g in &games {
                    let current = g.server_url == app.config.server_url && g.campaign == app.config.campaign;
                    let frame = if current { theme::card_hi() } else { theme::card() };
                    frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let room = theme::Room::new(ui, ("game", g.key()));
                            ui.vertical(|ui| {
                                room.limit(ui);
                                ui.label(egui::RichText::new(&g.campaign).strong());
                                let who = match (g.character_name.is_empty(), g.player_name.is_empty()) {
                                    (false, false) => format!("{} · as {}", g.character_name, g.player_name),
                                    (false, true) => g.character_name.clone(),
                                    (true, false) => format!("no character yet · as {}", g.player_name),
                                    (true, true) => "no character yet".to_string(),
                                };
                                ui.label(theme::muted(who));
                                ui.label(theme::muted(g.server_url.trim_start_matches("http://")).small());
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add_enabled(!app.busy, theme::primary("Join")).clicked() {
                                    app.forget_armed = None;
                                    app.join_game(g);
                                }
                                room.note(ui);
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            let has_copy = app.store.load_cached_in(&g.key(), &g.character_id).is_some();
                            if has_copy && ui.button("Open without the DM").clicked() {
                                app.forget_armed = None;
                                app.open_game_offline(g);
                            }
                            let armed = app.forget_armed.as_ref().map(|(u, c)| u == &g.server_url && c == &g.campaign).unwrap_or(false);
                            if armed {
                                if ui.button(egui::RichText::new("Really forget? Tap again").color(theme::BAD)).clicked() {
                                    app.forget_armed = None;
                                    app.forget_game(g);
                                }
                            } else if ui.button("Forget").clicked() {
                                app.forget_armed = Some((g.server_url.clone(), g.campaign.clone()));
                            }
                        });
                    });
                }
                ui.label(theme::muted("One entry per DM and campaign. Forgetting one only removes it from this device; the DM still has your character.").small());
                theme::section(ui, "Join another game");
            }

            theme::section(ui, "Who are you?");
            ui.label(theme::muted("Your name. The DM sees it and hands this device your character."));
            ui.add(crate::theme::line_edit(&mut app.name_input).hint_text("e.g. Sam").char_limit(40).desired_width(f32::INFINITY).font(egui::TextStyle::Heading));

            theme::section(ui, "Find the DM on this Wi-Fi");
            ui.horizontal(|ui| {
                if ui.add_enabled(!app.busy, theme::primary("📡  Scan network")).clicked() {
                    app.scan_results.clear();
                    let _ = app.client.tx.send(Cmd::Scan);
                }
                if app.busy {
                    ui.spinner();
                }
                if let Some(ip) = crate::client::local_ipv4() {
                    ui.label(theme::muted(format!("you are {ip}")).small());
                }
            });
            let results = app.scan_results.clone();
            for (url, campaign) in results {
                theme::card_hi().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let room = theme::Room::new(ui, ("scan", &url));
                        ui.vertical(|ui| {
                            room.limit(ui);
                            ui.label(egui::RichText::new(&campaign).strong());
                            ui.label(theme::muted(&url).small());
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(theme::primary("Join")).clicked() {
                                app.url_input = url.clone();
                                app.connect();
                            }
                            room.note(ui);
                        });
                    });
                });
            }

            theme::section(ui, "Or type the address the DM shows");
            ui.label(theme::muted("Dashboard > “Players connect to”, e.g. 192.168.1.23:8420"));
            ui.add(crate::theme::line_edit(&mut app.url_input).hint_text("192.168.1.23:8420").desired_width(f32::INFINITY).font(egui::TextStyle::Heading));
            ui.horizontal(|ui| {
                ui.label(theme::muted("Join code"));
                ui.add(crate::theme::line_edit(&mut app.code_input).hint_text("if the DM set one").desired_width(160.0));
            });
            if !app.config.recent_servers.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(theme::muted("Recent:"));
                    let recent = app.config.recent_servers.clone();
                    for r in recent {
                        if ui.button(r.trim_start_matches("http://")).clicked() {
                            app.url_input = r;
                        }
                    }
                });
            }
            ui.add_space(6.0);
            if ui.add_enabled(!app.busy && !app.url_input.trim().is_empty() && !app.name_input.trim().is_empty(), theme::primary("Connect")).clicked() {
                app.connect();
            }

            if !app.status.is_empty() && app.status_at.elapsed().as_secs() < 20 {
                ui.add_space(8.0);
                ui.colored_label(if app.status_error { theme::BAD } else { theme::GOOD }, &app.status);
            }

            if app.builder.is_some() {
                ui.add_space(16.0);
                theme::section(ui, "Before the game");
                ui.label(theme::muted("Build a character now, even away from the table, and upload it when you next connect before the game starts."));
                if app.builder_bundled {
                    ui.label(theme::muted("No DM needed yet: this uses the standard species, classes and backgrounds. Your DM's campaign may add or leave out some.").small());
                }
                let label = if app.draft.ability_method.is_empty() { "✏  Create a character" } else { "✏  Continue your draft" };
                if ui.button(label).clicked() {
                    app.open_builder();
                }
            }

            if app.character.is_some() {
                ui.add_space(16.0);
                theme::section(ui, "Offline");
                if ui.button("Open last character (cached)").clicked() {
                    app.screen = Screen::Sheet;
                }
            }
            ui.add_space(24.0);
            ui.label(theme::muted("Both devices must be on the same Wi-Fi. If the scan finds nothing, the DM's computer may be blocking it: allow dnd-dm through the firewall (TCP port 8420).").small());
        });
    });
}
