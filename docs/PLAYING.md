# Playing a session

What the apps do during a game, and who presses what. The DM's PC decides
everything (dice, hits, damage); phones only ask.

## Before the game

1. **DM**: open `dnd-dm`, pick or make a campaign, start the server (top bar).
2. **Players**: open `dnd-player`, type a name, connect. Each makes a
   character (species, class, scores, skills, gear, **spells**) and sends it.
   A player who already has a character can still swap chosen spells on the
   Spells tab until the game starts.
3. **DM**: Dashboard → *Start game*. From then on the DM hands out
   characters, and players can add spells (after a level-up) but not swap.

## Exploring

- Map tab on the phone: drag your token or tap a square to walk, tap a door
  to open it. Each player sees only what their character sees.
- Any roll from the sheet (skill, save, attack, the Dice tab) is rolled on
  the DM's PC and shows up for everyone under **Dice → At the table**, and
  for the DM at the bottom of the Encounter screen.
- *Next d20: normal / advantage / disadvantage* applies to the next roll and
  then goes back to normal.

## A fight

1. **DM**: Encounter → add the party (*All PCs*) and the creatures, place
   them on the Map tab, *Roll initiative (all)*, *Start*.
2. The table feed announces each turn. The player whose turn it is sees
   **Your turn** and an **End turn** button on every tab.
3. **Attacking (player)**: tap a creature on the map (or pick it under
   *Target* on the Combat tab), then press an attack. The DM's PC rolls to
   hit against the target's armor class, rolls damage (doubled dice on a
   20), halves or cancels it for resistances and immunities, and takes it off
   the target. Without a target the buttons just roll the dice.
4. **Casting (player)**: Spells tab → *Cast*. A slot is spent by itself
   (the ⬆ menu casts with a bigger slot, or as a ritual). With targets
   picked: attack spells roll to hit each one, save spells make each target
   roll its save against your DC, healing heals, and anything else is
   announced. Concentration is marked on the caster.
5. **Creatures (DM)**: click the creature in the tracker, pick a *target*
   and press one of its attacks. Damage lands on the character's sheet and
   phone at once. *Next ▶* moves on (dead creatures are skipped).
6. **At 0 hit points** a player gets *Roll death save*: three passes make
   them stable, three fails kill, a 20 puts them back up with 1.

Dashboard → *Players' attacks change hit points*: turn this off and attacks
and spells are only rolled and announced; the DM applies the numbers.

## Resting and levelling

- Combat tab → *Spend a d8* (a hit die: rolled, plus Constitution),
  *Short rest*, *Long rest*. The DM has *Short rest (party)* and *Long rest
  (party)* on the Dashboard. A long rest restores hit points, spell slots,
  limited features and half the hit dice.
- Features with a number of uses (Second Wind, Channel Divinity, Arcane
  Recovery…) are dots under **Limited uses** on the Combat tab.
- **Level up**: the DM raises the level on the Characters screen. If the new
  level brings spells, the player's Spells tab shows **Learn** with what
  they may pick.

## Who knows which spells

| Class kind | Example | Chooses | Prepares |
|---|---|---|---|
| Spellbook | Wizard | cantrips + 6 spells, 2 more per level | from the book, each long rest |
| Whole list | Cleric | cantrips only | from the whole class list |
| Known | (homebrew sorcerer, bard) | cantrips + `spells_known` per level | nothing: all are ready |

The DM can always add any spell to a character by hand; those stay.

## Trying it alone

```text
cargo run --release -p dnd-player --example demo
```

opens the player app against a small fight running inside the same program
(nothing is opened to the network). `DND_PLAYER_TAB=spells` starts on a tab.
