//! Printable exports. HTML is used because every OS can print it (and save
//! it as PDF) from a browser without extra dependencies.

use crate::expr::fmt_bonus;
use crate::library::Library;
use crate::model::Creature;
use crate::sheet::Sheet;
use std::fmt::Write as _;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn para(s: &str) -> String {
    s.split("\n\n").map(|p| format!("<p>{}</p>", esc(p.trim()).replace('\n', "<br>"))).collect()
}

const CSS: &str = r#"
:root { --ink:#1b1b1b; --line:#8a7d6a; --soft:#f3ecdc; }
* { box-sizing:border-box; }
body { font-family: Georgia, 'Times New Roman', serif; color:var(--ink); margin:0; padding:24px; background:#fff; }
h1 { margin:0; font-size:28px; letter-spacing:1px; }
h2 { font-size:14px; text-transform:uppercase; letter-spacing:2px; border-bottom:2px solid var(--line); margin:18px 0 8px; padding-bottom:2px; }
h3 { font-size:13px; margin:10px 0 2px; }
.sub { color:#555; font-size:13px; }
.grid { display:grid; gap:12px; }
.cols3 { grid-template-columns: 1fr 1fr 1fr; }
.cols2 { grid-template-columns: 1fr 1fr; }
.box { border:1.5px solid var(--line); border-radius:8px; padding:8px 10px; background:#fff; }
.stat { text-align:center; }
.stat .big { font-size:26px; font-weight:bold; }
.stat .small { font-size:11px; text-transform:uppercase; letter-spacing:1px; color:#555; }
.abilities { display:grid; grid-template-columns: repeat(6, 1fr); gap:8px; }
table { width:100%; border-collapse:collapse; font-size:12.5px; }
td, th { padding:2px 4px; border-bottom:1px solid #ddd; text-align:left; vertical-align:top; }
th { background:var(--soft); font-size:11px; text-transform:uppercase; letter-spacing:1px; }
.dot { display:inline-block; width:9px; height:9px; border:1px solid var(--ink); border-radius:50%; margin-right:4px; }
.dot.on { background:var(--ink); }
.note { color:#7a4b00; font-size:11px; }
.feat { font-size:12.5px; margin-bottom:6px; }
.feat b { display:block; }
p { margin:4px 0; }
.warn { color:#a00; font-size:12px; }
@media print { body { padding:0; } .nobreak { break-inside:avoid; } @page { margin:12mm; } }
"#;

/// Full printable character sheet.
pub fn character_sheet_html(s: &Sheet, lib: &Library) -> String {
    let mut h = String::new();
    let _ = write!(
        h,
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{} - Character Sheet</title><style>{CSS}</style></head><body>",
        esc(&s.name)
    );
    let _ = write!(
        h,
        "<div class=\"grid cols2\"><div><h1>{}</h1><div class=\"sub\">{} &middot; {} &middot; {}</div><div class=\"sub\">Level {} &middot; {} XP{}</div></div><div class=\"sub\" style=\"text-align:right\">Player: {}<br>Alignment: {}<br>Size: {}</div></div>",
        esc(&s.name),
        esc(&s.class_line),
        esc(&s.species),
        esc(&s.background),
        s.level,
        s.xp,
        s.next_level_xp.map(|n| format!(" (next level at {n})")).unwrap_or_default(),
        esc(&s.player),
        esc(&s.alignment),
        esc(&s.size)
    );

    // abilities
    h.push_str("<h2>Abilities</h2><div class=\"abilities\">");
    for (k, v) in s.abilities.iter() {
        let m = s.ability_mods.get(k).copied().unwrap_or(0);
        let _ = write!(h, "<div class=\"box stat\"><div class=\"small\">{}</div><div class=\"big\">{}</div><div>{}</div></div>", k.to_uppercase(), fmt_bonus(m as i64), v);
    }
    h.push_str("</div>");

    // core stats
    let hd: Vec<String> = s.hit_dice.iter().map(|(d, n)| format!("{n}{d}")).collect();
    let _ = write!(
        h,
        "<div class=\"grid cols3\" style=\"margin-top:12px\">\
        <div class=\"box stat\"><div class=\"small\">Armor Class</div><div class=\"big\">{}</div><div class=\"note\">{}</div></div>\
        <div class=\"box stat\"><div class=\"small\">Hit Points</div><div class=\"big\">{} / {}</div><div class=\"note\">temp {} &middot; hit dice {}</div></div>\
        <div class=\"box stat\"><div class=\"small\">Initiative / Speed / Prof.</div><div class=\"big\">{} / {} ft / {}</div><div class=\"note\">passive Perception {}{}</div></div>\
        </div>",
        s.ac,
        esc(&s.ac_breakdown),
        s.hp_current,
        s.hp_max,
        s.hp_temp,
        hd.join(" "),
        fmt_bonus(s.initiative as i64),
        s.speed,
        fmt_bonus(s.proficiency_bonus as i64),
        s.passive_perception,
        if s.darkvision > 0 { format!(" &middot; darkvision {} ft", s.darkvision) } else { String::new() }
    );

    // saves + skills
    h.push_str("<div class=\"grid cols2\" style=\"margin-top:12px\"><div><h2>Saving Throws</h2><table>");
    for sv in &s.saves {
        let _ = write!(
            h,
            "<tr><td><span class=\"dot{}\"></span>{}</td><td><b>{}</b></td><td class=\"note\">{}</td></tr>",
            if sv.proficient { " on" } else { "" },
            esc(&sv.name),
            fmt_bonus(sv.bonus as i64),
            esc(&sv.notes.join("; "))
        );
    }
    h.push_str("</table><h2>Attacks</h2><table><tr><th>Name</th><th>Hit</th><th>Damage</th><th>Range</th></tr>");
    for a in &s.attacks {
        let _ = write!(
            h,
            "<tr><td>{}<div class=\"note\">{}</div></td><td><b>{}</b></td><td>{} {}</td><td>{}</td></tr>",
            esc(&a.name),
            esc(&a.notes.join("; ")),
            fmt_bonus(a.bonus as i64),
            esc(&a.damage),
            esc(&a.damage_type),
            esc(&a.range)
        );
    }
    h.push_str("</table>");
    if !s.spellcasting.is_empty() {
        h.push_str("<h2>Spellcasting</h2><table><tr><th>Class</th><th>Ability</th><th>Attack</th><th>DC</th><th>Prepared</th></tr>");
        for sc in &s.spellcasting {
            let _ = write!(
                h,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc(&sc.class_name),
                sc.ability.to_uppercase(),
                fmt_bonus(sc.attack_bonus as i64),
                sc.save_dc,
                sc.prepared_max.map(|p| p.to_string()).unwrap_or_else(|| "-".into())
            );
        }
        h.push_str("</table><div class=\"sub\">Slots: ");
        for (i, n) in s.spell_slots.iter().enumerate() {
            if *n > 0 {
                let used = s.spell_slots_used.get(i).copied().unwrap_or(0);
                let _ = write!(h, "L{} {}/{} &nbsp;", i + 1, n - used.min(*n), n);
            }
        }
        if let Some((n, l)) = s.pact_slots {
            let _ = write!(h, "Pact {n} × level {l}");
        }
        h.push_str("</div>");
        if !s.spells_prepared.is_empty() || !s.spells_known.is_empty() {
            h.push_str("<table><tr><th>Spell</th><th>Lvl</th><th>Time</th><th>Range</th><th>Prep</th></tr>");
            let mut ids: Vec<&String> = s.spells_known.iter().chain(s.spells_prepared.iter()).collect();
            ids.sort();
            ids.dedup();
            for id in ids {
                let (name, lvl, time, range) = match lib.spells.get(id) {
                    Some(sp) => (sp.name.clone(), sp.level.to_string(), sp.casting_time.clone(), sp.range.clone()),
                    None => (id.clone(), "?".into(), String::new(), String::new()),
                };
                let _ = write!(
                    h,
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    esc(&name),
                    lvl,
                    esc(&time),
                    esc(&range),
                    if s.spells_prepared.contains(id) { "✓" } else { "" }
                );
            }
            h.push_str("</table>");
        }
    }
    h.push_str("</div><div><h2>Skills</h2><table>");
    for sk in &s.skills {
        let _ = write!(
            h,
            "<tr><td><span class=\"dot{}\"></span>{}{}</td><td class=\"sub\">{}</td><td><b>{}</b></td><td class=\"note\">{}</td></tr>",
            if sk.proficient { " on" } else { "" },
            esc(&sk.name),
            if sk.expertise { " ★" } else { "" },
            sk.ability.to_uppercase(),
            fmt_bonus(sk.bonus as i64),
            esc(&sk.notes.join("; "))
        );
    }
    h.push_str("</table></div></div>");

    // proficiencies
    let p = &s.proficiencies;
    let _ = write!(
        h,
        "<h2>Proficiencies &amp; Languages</h2><div class=\"sub\"><b>Armor:</b> {} &nbsp; <b>Weapons:</b> {} &nbsp; <b>Tools:</b> {} &nbsp; <b>Languages:</b> {}</div>",
        esc(&p.armor.join(", ")),
        esc(&p.weapons.join(", ")),
        esc(&p.tools.join(", ")),
        esc(&p.languages.join(", "))
    );

    // features
    h.push_str("<h2>Features &amp; Traits</h2><div class=\"grid cols2\">");
    for f in &s.features {
        let _ = write!(
            h,
            "<div class=\"feat nobreak\"><b>{} <span class=\"sub\">({}{})</span></b>{}</div>",
            esc(&f.name),
            esc(&f.source),
            f.uses.map(|u| format!(", {u} uses/{}", if f.recharge.is_empty() { "rest" } else { &f.recharge })).unwrap_or_default(),
            para(&f.description)
        );
    }
    h.push_str("</div>");

    // inventory
    let _ = write!(
        h,
        "<h2>Equipment</h2><table><tr><th>Item</th><th>Qty</th><th>Wt</th><th>Eq.</th><th>Notes</th></tr>"
    );
    for i in &s.inventory {
        let _ = write!(
            h,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}{}</td><td>{}</td></tr>",
            esc(&i.name),
            i.qty,
            i.weight * i.qty as f64,
            if i.equipped { "✓" } else { "" },
            if i.attuned { " (attuned)" } else { "" },
            esc(&i.notes)
        );
    }
    let c = &s.currency;
    let _ = write!(
        h,
        "</table><div class=\"sub\">Weight {} / {} lb &nbsp; Coins: {} cp, {} sp, {} ep, {} gp, {} pp</div>",
        s.total_weight, s.carrying_capacity, c.cp, c.sp, c.ep, c.gp, c.pp
    );

    if !s.other.is_empty() {
        h.push_str("<h2>Other</h2><table>");
        for (k, v) in &s.other {
            let _ = write!(h, "<tr><td>{}</td><td>{}</td></tr>", esc(k), v);
        }
        h.push_str("</table>");
    }
    if !s.warnings.is_empty() {
        h.push_str("<h2>Warnings</h2>");
        for w in &s.warnings {
            let _ = write!(h, "<div class=\"warn\">{}</div>", esc(w));
        }
    }
    h.push_str("</body></html>");
    h
}

/// Printable stat block for a creature.
pub fn creature_html(c: &Creature) -> String {
    let mut h = String::new();
    let _ = write!(h, "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>{CSS}</style></head><body>", esc(&c.name));
    let speed: Vec<String> = c.speed.iter().map(|(k, v)| if k == "walk" { format!("{v} ft.") } else { format!("{k} {v} ft.") }).collect();
    let _ = write!(
        h,
        "<h1>{}</h1><div class=\"sub\">{} {}, {}</div><div class=\"box\" style=\"margin-top:8px\"><b>Armor Class</b> {} {}<br><b>Hit Points</b> {} {}<br><b>Speed</b> {}</div>",
        esc(&c.name),
        esc(&c.size),
        esc(&c.kind),
        esc(&c.alignment),
        c.ac,
        if c.ac_note.is_empty() { String::new() } else { format!("({})", esc(&c.ac_note)) },
        c.hp,
        if c.hp_formula.is_empty() { String::new() } else { format!("({})", esc(&c.hp_formula)) },
        speed.join(", ")
    );
    h.push_str("<div class=\"abilities\" style=\"margin-top:8px\">");
    for (k, v) in c.abilities.iter() {
        let _ = write!(h, "<div class=\"box stat\"><div class=\"small\">{}</div><div class=\"big\">{}</div><div>{}</div></div>", k.to_uppercase(), v, fmt_bonus(crate::model::ability_mod(v) as i64));
    }
    h.push_str("</div><div class=\"sub\" style=\"margin-top:8px\">");
    if !c.saves.is_empty() {
        let v: Vec<String> = c.saves.iter().map(|(k, v)| format!("{} {}", k.to_uppercase(), fmt_bonus(*v as i64))).collect();
        let _ = write!(h, "<b>Saving Throws</b> {}<br>", v.join(", "));
    }
    if !c.skills.is_empty() {
        let v: Vec<String> = c.skills.iter().map(|(k, v)| format!("{} {}", k, fmt_bonus(*v as i64))).collect();
        let _ = write!(h, "<b>Skills</b> {}<br>", esc(&v.join(", ")));
    }
    for (label, list) in [
        ("Damage Vulnerabilities", &c.damage_vulnerabilities),
        ("Damage Resistances", &c.damage_resistances),
        ("Damage Immunities", &c.damage_immunities),
        ("Condition Immunities", &c.condition_immunities),
        ("Senses", &c.senses),
        ("Languages", &c.languages),
    ] {
        if !list.is_empty() {
            let _ = write!(h, "<b>{label}</b> {}<br>", esc(&list.join(", ")));
        }
    }
    let _ = write!(h, "<b>Challenge</b> {} ({} XP) &nbsp; <b>Proficiency Bonus</b> {}</div>", esc(&c.cr), c.xp, fmt_bonus(c.proficiency_bonus() as i64));
    for (title, list) in [
        ("Traits", &c.traits),
        ("Actions", &c.actions),
        ("Bonus Actions", &c.bonus_actions),
        ("Reactions", &c.reactions),
        ("Legendary Actions", &c.legendary_actions),
    ] {
        if list.is_empty() {
            continue;
        }
        let _ = write!(h, "<h2>{title}</h2>");
        for f in list {
            let atk = f
                .attack
                .as_ref()
                .map(|a| format!(" <i>{} to hit, {}, {} {} damage{}</i>", fmt_bonus(a.to_hit as i64), esc(&a.reach), esc(&a.damage), esc(&a.damage_type), if a.extra.is_empty() { String::new() } else { format!(", {}", esc(&a.extra)) }))
                .unwrap_or_default();
            let _ = write!(h, "<div class=\"feat\"><b>{}</b>{}{}</div>", esc(&f.name), atk, para(&f.description));
        }
    }
    if !c.description.is_empty() {
        let _ = write!(h, "<h2>Description</h2>{}", para(&c.description));
    }
    h.push_str("</body></html>");
    h
}
