//! Shown while this device has no character: either a waiting room (the DM
//! assigns characters) or a list to choose from (when the DM allows it).

use crate::app::{PlayerApp, Screen};
use crate::client::Cmd;
use crate::theme;

pub fn show(app: &mut PlayerApp, ctx: &egui::Context) {
    let (inset_top, inset_bottom) = crate::system_insets(ctx);
    let can_choose = app.can_choose();
    let can_create = app.can_create();
    let live = app.game_live();
    let my_name = app.seat.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| app.config.player_name.clone());
    egui::TopBottomPanel::top("pick_top").frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin { left: 12, right: 12, top: 12 + inset_top as i8, bottom: 12 })).show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("‹").clicked() {
                app.screen = Screen::Connect;
            }
            ui.heading(theme::accent(if can_choose { "Pick your character" } else { "Waiting for the DM" }));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⟳").clicked() {
                    let _ = app.client.tx.send(Cmd::Refresh);
                }
                let dot = if app.online { ("⏺", theme::GOOD) } else { ("○", theme::MUTED) };
                ui.label(egui::RichText::new(dot.0).color(dot.1));
            });
        });
        let campaign = app.info.as_ref().map(|i| i.campaign.clone()).unwrap_or_default();
        ui.label(theme::muted(format!("{my_name} · {campaign}")));
    });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin { left: 14, right: 14, top: 14, bottom: 14 + inset_bottom as i8 })).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if !app.status.is_empty() && app.status_at.elapsed().as_secs() < 8 {
                ui.colored_label(if app.status_error { theme::BAD } else { theme::GOOD }, &app.status);
            }
            if can_create {
                theme::card_hi().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new("Make your own character").strong().size(18.0));
                    ui.label(theme::muted("The game hasn't started, so you can build a character here and send it to the DM. It becomes yours straight away."));
                    ui.add_space(6.0);
                    let label = if app.draft.ability_method.is_empty() { "✏  Create a character" } else { "✏  Continue your draft" };
                    if ui.add(theme::primary(label)).clicked() {
                        app.open_builder();
                    }
                });
            } else if live {
                ui.label(theme::muted("The game is running, so new characters come from the DM."));
            }
            if !can_choose {
                ui.add_space(if can_create { 12.0 } else { 36.0 });
                ui.vertical_centered(|ui| {
                    if app.online {
                        ui.spinner();
                    }
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(format!("You're in, {my_name}.")).strong().size(22.0));
                    ui.add_space(4.0);
                    ui.label(theme::muted("The DM can see this device on the Dashboard under “Players at the table”."));
                    ui.label(theme::muted(if can_create { "Make a character above, or wait: your sheet opens here by itself if the DM gives you one." } else { "Your sheet opens here by itself as soon as they give you a character." }));
                    if !app.online {
                        ui.add_space(8.0);
                        ui.colored_label(theme::BAD, "Not connected right now; retrying…");
                    }
                    ui.add_space(18.0);
                    if ui.button("Wrong name? Change it").clicked() {
                        app.screen = Screen::Connect;
                    }
                });
                return;
            }
            if app.characters.is_empty() {
                ui.label(theme::muted("No characters yet. Ask the DM to create yours on the Characters tab; it appears here automatically."));
            }
            let list = app.characters.clone();
            for c in list {
                let r = theme::card()
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&c.name).strong().size(20.0));
                                ui.label(theme::muted(format!("{} · level {}{}", c.class_line, c.level, if c.player.is_empty() { String::new() } else { format!(" · {}", c.player) })));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(theme::accent("›").size(28.0));
                            });
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if r.clicked() {
                    app.claim_character(&c.id);
                }
            }
        });
    });
}
