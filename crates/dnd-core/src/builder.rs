//! Player-made characters. A player builds a character in their app (between
//! sessions) and uploads it; the server never trusts the upload and rebuilds
//! a clean character from only the choices a player is allowed to make.

use crate::api::BuilderOptions;
use crate::library::Library;
use crate::model::*;
use std::collections::BTreeMap;

/// Longest free-text field a player may upload.
const TEXT_LIMIT: usize = 4000;

/// Everything the player app needs to offer the choices: the visible species,
/// classes, subclasses, backgrounds and feats, item names for starting gear,
/// plus the rules tables.
pub fn builder_options(lib: &Library, start_level: u32) -> BuilderOptions {
    BuilderOptions {
        species: lib.species.values().filter(|s| !s.hidden).cloned().collect(),
        classes: lib.classes.values().filter(|c| !c.hidden).cloned().collect(),
        subclasses: lib.subclasses.values().filter(|c| !c.hidden).cloned().collect(),
        backgrounds: lib.backgrounds.values().filter(|b| !b.hidden).cloned().collect(),
        feats: lib.feats.values().filter(|f| !f.hidden).cloned().collect(),
        items: item_names(lib),
        rules: lib.rules.clone(),
        start_level: start_level.clamp(1, 20),
        spells: lib.spells.values().filter(|s| !s.hidden).cloned().collect(),
        item_entries: lib.items.values().filter(|i| !i.hidden).cloned().collect(),
    }
}

/// Class options (fighting styles, invocations, metamagic, ancestries...)
/// are feats tagged `option`: they are picked through the feature that
/// offers them, never as a general feat at an ability score improvement.
pub fn is_option_feat(f: &Feat) -> bool {
    f.tags.iter().any(|t| t == "option")
}

/// The feats a feature's choice may pick from.
pub fn feature_options<'a>(lib: &'a Library, tag: &str) -> Vec<&'a Feat> {
    lib.feats.values().filter(|f| !f.hidden && f.tags.iter().any(|t| t == tag)).collect()
}

/// The character text field a background suggestion table fills.
pub fn suggestion_field<'a>(ch: &'a mut Character, fills: &str) -> Option<&'a mut String> {
    Some(match fills {
        "personality" => &mut ch.personality,
        "ideals" => &mut ch.ideals,
        "bonds" => &mut ch.bonds,
        "flaws" => &mut ch.flaws,
        "backstory" => &mut ch.backstory,
        "notes" => &mut ch.notes,
        _ => return None,
    })
}

/// Friendly name of a suggestion target field.
pub fn suggestion_field_label(fills: &str) -> &'static str {
    match fills {
        "personality" => "Personality",
        "ideals" => "Ideals",
        "bonds" => "Bonds",
        "flaws" => "Flaws",
        "backstory" => "Backstory",
        "notes" => "Notes",
        _ => "",
    }
}

/// True when the field already holds this suggestion (one per line).
pub fn has_suggestion(ch: &Character, fills: &str, line: &str) -> bool {
    let mut c = ch.clone();
    suggestion_field(&mut c, fills).map(|f| f.lines().any(|l| l.trim() == line.trim())).unwrap_or(false)
}

/// Add a suggestion to its field on a new line, or take it out again if
/// it's already there.
pub fn toggle_suggestion(ch: &mut Character, fills: &str, line: &str) {
    let Some(field) = suggestion_field(ch, fills) else { return };
    let line = line.trim();
    if field.lines().any(|l| l.trim() == line) {
        *field = field.lines().filter(|l| l.trim() != line).collect::<Vec<_>>().join("\n");
    } else {
        if !field.trim().is_empty() {
            field.push('\n');
        }
        field.push_str(line);
    }
}

/// Visible items, id -> name.
pub fn item_names(lib: &Library) -> BTreeMap<String, String> {
    lib.items.values().filter(|i| !i.hidden).map(|i| (i.id.clone(), i.name.clone())).collect()
}

