# Battle maps & fog of war — research and proposal

Status: **proposal, not built.** Nothing here is implemented beyond the
plain grid map that exists today. Approve, change or reject before any map
work starts.

## What the good VTTs do

Three levels of fog exist in the wild, and the best tools offer all three:

1. **Manual fog** — the DM paints/erases hidden areas with a brush or shapes.
   Roll20's classic fog works this way; Owlbear Rodeo's native fog is
   pixel-accurate, "so it supports any arbitrary shape"
   ([Owlbear blog](https://blog.owlbear.rodeo/owlbear-rodeo-2-0-dev-log-3/)).
   Cheap, predictable, never wrong. Still the right default for theatre-of-
   the-mind-with-a-picture games.
2. **Explored fog (memory)** — areas a token has seen stay dimly visible
   after it leaves. Roll20's Advanced Fog and Foundry's "fog exploration" both
   record this per scene and offer a reset
   ([Roll20](https://help.roll20.net/hc/en-us/articles/360037774493-Advanced-Fog-of-War-Classic-VTT),
   [Foundry lighting](https://foundryvtt.com/article/lighting/)).
3. **Dynamic fog / vision** — walls block sight and light; each token's
   visible area is computed live from its position. RollQuest describes it as
   the VTT "knowing where walls are, working out what each token can see, and
   clearing fog to match" ([RollQuest](https://www.rollquest.io/features/fog-of-war)).
   Owlbear got there via its Warp Core engine (GPU, real-time) and the
   *Dynamic Fog* / *Smoke & Spectre* extensions
   ([2.3 launch](https://blog.owlbear.rodeo/owlbear-rodeo-2-3-release-week-day-3/),
   [Smoke & Spectre](https://extensions.owlbear.rodeo/smoke)). Foundry models
   it with wall types, light sources with bright/dim radius, colour, angle and
   animation, a scene darkness level 0–1, and "darkness sources" that eat light
   ([Foundry](https://foundryvtt.com/article/lighting/)).

Things players notice and love:

- **Per-player vision**: each player sees what *their* token sees (Smoke &
  Spectre does this; Roll20 shows the union of all owned tokens).
- **Doors** that the DM clicks open/closed and that update vision instantly.
- **Light matters**: torches, darkvision, dim light; "bright / dim / dark"
  shading rather than a binary reveal.
- **Soft edges** and smooth reveal; jagged staircases along walls look cheap
  (a known Roll20 complaint).
- **The DM sees everything** with the fog drawn translucent, plus a toggle to
  "see as player X".

Known pitfalls to design against:

- **Wall-hack**: dragging a token *through* a wall momentarily reveals the
  other side (Roll20 bug thread). Fix: validate moves against walls server-side
  and compute vision from the last legal position only.
- **Performance**: visibility must be recomputed on every token move; keep
  wall counts modest or precompute per-grid-cell visibility.
- **Importing maps**: map makers already export wall data; don't make the DM
  draw walls twice.

## Map formats worth supporting

**Universal VTT (`.dd2vtt` / `.df2vtt` / `.uvtt`)** — one JSON file exported
by Dungeondraft, DungeonFog and Arkenforge containing the image *and* the
walls, doors and lights
([Dungeondraft guide](https://dungeondraft-encyclopaedia.gitbook.io/guide/final-steps/exporting-your-map/universal-vtt),
[Roll20 UVTT](https://help.roll20.net/hc/en-us/articles/41643201127831-Universal-Virtual-Tabletop-UVTT-Support)).
Reading the open-source [uvtt2fgu](https://github.com/Imagix/uvtt2fgu) converter
gives the field layout:

| key | type | units |
|-----|------|-------|
| `resolution.pixels_per_grid` | int | pixels per square |
| `resolution.map_size.{x,y}` | number | squares |
| `resolution.map_origin.{x,y}` | number | squares |
| `line_of_sight` | list of polylines of `{x,y}` | squares |
| `objects_line_of_sight` | same | squares (furniture etc.) |
| `portals[]` | `position`, `bounds` (pairs of `{x,y}`), `rotation` (radians), `closed`, `freestanding` | squares |
| `lights[]` | `position`, `range`, `intensity`, `color` (hex, no `#`), `shadows` | squares |
| `environment` | ambient light / baked lighting flags | |
| `image` | base64 PNG/WebP | |

Importing this gives us walls, doors and lights for free from the tools people
already own. Plain **PNG/JPG + grid size** must also work (hand-drawn walls).

## Algorithm choice

Two families, both well understood:

- **Grid shadowcasting** (roguelike FOV). Symmetric shadowcasting
  ([Albert Ford's tutorial](https://www.albertford.com/shadowcasting/),
  Rust crate [`symmetric-shadowcasting`](https://lib.rs/crates/symmetric-shadowcasting))
  is exact and symmetric (if A sees B, B sees A), trivial to run per token on
  our existing square grid, and plays nicely with 5e's square-based rules.
  Walls live on cells or cell edges. Good enough for 90 % of tables and fast
  enough for phones.
- **Continuous visibility polygon** (Red Blob Games'
  [2D visibility](https://www.redblobgames.com/articles/visibility/)). Sort
  wall endpoints by angle, sweep, build a polygon; this is what Foundry and
  Owlbear draw. Produces the smooth, "premium" look and matches UVTT wall
  segments exactly (they are arbitrary line segments, not grid cells). Needs
  care at corners (cast ±ε rays per vertex).

Recommendation: **implement the polygon sweep** on wall *segments*, since UVTT
gives segments, and rasterise the result to grid cells for rules questions
("is this square visible / lit?"). One algorithm, both uses.

## Proposed design for dnd-dm

Data (all in `campaign.toml`, human-readable like everything else):

```toml
[map]
name = "Goblin caves"
image = "maps/goblin_caves.png"      # or import from .dd2vtt
width = 40
height = 30
feet_per_square = 5
pixels_per_square = 70
darkness = 0.0                       # 0 bright … 1 pitch black (Foundry-style)
fog_mode = "dynamic"                 # off | manual | dynamic

[[map.walls]]                        # polylines in squares; UVTT line_of_sight maps 1:1
points = [[3.0, 2.0], [3.0, 9.5], [8.0, 9.5]]
kind = "wall"                        # wall | terrain (blocks sight not movement) | invisible | door | secret_door | window
one_way = false

[[map.doors]]
id = "d1"
segment = [[8.0, 9.5], [9.0, 9.5]]
open = false
locked = true
secret = false

[[map.lights]]
position = [12.0, 7.0]
bright = 20                          # feet
dim = 40
color = "ff9a2e"
flicker = true
on = true

[map.fog]
revealed = ["3,2", "4,2", "…"]       # manual reveals (cells), DM brush
explored = { "bruenor" = ["…"] }     # per-character memory, optional

[[encounter.combatants]]
# existing fields plus:
vision = { range = 60, darkvision = 60, blindsight = 0, truesight = 0 }
light = { bright = 20, dim = 20 }    # carried torch
```

Behaviour:

- **DM app**: paint-mode brush for manual fog (reveal / hide, square or
  circle), wall drawing tool with snapping, door tool (click to toggle open /
  locked / secret), light tool; import `.dd2vtt`; "view as <player>" toggle;
  everything also editable in the TOML.
- **Server**: computes each PC's visible + lit cells from its position, the
  walls, doors, lights, darkness level and its own senses (darkvision ignores
  darkness within its range; dim light = disadvantage on Perception, shown as
  a shade). Sends per-character fog, so a player app never receives what it
  shouldn't know (no client-side cheating). Secret doors are plain walls to
  players until the DM flips them. Movement through a wall is refused.
- **Player app**: map image masked by that player's fog (three shades: dark /
  explored / visible), tokens only when visible, soft 2–3 px feather on fog
  edges, smooth animated reveal on move.
- **Items on the map** (the "fog of war for items" idea): map markers for
  loot, traps and notes with `visible_to = "dm" | "all" | ["bruenor"]`; the
  DM reveals them with a click. Same `hidden` idea as inventory items, so it
  is one concept across the app.

Phasing, so each step is useful on its own:

1. Image backgrounds + pixels-per-square calibration + manual fog brush
   (manual fog is 80 % of the value at 20 % of the effort).
2. Walls & doors (hand-drawn + `.dd2vtt` import), polygon visibility, per-PC
   fog pushed to player apps, explored memory.
3. Lights, darkness level, darkvision, bright/dim shading, torches carried by
   tokens, animated flicker.
4. Map markers / hidden loot, measurement templates for spells drawn on the
   image (we already have the grid math), optional elevation / multi-level.

Rough effort: phase 1 a day, phase 2 two to three days, phase 3 one to two
days, phase 4 a day. All of it is plain Rust in `dnd-core` (geometry) and
the two apps (drawing); no new heavy dependencies except an image decoder
(`image` crate) already needed for backgrounds.

## Sources

- [Roll20 — Fog of War (Classic)](https://help.roll20.net/hc/en-us/articles/360037774513-Fog-of-War-Classic-VTT)
- [Roll20 — Advanced Fog of War](https://help.roll20.net/hc/en-us/articles/360037774493-Advanced-Fog-of-War-Classic-VTT)
- [Roll20 — Dynamic Lighting](https://pages.roll20.net/dynamic-lighting)
- [Roll20 — Universal VTT support](https://help.roll20.net/hc/en-us/articles/41643201127831-Universal-Virtual-Tabletop-UVTT-Support)
- [RollQuest — dynamic fog of war](https://www.rollquest.io/features/fog-of-war)
- [Shard Tabletop — vision options](https://www.shardtabletop.com/howto/vision-options)
- [Skirmish! VTT](https://www.skirmish-vt.com/)
- [Owlbear Rodeo 2.3 — realtime dynamic fog](https://blog.owlbear.rodeo/owlbear-rodeo-2-3-release-week-day-3/)
- [Owlbear Rodeo dev log 3 (pixel-accurate fog)](https://blog.owlbear.rodeo/owlbear-rodeo-2-0-dev-log-3/)
- [Smoke & Spectre extension](https://extensions.owlbear.rodeo/smoke)
- [Foundry VTT — Lighting](https://foundryvtt.com/article/lighting/)
- [Dungeondraft — Universal VTT export](https://dungeondraft-encyclopaedia.gitbook.io/guide/final-steps/exporting-your-map/universal-vtt)
- [uvtt2fgu converter (field names)](https://github.com/Imagix/uvtt2fgu)
- [Red Blob Games — 2D visibility](https://www.redblobgames.com/articles/visibility/)
- [Albert Ford — symmetric shadowcasting](https://www.albertford.com/shadowcasting/)
- [symmetric-shadowcasting crate](https://lib.rs/crates/symmetric-shadowcasting)
- [Eric Lippert — Shadowcasting](https://ericlippert.com/2011/12/12/9067/)
- [pem2k/project-2-vtt issue #38 — dynamic lighting design notes](https://github.com/pem2k/project-2-vtt/issues/38)
