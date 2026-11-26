# dnd-dm

An open-source Dungeon Master toolkit for D&D 5e and homebrew, written in Rust.
The desktop app (Windows or Linux) is the **server**: it holds the content
library, the characters and the live game state, and shares them over Wi-Fi
with the player app (Android phones, or a desktop window).

Everything the game needs is a plain text **TOML** file you can read, write
and version in git: species, classes, subclasses, backgrounds, items, feats,
spells, creatures, rules tables, characters and whole saved campaigns.

## Features (v0.1)

- **Form-based editing, files as the backend** — every species, class,
  item, spell, creature… is edited with ordinary fields and add/remove rows
  on the Content tab; the app writes the TOML. Base content is never touched:
  edits become overrides in `homebrew/`.
- **Autosave** — the campaign (characters, encounter, map, log) is written to
  `saves/<campaign>/` about a second after every change; content edits are
  saved the same way. The last campaign reopens on start.
- **Overview dashboard** — every campaign on disk with one-click load, every
  character from every campaign (open or import), server address, content
  health, recent log. Orange-on-black theme.
- **Player-safe views** — content entries and inventory items can be marked
  hidden ("fog of war" for loot); DM notes never leave the DM app. The
  `/api/player/*` endpoints strip all of it, and `/api/dm/*` lets a remote DM
  tool add, hide or remove items on a sheet.
- **Human-writable content packs** — `content/` ships the whole SRD: all 12
  classes (levels 1–20) plus the Artificer, each with its subclass and every
  class choice (fighting styles, invocations, metamagic, infusions…), 17
  races and subraces, 335 spells with class lists, the full weapon and armor
  tables, adventuring gear, packs, poisons and 11 backgrounds with suggested
  personality traits, ideals, bonds and flaws. `homebrew/` is yours. Later
  packs override earlier ones by id.
- **Help for new players** — a small (i) next to every game term, stat,
  skill, condition, class, spell and item explains it in plain language:
  hover on a PC, tap on a phone. A test keeps every entry explained.
- **Rules checks** — a subclass taken too early, an invocation without its
  pact, a spell above the castable level, armor the character can't wear
  well… The DM may give a character anything (it shows with a red outline and
  the reason); players can only pick what their character qualifies for
  unless the DM allows rule breaks, for everyone or for one character.
- **Death is final** — three failed death saves and the character is dead:
  no resting, no ordinary healing, no more death saves until revivify, raise
  dead, resurrection or the DM brings them back.
- **Rules engine** — ability modifiers, proficiency, saves, skills, expertise,
  AC (armor, shields, Unarmored-Defense-style formulas), hit points, hit dice,
  initiative, speed, attacks, spell attack / DC / slots (single and multiclass),
  passive scores, carrying capacity. Every number carries a trace of which
  source changed it and why.
- **Formulas everywhere** — `"2 * level"`, `"max(1, wis_mod + class.cleric.level)"`,
  `"1d8 + str_mod"`, conditions like `"armored && level >= 5"`.
- **Creatures & grid math** — stat blocks, initiative tracker, HP, conditions,
  token positions, distance (5e / 5-10-5 / Euclidean), "who is within 20 ft",
  spheres, cubes, cones and lines highlighted on a battle map.
- **Printable sheets** — one click exports an HTML character sheet or stat
  block and opens it in your browser (print or save as PDF).
- **Save game state** — campaign, encounter, map and every character go to
  `saves/<campaign>/` as TOML.
- **Wi-Fi server** — JSON API + WebSocket on the LAN for player apps.
- **Player app (Android + desktop)** — `crates/dnd-player`: finds the DM on
  the Wi-Fi, shows the live sheet, rolls dice, tracks HP/slots/conditions/
  items/notes and sends the changes back; works offline from a cache. See
  [docs/PLAYER_APP.md](docs/PLAYER_APP.md) and `build-android.ps1`.

## Linux

`./build-linux.sh --deps --desktop --firewall` builds both apps on x86_64 or
ARM64 and puts them in `dist/dnd-dm-linux-<arch>/`. See
[docs/LINUX.md](docs/LINUX.md).

## Build & run

Requires Rust 1.80+ (https://rustup.rs). From this folder:

```
cargo run --release -p dnd-dm
```

The app looks for `content/` next to the executable or in the current folder,
creates `homebrew/`, `saves/`, `exports/` and a `dnd-dm.toml` config on first
start. If a build fails with "Access is denied", close the running app first:
Windows won't replace a program that is open.

Run the tests (rules engine, grid math, save/load, and a check that every file
in `content/` parses):

```
cargo test
```

## Folder layout

```
content/            base pack (complete SRD) — species/ classes/ items/ ... rules.toml
homebrew/           your pack; same layout; overrides content/ by id
saves/<campaign>/   campaign.toml + characters/*.toml
exports/            printable HTML sheets and TOML character exports
crates/dnd-core     rules engine, data model, TOML loader, grid, export
crates/dnd-server   axum HTTP + WebSocket LAN server
crates/dnd-dm       egui desktop app (the DM)
crates/dnd-player   egui player app: Android (NativeActivity) + desktop
build-android.ps1   cargo-ndk + Gradle -> dist/dnd-player.apk
docs/CONTENT_FORMAT.md   how to write every kind of file, all modifier targets
docs/API.md              what player apps can call
docs/PLAYING.md          running a session: fights, spells, rests, death, levelling
docs/PLAYER_APP.md       the phone app: build, screens, how it is put together
docs/MAPS.md             maps, fog of war, lighting, walking and doors
docs/LINUX.md            building on Linux
docs/DEVELOPING.md       dev switches, screenshots, tests, editing pitfalls
docs/MAP_RESEARCH.md     the research the map design came from
```

## Characters

The DM makes characters on the **Characters** tab, or players build their own
in the phone app between sessions and send them in. Either way every pick
(species, class, subclass, background, fighting style, invocations, feats,
skills, spells, gear) is explained by an (i), and anything the character
doesn't qualify for is outlined in red with the reason. The DM can always
go ahead; players need the DM's say-so (Dashboard → *Players may break the
rules*, or the checkbox on one character's Build tab).

## Writing content

See [docs/CONTENT_FORMAT.md](docs/CONTENT_FORMAT.md). Short version: copy a
file from `content/<kind>/` into `homebrew/<kind>/`, change the `id`, edit the
rest, press **Reload content** in the app (or create it from the Content tab).

A modifier is the one mechanism that changes numbers:

```toml
[[modifiers]]
target = "ac"            # what to change
op = "add"               # add | set | max | min | mul | grant | expertise | advantage | disadvantage | note
value = "1"              # a formula
condition = "armored"    # optional formula; applies only when true
```

## License

MIT. The content in `content/` includes game mechanics from the System
Reference Document 5.1 ("SRD 5.1") and 5.2 ("SRD 5.2") by Wizards of the
Coast LLC, available at https://www.dndbeyond.com/srd, licensed under the
Creative Commons Attribution 4.0 International License
(https://creativecommons.org/licenses/by/4.0/legalcode). Descriptions are
written in our own words. Material outside the SRD (the books' other
subclasses, races and spells) is not included; add it in `homebrew/`.