/// The reverse of [`builder_options`]: a small library good enough to derive
/// a preview sheet on the player's device.
pub fn library_from_options(o: &BuilderOptions) -> Library {
    let mut lib = Library::new();
    lib.rules = o.rules.clone();
    for s in &o.species {
        lib.insert(Entry::Species(s.clone()), None);
    }
    for c in &o.classes {
        lib.insert(Entry::Class(c.clone()), None);
    }
    for c in &o.subclasses {
        lib.insert(Entry::Subclass(c.clone()), None);
    }
    for b in &o.backgrounds {
        lib.insert(Entry::Background(b.clone()), None);
    }
    for f in &o.feats {
        lib.insert(Entry::Feat(f.clone()), None);
    }
    for s in &o.spells {
        lib.insert(Entry::Spell(s.clone()), None);
    }
    for i in &o.item_entries {
        lib.insert(Entry::Item(i.clone()), None);
    }
    lib
}

fn clean_text(s: &str, limit: usize) -> String {
    s.trim().chars().filter(|c| !c.is_control() || *c == '\n').take(limit).collect()
}

/// Skills a class lets a new character choose from (every skill when the
/// class does not restrict them).
pub fn skill_options(class: &Class, rules: &Rules) -> Vec<String> {
    if class.skill_choices.from.is_empty() {
        rules.skills.iter().map(|s| s.key.clone()).collect()
    } else {
        class.skill_choices.from.clone()
    }
}

fn same_numbers(a: &[i32], b: &[i32]) -> bool {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    a.sort();
    b.sort();
    a == b
}

/// Check the ability scores against the chosen method. `rolled` is the set
/// the server rolled for this device (empty = not rolled yet). `Ok` or a
/// sentence the player can act on.
pub fn check_abilities(a: &Abilities, method: &str, rules: &Rules, rolled: &[i32]) -> Result<(), String> {
    let scores: Vec<i32> = a.iter().map(|(_, v)| v).collect();
    match method {
        "standard_array" => {
            if !same_numbers(&scores, &rules.standard_array) {
                return Err(format!("use each of {:?} exactly once", rules.standard_array));
            }
        }
        "point_buy" => {
            let pb = &rules.point_buy;
            if scores.iter().any(|s| *s < pb.min || *s > pb.max) {
                return Err(format!("point-buy scores must be {} to {}", pb.min, pb.max));
            }
            match rules.point_buy_cost(&scores) {
                Some(cost) if cost <= pb.budget => {}
                Some(cost) => return Err(format!("that costs {cost} points; the budget is {}", pb.budget)),
                None => return Err("those scores are not on the point-buy table".into()),
            }
        }
        "rolled" => {
            if rolled.len() != 6 {
                return Err("roll your scores first (the DM's server rolls them)".into());
            }
            if !same_numbers(&scores, rolled) {
                return Err(format!("use each of your rolls {rolled:?} exactly once"));
            }
        }
        _ => return Err("pick standard array, point buy or rolled".into()),
    }
    Ok(())
}

/// The levels (up to `level`) at which this class improves an ability or takes a feat.
pub fn improvement_levels(class: &Class, level: u32) -> Vec<u32> {
    let mut v: Vec<u32> = class.asi_levels.iter().copied().filter(|l| *l <= level).collect();
    v.sort();
    v.dedup();
    v
}

/// Subclasses a character of this class and level must choose from (empty
/// when the class has not reached its subclass level, or has none).
pub fn subclass_options<'a>(class: &Class, level: u32, all: impl Iterator<Item = &'a Subclass>) -> Vec<&'a Subclass> {
    if class.subclass_level == 0 || level < class.subclass_level {
        return Vec::new();
    }
    all.filter(|s| s.class == class.id && !s.hidden).collect()
}

// ---------------------------------------------------------------- gear

/// One thing in a starting-equipment option.
#[derive(Debug, Clone, PartialEq)]
pub struct GearItem {
    /// The words from the content file, without the quantity.
    pub text: String,
    /// Library item id when the words name a known item.
    pub item: Option<String>,
    pub qty: u32,
    /// Set (to `gp`, `sp`...) when this is money rather than an item.
    pub coins: Option<String>,
}

/// One way to take a line of starting equipment ("chain mail" *or* "leather armor + longbow").
#[derive(Debug, Clone, PartialEq)]
pub struct GearOption {
    pub label: String,
    pub items: Vec<GearItem>,
}

