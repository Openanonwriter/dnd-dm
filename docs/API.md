# LAN API (for player apps)

The DM app serves plain JSON on `http://<dm-ip>:8420` (port configurable).
Reads are open to anyone on the network. Writes require the campaign's join
code when one is set: header `X-Join-Code: 1234` or `?code=1234`.

| method | path | notes |
|--------|------|-------|
| GET | `/` | human status page |
| GET | `/api/info` | `{app, version, campaign, revision, join_code_required, characters, encounter_active, packs}` |
| GET | `/api/rules` | skills, PB table, XP table, conditions… |
| GET | `/api/content` | `{species: n, classes: n, …}` |
| GET | `/api/content/{kind}` | `[{id, name}]` — kind is a folder name (`spells`) |
| GET | `/api/content/{kind}/{id}` | full entry, tagged with `"kind"` |
| GET | `/api/characters` | `[{id, name, player, level, class_line}]` |
| GET | `/api/characters/{id}` | the character's choices (same shape as the TOML) |
| PUT | `/api/characters/{id}` | replace it wholesale. DM only: needs the `dm_code`. |
| POST | `/api/characters` | create; body is a character, `id` optional. DM only: needs the `dm_code`. |
| GET | `/api/characters/{id}/sheet` | every derived number: abilities, saves, skills, AC, HP, attacks, spellcasting, features, inventory, trace |
| GET | `/api/characters/{id}/sheet.html` | printable sheet |
| GET | `/api/encounter` | player-safe view: visible combatants, positions, conditions, current turn; monster HP only if the DM allows |
| POST | `/api/roll` | `{"formula": "1d20+5"}` → `{total, rolls}` |
| GET | `/ws` | WebSocket; sends `{"type":"revision","revision":N}` whenever anything changes — then re-fetch what you need |

## Player-safe views (use these in the player app)

Same shapes as above, with DM-only data removed: content entries marked
`hidden = true`, inventory items with `hidden = true` ("fog of war" for
loot), every `dm_notes` field, and the sheet's internal trace. Numbers are
not changed: a hidden ring still gives its +1.

| method | path |
|--------|------|
| GET | `/api/player/content/{kind}` |
| GET | `/api/player/content/{kind}/{id}` |
| GET | `/api/player/characters/{id}` |
| GET | `/api/player/characters/{id}/sheet` |
| GET | `/api/player/encounter` (alias of `/api/encounter`) |
| PATCH | `/api/player/characters/{id}` — body is a `PlayerPatch` (any of `hp, death_saves, spells, conditions, exhaustion, inspiration, currency, inventory, notes, appearance, personality, ideals, bonds, flaws, backstory, player, custom`); needs the join code; returns the updated player view. Hidden items and DM notes are preserved server-side; an `inventory` list replaces only the visible items. |

## Who is at the table (devices)

Each player app makes up a random `device_id` on first run and asks the
player for their name. The DM sees every device by name on the Dashboard
("Players at the table") and assigns a character to each.

| method | path | notes |
|--------|------|-------|
| POST | `/api/player/register` | body `{device_id, name}`; needs the join code. Call on connect and on every refresh (it is also the heartbeat). Returns a `Seat`: `{device_id, name, character, character_name, can_choose}`; `character` is `null` until the DM assigns one. |
| POST | `/api/player/claim` | body `{device_id, character}`; only when the DM ticked "Players may pick their own character" (`can_choose`), else 403; 409 if another device holds that character. Returns the `Seat`. |
| GET | `/ws?device=<device_id>` | same socket as `/ws`; while it is open the device shows as online. |

A device counts as online for 45 s after its last check-in. Send
`X-Device-Id` with `PATCH /api/player/characters/{id}`: unless picking is
allowed, the server refuses (403) edits to a character that is not assigned
to that device. Assignments are saved in `campaign.toml` under `[[devices]]`.

## Player-made characters

Between sessions a player may build a character in the app and upload it.
The DM switches between "Between sessions" and "Game in progress" on the
Dashboard (`settings.game_live` in `campaign.toml`) and sets the level new
characters start at (`settings.player_start_level`, default 1). While a game
is live, uploads are refused and the DM assigns characters instead.

