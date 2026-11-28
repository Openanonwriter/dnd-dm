# Developing

How to build, test and look at the apps while working on them.

## Build and test

```text
cargo build --release --workspace            # both apps and the server
cargo build --release -p dnd-player --example demo   # the player demo
cargo test --workspace
```

`--examples` on its own builds *only* the examples, not the apps, so build
the apps and the demo separately. On Windows a build fails with "Access is
denied" while the app it would replace is running: close it first (the app
saves on exit).

The tests load the real `content/` pack, so a broken TOML file, a feature
that grants a spell that doesn't exist, a background without its suggestion
tables or an entry without a usable description fails `cargo test`.

## Seeing the apps without a screen

egui windows can't be captured from a script (screen grabs come back white),
so each app can save its own frame and exit. Keep screenshot runs on a
*copy* of a campaign: the DM app opened with `DND_DM_SHOT` never saves, and
one opened with `DND_DM_CAMPAIGN` saves back to that folder, never over the
campaign of the same name in `saves/`.

DM app (`target/release/dnd-dm.exe`):

| variable | effect |
|----------|--------|
| `DND_DM_SHOT=<file.bmp>` | save a picture after ~45 frames, then exit; never saves the campaign; doesn't start the server |
| `DND_DM_CAMPAIGN=<dir>` | open that campaign folder (and save back to it) |
| `DND_DM_VIEW=dashboard\|content\|characters\|encounter\|map\|campaign` | which tab |
| `DND_DM_SELECT=<id>` | the selected character (Characters tab) or combatant (Encounter tab) |
| `DND_DM_CHAR_TAB=build\|sheet\|inventory\|spells\|modifiers\|notes` | which character sub-tab |
| `DND_DM_SHOT_SIZE=1366x768` | window size in points |
| `DND_DM_SHOT_ZOOM=1.5` | mimic Windows display scaling (125 % = 1.25, 150 % = 1.5) |

Player demo (`target/release/examples/demo.exe`, a wizard and a fighter in a
cellar with two goblins, server on 127.0.0.1 only):

| variable | effect |
|----------|--------|
| `DND_PLAYER_SHOT=<file.bmp>` | save a picture and exit |
| `DND_PLAYER_TAB=stats\|combat\|spells\|items\|notes\|map\|dice` | which tab |
| `DND_PLAYER_CREATE="species,class,background"` | open the character builder with those picks |
| `DND_PLAYER_SCROLL=<points>` | start the builder scrolled down |

Convert BMP to PNG with PowerShell:

```powershell
Add-Type -AssemblyName System.Drawing
$b = [System.Drawing.Bitmap]::FromFile("C:\path\shot.bmp"); $b.Save("C:\path\shot.png", [System.Drawing.Imaging.ImageFormat]::Png); $b.Dispose()
```

## A practice table (playing a fight without a DM)

```text
cargo run --release -p dnd-player --example table            # port 8421, this machine only
cargo run --release -p dnd-player --example table -- --lan   # reachable from the Wi-Fi
```

A real server with the cellar fight on it. The first device to connect
plays Thia the wizard; the goblins and Borin take their own turns, and two
more goblins come when the first two are beaten. Over USB:
`adb reverse tcp:8421 tcp:8421`, then connect the phone to `127.0.0.1:8421`.
Everything a player does is also reachable with `curl` (see
`docs/API.md`, "Playing"), which is how a session can be scripted while
taking pictures with `DND_PLAYER_SHOT`.

## Fonts

egui's built-in font has no arrows (→ ↓ ▾), check marks (✓ ✗) or many
symbols; they draw as boxes. Use plain text, or draw the shape (the (i)
icon in `crates/dnd-mapview/src/help.rs` is painted, not typed). A few
emoji the font does have are used on buttons; check a new one with
`cargo run -p dnd-mapview --example glyphs`.

## Where things live

| what | where |
|------|-------|
| rules engine, sheet maths | `crates/dnd-core/src/sheet.rs` |
| playing (attacks, spells, rests, death, turns) | `crates/dnd-core/src/play.rs` |
| what a character qualifies for | `crates/dnd-core/src/requirements.rs` |
| player-made characters (upload sanitising, suggestions) | `crates/dnd-core/src/builder.rs` |
| plain-language glossary for the (i) icons | `crates/dnd-core/src/glossary.rs` |
| entry summaries for the (i) icons | `crates/dnd-core/src/describe.rs` |
| the (i) icon itself | `crates/dnd-mapview/src/help.rs` |
| DM app screens | `crates/dnd-dm/src/ui/*.rs` |
| player app screens | `crates/dnd-player/src/ui/*.rs` |
| LAN server routes | `crates/dnd-server/src/lib.rs` |

## The look (both apps)

Both apps are built from the same few parts, each defined once in its
`theme.rs` (`crates/dnd-dm/src/ui/theme.rs`, `crates/dnd-player/src/theme.rs`):
one control height and corner radius (`H_CTRL`, `R_CTRL`: 30 and 8 on the
desk, 44 and 12 on the phone), one card radius (`R_CARD`), the same colours,
and the same text face (Adwaita Sans, in `crates/dnd-mapview/assets/fonts`,
installed by `dnd_mapview::face::install`). Use the parts rather than
one-off sizes: `theme::primary` for the main action, a plain `ui.button`
for the rest, `theme::chip` / `theme::choice` for on/off and one-of-several
(`selectable_label` is for rows inside menus and tables only),
`theme::line_edit` for one-line text, `theme::card()` and
`theme::section()`. A button may never sit on top of text: where a row has
words and buttons, give the words the room the buttons leave
(`theme::Room`), or put the buttons on their own line. Lists of check boxes
go in a grid so they line up. `docs/PLAYER_APP.md` has the full table.

## Layout rules of thumb

Windows display scaling shrinks the space the app has. Rows of widgets use
`horizontal_wrapped`; a `ComboBox` can't wrap by itself, so `id_combo` in
`crates/dnd-dm/src/ui/mod.rs` starts a new row when the remaining width is
under 200 points (`available_size_before_wrap`, not `available_width`).
Framed tiles don't wrap either: lay them out in rows sized to the panel.