/// Starting-equipment lines for a class and background, in a fixed order.
pub fn gear_lines(class: &Class, background: Option<&Background>) -> Vec<String> {
    let mut v = class.starting_equipment.clone();
    if let Some(b) = background {
        v.extend(b.starting_equipment.iter().cloned());
    }
    v
}

fn split_all(parts: Vec<String>, sep: &str) -> Vec<String> {
    parts.iter().flat_map(|p| p.split(sep).map(|s| s.trim().to_string()).collect::<Vec<_>>()).filter(|s| !s.is_empty()).collect()
}

fn resolve_item(text: &str, items: &BTreeMap<String, String>) -> Option<String> {
    let lower = text.trim().to_lowercase();
    let slug = Character::slug(&lower);
    let hit = |l: &str, s: &str| items.iter().find(|(id, name)| id.as_str() == s || name.to_lowercase() == l).map(|(id, _)| id.clone());
    hit(&lower, &slug).or_else(|| {
        let (l, s) = (lower.strip_suffix('s')?, slug.strip_suffix('s').unwrap_or(&slug));
        hit(l, s)
    })
}

fn gear_item(text: &str, items: &BTreeMap<String, String>) -> GearItem {
    let text = text.trim().trim_matches(',').trim();
    let text = text.strip_prefix("and ").unwrap_or(text).trim();
    let (first, rest) = text.split_once(' ').unwrap_or((text, ""));
    let words = [("a", 1), ("an", 1), ("one", 1), ("two", 2), ("three", 3), ("four", 4), ("five", 5), ("six", 6), ("ten", 10), ("twenty", 20)];
    let qty = first.parse::<u32>().ok().or_else(|| words.iter().find(|(w, _)| *w == first.to_lowercase()).map(|(_, n)| *n));
    let (qty, name) = match qty {
        Some(n) if !rest.trim().is_empty() => (n.clamp(1, 9999), rest.trim()),
        _ => (1, text),
    };
    let coin = name.to_lowercase();
    if first.parse::<u32>().is_ok() && ["cp", "sp", "ep", "gp", "pp"].contains(&coin.as_str()) {
        return GearItem { text: name.to_string(), item: None, qty, coins: Some(coin) };
    }
    // "two handaxes" is two of an item; "two martial weapons" stays words for the DM to settle
    match resolve_item(name, items) {
        Some(id) => GearItem { text: name.to_string(), item: Some(id), qty, coins: None },
        None => match resolve_item(text, items) {
            Some(id) => GearItem { text: text.to_string(), item: Some(id), qty: 1, coins: None },
            None => GearItem { text: name.to_string(), item: None, qty, coins: None },
        },
    }
}

/// The ways one starting-equipment line can be taken. Options are separated
/// by "or"; the things inside an option by "+", "and" or commas. Words that
/// name a library item (by id or name) become that item, anything else is
/// kept as a free-text item for the DM to settle.
pub fn gear_options(line: &str, items: &BTreeMap<String, String>) -> Vec<GearOption> {
    // "A, B or C" is a three-way choice; without an "or" the commas list things you get together
    let choice = line.contains(" or ");
    let mut options = vec![line.replace(", or ", " or ")];
    if choice {
        options = split_all(split_all(options, " or "), ", ");
    }
    let seps: &[&str] = if choice { &[" + ", " and "] } else { &[" + ", " and ", ", "] };
    options
        .into_iter()
        .map(|label| {
            let mut parts = vec![label.clone()];
            for sep in seps {
                parts = split_all(parts, sep);
            }
            GearOption { items: parts.iter().map(|p| gear_item(p, items)).collect(), label }
        })
        .collect()
}