`Seat` carries what the app needs to know: `game_live`, `start_level`,
`rolled` (the ability scores the server rolled for this device, empty until
asked), `made_character` (the assigned character is one this player built)
and `can_create` (no game running, and either nothing assigned or the
assigned character is the player's own, which they may rebuild).

| method | path | notes |
|--------|------|-------|
| GET | `/api/player/builder` | → `{species, classes, subclasses, backgrounds, feats, items, rules, start_level}`: every non-hidden choice, item names (id → name) and the rules tables, enough to build and preview a character offline. |
| POST | `/api/player/abilities/roll` | body `{device_id}`. The server rolls 4d6-drop-lowest six times, stores the set on the device, writes it to the campaign log and returns the `Seat`. Calling again returns the same set until the DM clears it (↻ on the device's row). 403 while the game is live. |
| POST | `/api/player/characters` | body `{device_id, character, gear}`; needs the join code and a registered device. 403 while the game is live, 409 if the DM assigned this device its character, 400 with a readable reason if a choice is not legal. Returns 201 and the new `Seat`; the character is assigned to the device. If the device already plays a character it made, that one is replaced. |

The server keeps only what a player may choose: name, species, one class at
the campaign's start level, the subclass (required once the class has
reached its subclass level), background, base ability scores, class skill
picks, one improvement per ASI level reached (`advancements`: +2 to one
ability, +1 to two, or a feat) and the free-text fields. `ability_method`
must be `standard_array`, `point_buy` or `rolled`; rolled scores must be
exactly the set the server rolled for that device. Modifiers, extra items,
XP, HP overrides and DM notes in the upload are dropped.

Starting gear comes from the class's and background's `starting_equipment`
lines, in that order. In a line with an "or", " or " and commas
separate the options ("A, B or C") and "+" or "and" join the things in one
option; a line without "or" is a list you get all of. `gear[i]` picks the option
of line `i` (0 = first). Words that match an item id or name become that
item ("20 arrows" → 20 × `arrow`), "10 gp" becomes money, and anything else
is kept as a free-text item for the DM to settle.

`POST /api/characters` and `PUT /api/characters/{id}` (which take a whole
character, unchecked) need the `dm_code`.

## DM remote control

Require the campaign's `dm_code` (header `X-DM-Code` or `?dm_code=`); refused
while it is empty.

| method | path | body |
|--------|------|------|
| POST | `/api/dm/characters/{id}/inventory` | `{item, qty?, hidden?, custom?, name?, notes?, dm_notes?}` → `{index}` |
| PATCH | `/api/dm/characters/{id}/inventory/{index}` | any of `{hidden, qty, equipped, attuned, notes, dm_notes}` |
| DELETE | `/api/dm/characters/{id}/inventory/{index}` | |
| GET | `/api/dm/devices` | → `[{id, name, character, online, last_seen}]` |
| PUT | `/api/dm/devices/{id}` | `{character: "<id>"}` assigns, `{character: null}` unassigns → `Seat` |
| DELETE | `/api/dm/devices/{id}` | forget the device |

All JSON mirrors the TOML files one to one (same field names), so a player
app can cache a character as TOML or JSON interchangeably.

## The map

See `docs/MAPS.md`. `GET /api/encounter` no longer carries the map's content
(only its name and size), and a creature the party cannot see has no `pos`.

| method | path | notes |
|--------|------|-------|
| GET | `/api/player/map?device=<device_id>` | → `MapView`: what that device's character may know. `cells` (terrain rows, `?` = never seen), `seen` (`0` never, `1` remembered, `2` in view), `light` (`0` dark, `1` dim, `2` bright), `walls`, `lights`, `props`, `markers`, `tokens` (`mine` = this player may move it). Without a device, the whole party's view. |
| POST | `/api/player/map/move` | body `{device_id, x, y}`; needs the join code. Walks the device's own character there. 409 with the reason when there is no way, it is too far or someone is standing there; 403 when it is not their turn. → `{feet, map}`. |
| POST | `/api/player/map/door` | body `{device_id, x, y}` (a point in squares). Opens or closes the door nearest that point if the character is next to it and it is not locked. → `{feet: 0, map}`. |

## Playing (rolls, attacks, spells, turns)

Everything here needs the join code and a `device_id` that has a character.
The dice are rolled on the server. `mode` is `normal`, `advantage` or
`disadvantage`. Each call answers `{text, hit, total, damage}` and adds a
line to the table feed; a refusal is 409 with the reason.

| method | path | notes |
|--------|------|-------|
| GET | `/api/player/table` | → the last 60 `TableEntry` `{seq, time, who, text, kind}`: what everyone at the table has seen happen. `kind` is `roll`, `attack`, `spell`, `turn` or `rest`. |
| GET | `/api/player/spells` | → every spell players may read. |
| POST | `/api/player/spells` | body `{device_id, known: [ids]}`. Sets the spells the character knows, within what its class allows (see `docs/PLAYING.md`). While a game is live, spells can be added but not dropped. → the character. |
| POST | `/api/player/roll` | body `{device_id, formula, label, mode}`. A roll the whole table sees. |
| POST | `/api/player/attack` | body `{device_id, attack, target, mode}`; `attack` is an index into the sheet's `attacks`, `target` a combatant id. Rolled against the target's armor class; damage is applied unless the campaign setting `players_apply_damage` is off. |
| POST | `/api/player/cast` | body `{device_id, spell, slot, targets: [ids], mode}`. `slot` 0 = cantrip or ritual. Spends the slot, then attack roll / saving throws / healing per target as the spell says. |
| POST | `/api/player/do` | body `{device_id, action, arg}` with `action` = `end_turn`, `short_rest`, `long_rest`, `hit_die` (`arg` = `"d8"`) or `death_save`. |

`PATCH /api/player/characters/{id}` can no longer change which spells are
known (only `prepared`, `slots_used`), and now also takes `uses` (uses spent
of limited features, by feature name).
