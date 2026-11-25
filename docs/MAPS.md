# Maps

A campaign has any number of maps. One is **in play** (the party is there and
players see it); the rest sit on the shelf, grouped by chapter, and can be
opened and prepared without moving the party. Pressing ▶ next to a map moves
the game there: creatures on the old map stay behind with it, the ones
prepared on the new map come out, and the player characters arrive at its
start point (⚑).

## Where they are saved

```text
saves/<campaign>/campaign.toml      the map in play, under [map]
saves/<campaign>/maps/<id>.toml     every other map
```

Drop a map file into `maps/` and it is on the shelf the next time the
campaign loads. A map is plain text:

```toml
name = "Cellar"
chapter = "Chapter 1"
width = 7
height = 5
feet_per_square = 5
fog = "vision"            # off | manual | vision
darkness = 1.0            # 0 daylight, 0.5 dim, 1 pitch black
cells = [
  "#######",
  "#..#..#",
  "#.....#",
  "#..#~~#",
  "#######",
]

[start]
x = 1
y = 2

[[walls]]                 # measured in squares from the top-left corner
x1 = 3.0
y1 = 2.0
x2 = 3.0
y2 = 3.0
kind = "door"             # wall | window | door | secret_door
locked = true

[[lights]]
x = 5.5
y = 1.5
bright = 20               # feet; dim reaches `dim` feet
dim = 40
color = "ff9a2e"

[[props]]
kind = "chest"
x = 5.5
y = 3.5
hidden = true             # only the DM sees it until revealed

[[markers]]
x = 1
y = 1
text = "ambush here"
hidden = true
```

Terrain characters: `#` solid rock (a wall: blocks sight and movement), `.`
stone floor, `=` wooden floor, `:` dirt, `,` grass, `_` sand, `*` snow, `~`
water, `^` rubble, `%` lava. Water, rubble and snow cost double movement.

Props: chest, barrel, crate, table, chair, bed, bookshelf, pillar, statue,
altar, stairs, campfire, tree, bush, boulder, well, trap, rug, bones, portal.

## Fog of war

- **Off**: players see the whole map.
- **Painted by me** (`manual`): players see only what the DM reveals with the
  Fog brush.
- **What characters see** (`vision`): rock, walls and closed doors block
  sight; in the dark a character sees only lit squares and as far as its
  darkvision. Each player sees through their own character; squares seen
  before stay on the map, dimmed. The party's memory is `explored` in the file.

The server works this out and sends each player app only what that player may
know (`GET /api/player/map`): unseen squares are `?`, hidden props, DM notes,
closed secret doors and out-of-sight creatures are not sent at all.

## Players moving

A player drags their own token (or taps a square). The server finds a way
round walls, rock and closed doors and refuses if there is none, if it is
further than the character's speed (four times that when no fight is on), or
if it is not their turn. Tapping a door next to the character opens or closes
it unless it is locked. Both can be switched off with
`players_move_tokens = false` under `[settings]`.

## Tools for checking

```text
cargo run -p dnd-core --example demo_campaign -- content saves
cargo run --release -p dnd-mapview --example snapshot -- content saves/<campaign> out.bmp dm
```

The first writes a demo campaign with a dungeon, a cave and a forest road; the
second draws a campaign's map to a picture without opening a window.