/// Build the starting inventory and money. `choices[i]` picks the option of
/// line `i` (missing or out of range = the first).
pub fn starting_gear(lines: &[String], choices: &[usize], items: &BTreeMap<String, String>) -> (Vec<InventoryItem>, Currency) {
    let mut inv: Vec<InventoryItem> = Vec::new();
    let mut money = Currency::default();
    for (i, line) in lines.iter().enumerate() {
        let options = gear_options(line, items);
        let Some(opt) = options.get(choices.get(i).copied().unwrap_or(0)).or(options.first()) else { continue };
        for g in &opt.items {
            if let Some(c) = &g.coins {
                match c.as_str() {
                    "cp" => money.cp += g.qty,
                    "sp" => money.sp += g.qty,
                    "ep" => money.ep += g.qty,
                    "pp" => money.pp += g.qty,
                    _ => money.gp += g.qty,
                }
                continue;
            }
            let (key, custom) = match &g.item {
                Some(id) => (id.clone(), false),
                None => (g.text.clone(), true),
            };
            if let Some(have) = inv.iter_mut().find(|x| x.item == key && x.custom == custom) {
                have.qty += g.qty;
            } else {
                inv.push(InventoryItem { item: key, name: if custom { g.text.clone() } else { String::new() }, qty: g.qty, custom, ..Default::default() });
            }
        }
    }
    (inv, money)
}

// ---------------------------------------------------------------- upload

/// What the server knows about an upload besides the character itself.
#[derive(Debug, Clone, Default)]
pub struct UploadContext<'a> {
    /// Level new characters start at in this campaign.
    pub start_level: u32,
    /// Ability scores the server rolled for this device (empty = none).
    pub rolled: &'a [i32],
    /// Starting-equipment choice per line, see [`starting_gear`].
    pub gear: &'a [usize],
    /// Id of the character this upload replaces; its id may be reused.
    pub replacing: Option<&'a str>,
}

