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
}

pub fn bonus(v: i32) -> String {
    dnd_core::expr::fmt_bonus(v as i64)
}
