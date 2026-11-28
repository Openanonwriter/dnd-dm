pub mod connect;
pub mod map;
pub mod create;
pub mod pick;
pub mod sheet;
pub mod spells;

#[derive(Default)]
pub struct UiState {
    pub spell_filter: String,
    pub damage_amount: i32,
    pub show_feature: Option<String>,
    pub notes_field: usize,
    pub hp_input: String,
    /// Spell whose description is open.
    pub spell_open: String,
    /// The latest roll or action, shown large under the header for a few seconds: total, hit or miss, the words.
    pub result: Option<(i64, Option<bool>, String, std::time::Instant)>,
    /// Tab the hit point panel was opened on; it closes when the player moves to another.
    pub hp_open_on: Option<crate::app::Tab>,
    /// The hit point panel under the header is open.
    pub hp_open: bool,
    /// Which page the "More" button opened last (dice, features and notes, settings).
    pub more_tab: Option<crate::app::Tab>,
}

pub fn bonus(v: i32) -> String {
    dnd_core::expr::fmt_bonus(v as i64)
}