/// Turn a player's upload into a character the campaign can accept: the
/// campaign's starting level, known species / class / subclass / background,
/// legal ability scores, skills and improvements, gear from the class's own
/// starting equipment, and nothing a player may not grant themselves
/// (modifiers, extra items, XP, DM notes). The id is unique in `campaign`.
pub fn prepare_player_character(lib: &Library, campaign: &Campaign, player: &str, up: &Character, cx: &UploadContext) -> Result<Character, String> {
    let level = cx.start_level.clamp(1, 20);
    let name = clean_text(&up.name, 60).replace('\n', " ");
    if name.is_empty() {
        return Err("give your character a name".into());
    }
    let species = lib.species.get(&up.species).filter(|s| !s.hidden).ok_or("pick a species")?;
    let up_class = up.classes.first();
    let class = lib.classes.get(up_class.map(|c| c.class.as_str()).unwrap_or("")).filter(|c| !c.hidden).ok_or("pick a class")?;
    let background = if up.background.is_empty() { None } else { Some(lib.backgrounds.get(&up.background).filter(|b| !b.hidden).ok_or("that background is not available")?) };
    check_abilities(&up.base_abilities, &up.ability_method, &lib.rules, cx.rolled)?;

    let subs = subclass_options(class, level, lib.subclasses.values());
    let subclass = if subs.is_empty() {
        None
    } else {
        let want = up_class.and_then(|c| c.subclass.clone()).unwrap_or_default();
        Some(subs.iter().find(|s| s.id == want).ok_or_else(|| format!("pick a {}", if class.subclass_label.is_empty() { "subclass" } else { &class.subclass_label }))?.id.clone())
    };

    let allowed = skill_options(class, &lib.rules);
    let mut skills: Vec<String> = Vec::new();
    for s in &up.skill_proficiencies {
        if !allowed.contains(s) || lib.rules.skill(s).is_none() {
            return Err(format!("{} can't choose the skill '{s}'", class.name));
        }
        if !skills.contains(s) {
            skills.push(s.clone());
        }
    }
    if skills.len() > class.skill_choices.count as usize {
        return Err(format!("{} chooses {} skill(s)", class.name, class.skill_choices.count));
    }

    let levels = improvement_levels(class, level);
    let mut advancements: Vec<Advancement> = Vec::new();
    for a in &up.advancements {
        if !levels.contains(&a.level) || advancements.iter().any(|x| x.level == a.level) {
            return Err(format!("no improvement is due at level {}", a.level));
        }
        let adv = match &a.feat {
            Some(f) => {
                let feat = lib.feats.get(f).filter(|x| !x.hidden && !is_option_feat(x)).ok_or("that feat is not available")?;
                if !a.ability_increases.is_empty() {
                    return Err(format!("level {}: take the feat or the ability increase, not both", a.level));
                }
                Advancement { level: a.level, feat: Some(feat.id.clone()), ..Default::default() }
            }
            None => {
                let ok = a.ability_increases.iter().all(|(k, v)| ABILITY_KEYS.contains(&k.as_str()) && (1..=2).contains(v));
                if !ok || a.ability_increases.values().sum::<i32>() != 2 {
                    return Err(format!("level {}: raise one ability by 2 or two abilities by 1", a.level));
                }
                Advancement { level: a.level, ability_increases: a.ability_increases.clone(), feat: None }
            }
        };
        advancements.push(adv);
    }
    advancements.sort_by_key(|a| a.level);

    let base = {
        let s = Character::slug(&name);
        if s.is_empty() {
            "character".to_string()
        } else {
            s
        }
    };
    let mut id = base.clone();
    let mut n = 2;
    while campaign.character(&id).is_some() && cx.replacing != Some(id.as_str()) {
        id = format!("{base}_{n}");
        n += 1;
    }

    let mut ch = Character::new(id, name);
    ch.player = player.to_string();
    ch.species = species.id.clone();
    ch.background = background.map(|b| b.id.clone()).unwrap_or_default();
    ch.classes = vec![ClassLevel { class: class.id.clone(), subclass, level, ..Default::default() }];
    ch.base_abilities = up.base_abilities;
    ch.ability_method = up.ability_method.clone();
    ch.skill_proficiencies = skills;
    ch.advancements = advancements;
    // choices: only for features this character has, only from that
    // feature's own options, and no more picks than it allows
    let sheet = crate::sheet::derive(&ch, lib);
    for f in sheet.features.iter().filter(|f| !f.options.is_empty()) {
        let mut picked: Vec<String> = Vec::new();
        for id in up.choices.get(&f.name).map(|v| v.as_slice()).unwrap_or(&[]) {
            let ok = lib.feats.get(id).map(|x| !x.hidden && x.tags.iter().any(|t| t == &f.options)).unwrap_or(false);
            if !ok {
                return Err(format!("{}: '{id}' is not one of its options", f.name));
            }
            if !picked.contains(id) {
                picked.push(id.clone());
            }
        }
        if picked.len() as u32 > f.picks {
            return Err(format!("{}: pick at most {}", f.name, f.picks));
        }
        if !picked.is_empty() {
            ch.choices.insert(f.name.clone(), picked);
        }
    }
    ch.xp = lib.rules.xp_thresholds.get(level as usize - 1).copied().unwrap_or(0);
    let (inventory, currency) = starting_gear(&gear_lines(class, background), cx.gear, &item_names(lib));
    ch.inventory = inventory;
    ch.currency = currency;
    ch.alignment = clean_text(&up.alignment, 40).replace('\n', " ");
    ch.appearance = clean_text(&up.appearance, TEXT_LIMIT);
    ch.personality = clean_text(&up.personality, TEXT_LIMIT);
    ch.ideals = clean_text(&up.ideals, TEXT_LIMIT);
    ch.bonds = clean_text(&up.bonds, TEXT_LIMIT);
    ch.flaws = clean_text(&up.flaws, TEXT_LIMIT);
    ch.backstory = clean_text(&up.backstory, TEXT_LIMIT);
    ch.notes = clean_text(&up.notes, TEXT_LIMIT);
    ch.hp.current = crate::sheet::derive(&ch, lib).hp_max;
    // the DM may allow rule breaks for everyone, or for the character being rebuilt
    let old = cx.replacing.and_then(|r| campaign.character(r));
    let free = campaign.settings.allow_rule_breaks || old.map(|c| c.allow_rule_breaks).unwrap_or(false);
    ch.allow_rule_breaks = old.map(|c| c.allow_rule_breaks).unwrap_or(false);
    // spells: the ones the class lets them learn, then what they prepared
    crate::play::choose_spells(lib, &mut ch, &up.spells.known, false, free)?;
    let sheet = crate::sheet::derive(&ch, lib);
    let available = crate::play::spells_available(lib, &ch, &sheet);
    let room: i32 = sheet.spellcasting.iter().filter_map(|b| b.prepared_max).sum();
    for p in &up.spells.prepared {
        let leveled = lib.spells.get(p).map(|s| s.level > 0).unwrap_or(false);
        if leveled && available.contains(p) && !ch.spells.prepared.contains(p) && (ch.spells.prepared.len() as i32) < room {
            ch.spells.prepared.push(p.clone());
        }
    }
    // a player may only take what the character qualifies for
    if !free {
        use crate::requirements::Area;
        if let Some(p) = crate::requirements::check(lib, &ch).into_iter().find(|p| matches!(p.area, Area::Choice | Area::Feat | Area::Spell)) {
            return Err(format!("{} (ask your DM to allow it)", p.message));
        }
    }
    Ok(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> Library {
        let mut lib = Library::new();
        lib.insert(Entry::Species(Species { id: "elf".into(), name: "Elf".into(), speed: 30, ..Default::default() }), None);
        lib.insert(Entry::Species(Species { id: "lich".into(), name: "Lich".into(), hidden: true, ..Default::default() }), None);
        lib.insert(
            Entry::Class(Class {
                id: "rogue".into(),
                name: "Rogue".into(),
                hit_die: 8,
                subclass_level: 3,
                asi_levels: vec![4, 8],
                skill_choices: SkillChoice { count: 2, from: vec!["stealth".into(), "acrobatics".into(), "deception".into()] },
                starting_equipment: vec!["rapier or shortsword".into(), "shortbow + 20 arrows, or two daggers".into(), "burglar's pack and thieves' tools".into(), "15 gp".into()],
                ..Default::default()
            }),
            None,
        );
        lib.insert(Entry::Subclass(Subclass { id: "thief".into(), name: "Thief".into(), class: "rogue".into(), ..Default::default() }), None);
        lib.insert(Entry::Feat(Feat { id: "lucky".into(), name: "Lucky".into(), ..Default::default() }), None);
        for (id, name) in [("rapier", "Rapier"), ("shortsword", "Shortsword"), ("shortbow", "Shortbow"), ("arrow", "Arrow"), ("dagger", "Dagger"), ("thieves_tools", "Thieves' Tools")] {
            lib.insert(Entry::Item(Item { id: id.into(), name: name.into(), ..Default::default() }), None);
        }
        lib
    }

    fn upload() -> Character {
        let mut c = Character::new("", "  Thia Moonwhisper ");
        c.species = "elf".into();
        c.classes = vec![ClassLevel { class: "rogue".into(), level: 20, ..Default::default() }];
        c.base_abilities = Abilities { str: 8, dex: 15, con: 13, int: 12, wis: 10, cha: 14 };
        c.ability_method = "standard_array".into();
        c.skill_proficiencies = vec!["stealth".into(), "deception".into()];
        c
    }

    fn cx(level: u32) -> UploadContext<'static> {
        UploadContext { start_level: level, ..Default::default() }
    }

    #[test]
    fn upload_is_rebuilt_from_allowed_choices_only() {
        let lib = lib();
        let mut camp = Campaign::new("T");
        let mut up = upload();
        up.dm_notes = "I am secretly a god".into();
        up.xp = 300_000;
        up.modifiers = vec![Modifier::new("ac", ModOp::Add, "10")];
        up.feats = vec!["lucky".into()];
        up.inventory = vec![InventoryItem { item: "vorpal_sword".into(), qty: 1, ..Default::default() }];
        up.hp.max_override = Some(999);
        let ch = prepare_player_character(&lib, &camp, "Sam", &up, &cx(1)).unwrap();
        assert_eq!(ch.id, "thia_moonwhisper");
        assert_eq!(ch.name, "Thia Moonwhisper");
        assert_eq!(ch.player, "Sam");
        assert_eq!(ch.total_level(), 1);
        assert!(ch.modifiers.is_empty() && ch.feats.is_empty() && ch.dm_notes.is_empty());
        assert!(!ch.inventory.iter().any(|i| i.item == "vorpal_sword"));
        assert_eq!(ch.xp, 0);
        assert_eq!(ch.hp.max_override, None);
        assert_eq!(ch.hp.current, 9, "d8 + con 13");
        // a second upload with the same name gets its own id, unless it replaces the first
        camp.characters.push(ch);
        assert_eq!(prepare_player_character(&lib, &camp, "Kim", &upload(), &cx(1)).unwrap().id, "thia_moonwhisper_2");
        let again = UploadContext { start_level: 1, replacing: Some("thia_moonwhisper"), ..Default::default() };
        assert_eq!(prepare_player_character(&lib, &camp, "Sam", &upload(), &again).unwrap().id, "thia_moonwhisper");
    }

    #[test]
    fn feature_choices_are_checked() {
        let mut lib = lib();
        let mut rogue = lib.classes.get("rogue").unwrap().clone();
        rogue.features.push(Feature { name: "Knack".into(), level: 1, options: "knack".into(), choose: "1".into(), ..Default::default() });
        lib.insert(Entry::Class(rogue), None);
        let tagged = |id: &str| Feat { id: id.into(), name: id.into(), tags: vec!["option".into(), "knack".into()], ..Default::default() };
        lib.insert(Entry::Feat(tagged("quick")), None);
        lib.insert(Entry::Feat(tagged("sly")), None);
        let camp = Campaign::new("T");
        let with = |picks: &[&str]| {
            let mut up = upload();
            up.choices.insert("Knack".into(), picks.iter().map(|s| s.to_string()).collect());
            up.choices.insert("Not a feature".into(), vec!["quick".into()]);
            prepare_player_character(&lib, &camp, "Sam", &up, &cx(1))
        };
        let ch = with(&["quick"]).unwrap();
        assert_eq!(ch.choices.get("Knack"), Some(&vec!["quick".to_string()]));
        assert!(!ch.choices.contains_key("Not a feature"), "picks for features the character lacks are dropped");
        assert!(with(&["quick", "sly"]).is_err(), "too many picks");
        assert!(with(&["lucky"]).is_err(), "a feat without the tag is not an option");
        // an option feat can't be taken as a general feat at an improvement
        let mut up = upload();
        up.advancements = vec![Advancement { level: 4, feat: Some("quick".into()), ..Default::default() }];
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &cx(4)).is_err());
    }

    #[test]
    fn illegal_choices_are_refused() {
        let lib = lib();
        let camp = Campaign::new("T");
        let bad = |f: &dyn Fn(&mut Character)| {
            let mut c = upload();
            f(&mut c);
            prepare_player_character(&lib, &camp, "Sam", &c, &cx(1)).is_err()
        };
        assert!(bad(&|c| c.name = "  ".into()));
        assert!(bad(&|c| c.species = "lich".into()), "hidden content is not for players");
        assert!(bad(&|c| c.classes.clear()));
        assert!(bad(&|c| c.background = "noble".into()));
        assert!(bad(&|c| c.base_abilities.str = 18), "not the standard array");
        assert!(bad(&|c| c.skill_proficiencies = vec!["arcana".into()]), "not a rogue skill here");
        assert!(bad(&|c| c.skill_proficiencies = vec!["stealth".into(), "deception".into(), "acrobatics".into()]));
        assert!(bad(&|c| {
            c.ability_method = "point_buy".into();
            c.base_abilities = Abilities { str: 15, dex: 15, con: 15, int: 15, wis: 15, cha: 15 };
        }));
        assert!(!bad(&|c| {
            c.ability_method = "point_buy".into();
            c.base_abilities = Abilities { str: 15, dex: 15, con: 15, int: 8, wis: 8, cha: 8 };
        }));
        assert!(bad(&|c| c.ability_method = "manual".into()));
        assert!(bad(&|c| c.advancements = vec![Advancement { level: 4, feat: Some("lucky".into()), ..Default::default() }]), "level 1 has no improvement");
    }

    #[test]
    fn rolled_scores_must_be_the_servers_rolls() {
        let lib = lib();
        let camp = Campaign::new("T");
        let mut up = upload();
        up.ability_method = "rolled".into();
        up.base_abilities = Abilities { str: 18, dex: 18, con: 18, int: 18, wis: 18, cha: 18 };
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &cx(1)).is_err(), "nothing rolled yet");
        let rolls = [16, 9, 12, 14, 11, 13];
        let with = UploadContext { start_level: 1, rolled: &rolls, ..Default::default() };
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &with).is_err(), "made-up numbers");
        up.base_abilities = Abilities { str: 9, dex: 16, con: 14, int: 12, wis: 11, cha: 13 };
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &with).is_ok());
    }

    #[test]
    fn higher_start_level_needs_subclass_and_takes_improvements() {
        let lib = lib();
        let camp = Campaign::new("T");
        let mut up = upload();
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &cx(5)).is_err(), "level 5 rogue needs a subclass");
        up.classes[0].subclass = Some("thief".into());
        up.advancements = vec![Advancement { level: 4, ability_increases: [("dex".to_string(), 2)].into_iter().collect(), feat: None }];
        let ch = prepare_player_character(&lib, &camp, "Sam", &up, &cx(5)).unwrap();
        assert_eq!(ch.total_level(), 5);
        assert_eq!(ch.classes[0].subclass.as_deref(), Some("thief"));
        assert_eq!(crate::sheet::derive(&ch, &lib).abilities.dex, 17);
        assert_eq!(ch.xp, lib.rules.xp_thresholds[4]);
        // a feat instead, and nonsense is refused
        up.advancements = vec![Advancement { level: 4, feat: Some("lucky".into()), ..Default::default() }];
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &cx(5)).is_ok());
        up.advancements = vec![Advancement { level: 4, ability_increases: [("dex".to_string(), 5)].into_iter().collect(), feat: None }];
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &cx(5)).is_err());
        up.advancements = vec![Advancement { level: 8, feat: Some("lucky".into()), ..Default::default() }];
        assert!(prepare_player_character(&lib, &camp, "Sam", &up, &cx(5)).is_err(), "level 8 is not reached");
    }

    #[test]
    fn starting_gear_comes_from_the_class_lines() {
        let lib = lib();
        let camp = Campaign::new("T");
        let names = item_names(&lib);
        let opts = gear_options("shortbow + 20 arrows, or two daggers", &names);
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].items.len(), 2);
        assert_eq!((opts[0].items[1].item.as_deref(), opts[0].items[1].qty), (Some("arrow"), 20));
        assert_eq!((opts[1].items[0].item.as_deref(), opts[1].items[0].qty), (Some("dagger"), 2));

        // "A, B or C" offers three things; a plain comma list gives them all
        let three = gear_options("rapier, shortsword or dagger", &names);
        assert_eq!(three.iter().map(|o| o.items.len()).collect::<Vec<_>>(), vec![1, 1, 1]);
        let all = gear_options("rapier, two daggers, and thieves' tools", &names);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].items.iter().map(|g| g.item.as_deref()).collect::<Vec<_>>(), vec![Some("rapier"), Some("dagger"), Some("thieves_tools")]);

        // rapier / two daggers / pack + tools / money
        let gear = [0usize, 1, 0, 0];
        let with = UploadContext { start_level: 1, gear: &gear, ..Default::default() };
        let ch = prepare_player_character(&lib, &camp, "Sam", &upload(), &with).unwrap();
        let has = |id: &str| ch.inventory.iter().find(|i| i.item == id).map(|i| (i.qty, i.custom));
        assert_eq!(has("rapier"), Some((1, false)));
        assert_eq!(has("shortsword"), None);
        assert_eq!(has("dagger"), Some((2, false)));
        assert_eq!(has("thieves_tools"), Some((1, false)));
        assert_eq!(has("burglar's pack"), Some((1, true)), "unknown things are kept as words: {:?}", ch.inventory);
        assert_eq!(ch.currency.gp, 15);
        // a silly choice index falls back to the first option
        let silly = [9usize];
        let with = UploadContext { start_level: 1, gear: &silly, ..Default::default() };
        assert!(prepare_player_character(&lib, &camp, "Sam", &upload(), &with).unwrap().inventory.iter().any(|i| i.item == "rapier"));
    }
}
