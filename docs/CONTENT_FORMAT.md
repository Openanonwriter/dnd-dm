# Content format

Everything is [TOML](https://toml.io). One entry per file is the normal
style (`content/spells/fireball.toml`). Several entries can share a file using
an array named after the folder: `[[items]]`, `[[spells]]`, `[[creatures]]`…
(or `[[entries]]`).

A file is identified by the **folder** it is in:

| folder        | kind        | required keys          |
|---------------|-------------|------------------------|
| `species/`    | species     | `id`, `name`           |
| `classes/`    | class       | `id`, `name`           |
| `subclasses/` | subclass    | `id`, `name`, `class`  |
| `backgrounds/`| background  | `id`, `name`           |
| `items/`      | item        | `id`, `name`           |
| `feats/`      | feat        | `id`, `name`           |
| `spells/`     | spell       | `id`, `name`           |
| `creatures/`  | creature    | `id`, `name`           |

A file anywhere else needs `kind = "spell"` (etc.) at the top.
Files and folders starting with `.` or `_` are ignored. Everything not listed
below is optional and has a sensible default.

`id` is what other files refer to (`species = "dwarf"`). Use lower-case
letters, digits and `_`. Two packs with the same id: the later pack wins.

Every entry accepts `hidden = true`: player apps then never see it (DM-only
monsters, unrevealed magic items, secret subclasses). The DM app shows it
normally.

You rarely need to touch these files by hand: the **Content** tab edits every
field with forms and writes the TOML for you. Edits to base content are saved
as overrides in `homebrew/`; base files are never changed.

## Formulas

Any `value`, `condition`, `uses`, `prepared`, `damage` or `hp_formula` string
is a formula. Numbers, `+ - * / % ^`, parentheses, comparisons
(`< <= > >= == !=`), `&&`, `||`, `!`, dice (`1d8`, `d20`, `4d6kh3`, `2d20kl1`),
and functions `floor ceil round abs max min clamp if mod`:

| variable                  | meaning                                  |
|---------------------------|------------------------------------------|
| `level`                   | total character level                    |
| `class.<id>.level`        | levels in one class (`class.fighter.level`) |
| `pb` / `proficiency_bonus`| proficiency bonus                        |
| `str dex con int wis cha` | final ability scores                     |
| `str_mod` … `cha_mod`     | ability modifiers                        |
| `armored`, `shield`       | 1 if wearing armor / a shield            |
| `armor_light/medium/heavy`| 1 if wearing that armor category         |
| `ac`, `hp.max`, `save.<k>`, `skill.<k>`, `caster_level` | available to *later* targets (attacks, features, uses) |
| `xp`                      | experience points                        |

Dice in a sheet value are averaged (`1d4` → 2.5); dice in rolls are rolled.

## Modifiers

```toml
[[modifiers]]
target = "skill.perception"   # see table
op = "add"                    # default add
value = "2"                   # formula; empty = 0
condition = "level >= 5"      # optional
min_level = 5                 # optional shortcut for the same thing
source = "Keen Eye"           # optional; otherwise the feature/item name
note = "vs. traps"            # shown on the sheet
```

Ops: `add`, `set`, `max` (take higher — Unarmored Defense, Gauntlets of Ogre
Power), `min`, `mul`, `grant` (proficiency), `expertise`, `advantage`,
`disadvantage`, `note`. Order of application for a target:
set → add → mul → max → min.

### Targets the engine understands

| target | effect |
|--------|--------|
| `ability.str` … `ability.cha` | ability score (applied first; conditions here may only use base scores and level) |
| `proficiency_bonus` | PB |
| `save.str` … `save.cha`, `save.all` | saving throw bonus |
| `skill.<key>`, `skill.all` | skill bonus (`expertise` op doubles PB) |
| `passive.perception` / `investigation` / `insight` | passive scores |
| `ac` | final armor class |
| `ac.base` | alternative AC formula candidate, e.g. `op = "max", value = "10 + dex_mod + con_mod"` |
| `hp.max`, `hp.per_level` | hit points |
| `initiative`, `speed`, `speed.fly`, `speed.swim`, `darkvision` | |
| `attack.melee`, `attack.ranged`, `attack.all` | weapon attack rolls |
| `damage.melee`, `damage.ranged`, `damage.all` | weapon damage (dice allowed; shown as a note) |
| `spell.attack`, `spell.dc` | |
| `proficiency.save.<k>`, `proficiency.skill.<k>`, `proficiency.armor.<x>`, `proficiency.weapon.<item id or category>`, `proficiency.tool.<x>`, `proficiency.language.<x>` | with `op = "grant"` |
| `carrying_capacity` | |
| anything else | collected under "Other" on the sheet — invent your own |

Skill keys are the `key` values from `rules.toml` (`sleight_of_hand`, …).

## Features

Species traits, class/subclass features, background features and creature
traits/actions all use the same shape:

```toml
[[features]]
name = "Second Wind"
level = 1                       # class level (or 1)
description = "…"
uses = "1"                      # formula, optional
recharge = "short_rest"         # free text
modifiers = [ { target = "ac", op = "add", value = "1", condition = "armored" } ]
attack = { to_hit = 4, reach = "reach 5 ft.", damage = "1d6 + 2", damage_type = "slashing" }  # creatures
```

## Species

```toml
id = "dwarf"
name = "Dwarf"
size = "Medium"          # Tiny Small Medium Large Huge Gargantuan
speed = 25
speeds = { fly = 30 }    # extra movement modes
darkvision = 60
languages = ["Common", "Dwarvish"]
description = "…"
[[modifiers]] …
[[traits]] …
```

## Classes

```toml
id = "fighter"
name = "Fighter"
hit_die = 10
primary_abilities = ["str", "dex"]
saving_throws = ["str", "con"]            # only the first class grants these
armor_proficiencies = ["light", "medium", "heavy", "shields"]
weapon_proficiencies = ["simple", "martial", "longsword"]   # categories or item ids
tool_proficiencies = []
subclass_level = 3
subclass_label = "Martial Archetype"
asi_levels = [4, 8, 12, 16, 19]
starting_equipment = ["…"]                # free text
[skill_choices]
count = 2
from = ["acrobatics", "athletics"]        # empty = any
[spellcasting]                            # optional; may also sit on a subclass
ability = "int"
progression = "full"                      # full | half | third | pact
prepared = "max(1, int_mod + class.wizard.level)"
cantrips_known = [3,3,3,4,…]              # by class level
spells_known = [ … ]                      # for "known" casters
ritual_casting = true
spell_lists = ["wizard"]                  # extra class lists to draw from
[[features]] …
```

Slots come from `rules.toml` (`spell_slots`, `pact_slots`) using the usual
multiclass caster-level maths.

## Subclasses

```toml
id = "champion"
name = "Champion"
class = "fighter"
[[features]] …           # `level` is the class level
```

## Backgrounds

```toml
id = "soldier"
name = "Soldier"
skill_proficiencies = ["athletics", "intimidation"]
tool_proficiencies = ["gaming set"]
languages = ["Dwarvish"]
language_choices = 1
starting_equipment = ["…"]
[feature]
name = "Military Rank"
description = "…"
```

## Items

```toml
id = "longsword"
name = "Longsword"
kind = "weapon"            # weapon armor shield gear consumable wondrous tool ammunition
cost = "15 gp"
weight = 3.0
rarity = "common"
requires_attunement = false
description = "…"
[weapon]
damage = "1d8"
damage_type = "slashing"
category = "martial"       # simple | martial
kind = "melee"             # melee | ranged
properties = ["versatile"] # finesse light heavy two_handed thrown reach ammunition loading versatile
versatile = "1d10"
range = "80/320"
bonus = 1                  # magic +N to hit and damage
[armor]
base_ac = 16
category = "heavy"         # light | medium | heavy | shield
dex_cap = 0                # omit for unlimited (light), 2 for medium, 0 for heavy
strength_requirement = 13
stealth_disadvantage = true
bonus = 0
[[modifiers]] …            # apply while equipped (and attuned if required)
```

Attack bonus = ability mod (Str; Dex for ranged; best of both for finesse)
+ PB if proficient + `bonus` + `attack.*` modifiers. Proficiency is checked
against the weapon's `category`, the item `id`, the item name, or a
`proficiency.weapon.<x>` grant.

## Feats

```toml
id = "tough"
name = "Tough"
prerequisite = ""
description = "…"
[[modifiers]] …
```

## Spells

```toml
id = "fireball"
name = "Fireball"
level = 3                 # 0 = cantrip
school = "evocation"
casting_time = "1 action"
range = "150 feet"
components = "V, S, M"
duration = "Instantaneous"
concentration = false
ritual = false
classes = ["wizard", "sorcerer"]
attack = false
save = "dex"
damage = "8d6"
damage_type = "fire"
area = { shape = "sphere", size = 20, width = 5 }   # sphere cube cone line cylinder
description = "…"
higher_levels = "…"
```

## Creatures

```toml
id = "goblin"
name = "Goblin"
size = "Small"
kind = "humanoid (goblinoid)"
alignment = "neutral evil"
ac = 15
ac_note = "leather armor, shield"
hp = 7
hp_formula = "2d6"
cr = "1/4"
xp = 50
senses = ["darkvision 60 ft."]
languages = ["Common", "Goblin"]
damage_resistances = []
damage_immunities = []
damage_vulnerabilities = []
condition_immunities = []
spells = ["fire_bolt"]
[speed]
walk = 30
fly = 60
[abilities]
str = 8
dex = 14
con = 10
int = 10
wis = 8
cha = 8
[saves]
dex = 4            # only list proficient saves; others derive from abilities
[skills]
stealth = 6
[[traits]] …
[[actions]] …      # with attack = { to_hit, reach, damage, damage_type }
[[bonus_actions]] …
[[reactions]] …
[[legendary_actions]] …
```

Proficiency bonus is derived from CR. Omitted `hp` is averaged from `hp_formula`.

## rules.toml

Skills, proficiency bonus table, XP thresholds, point-buy costs, standard
array, distance rule, feet per square, HP mode, spell slot tables, size
categories, condition and damage type lists. Any key you omit keeps the 5e
default. See `content/rules.toml`.

## Characters (`saves/<campaign>/characters/<id>.toml`)

Only *choices* are stored; every derived number is recomputed on load:

```toml
id = "bruenor"
name = "Bruenor"
player = "Bob"
species = "dwarf"
background = "soldier"
classes = [{ class = "fighter", subclass = "champion", level = 4 }]
base_abilities = { str = 16, dex = 12, con = 14, int = 8, wis = 10, cha = 10 }
skill_proficiencies = ["perception", "survival"]
expertise = []
feats = []
advancements = [{ level = 4, ability_increases = { str = 2 } }]
inventory = [
  { item = "chain_mail", equipped = true },
  { item = "longsword", equipped = true, qty = 1 },
  { item = "ring_of_protection", equipped = true, attuned = true, hidden = true, dm_notes = "actually cursed" },  # hidden = player app can't see it
]
dm_notes = "owes the thieves' guild 200 gp"   # never sent to players
hp = { current = 40, temp = 0 }
spells = { known = [], prepared = [], slots_used = [] }
modifiers = [{ target = "ac", op = "add", value = "1", source = "Blessing of Moradin" }]
```
