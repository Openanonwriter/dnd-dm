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

## Death

Three failed death saves and the character is **dead** (the DM sees a red
DEAD label, the player "Your character has died"). The dead can't rest,
spend hit dice or roll death saves, and ordinary healing (cure wounds,
potions, the +5 buttons) does nothing for them; the server refuses it all.
Only a revival spell brings them back: *revivify* or *raise dead* at 1 hit
point, *reincarnate*, *resurrection* or *true resurrection* at full. The DM
can also override it with *Bring back to life* on the character's Notes &
State tab. A long rest needs at least 1 hit point even for the living.

Dashboard → *Players' attacks change hit points*: turn this off and attacks
and spells are only rolled and announced; the DM applies the numbers.

## Reach, range and movement

With Dashboard → *Enforce reach, range and movement* on (the default), the
DM's PC checks every attack and spell against the map:

- **Melee** reaches 5 ft, 10 ft with a reach weapon. **Thrown** weapons work
  in melee and out to their range. **Ranged** weapons and spells have a
  normal range and, for weapons, a long range (a longbow is 150/600).
- Past normal range the roll has **disadvantage**; past long range it is
  refused. A ranged attack with an enemy within 5 ft has disadvantage.
- **Walls and closed doors** block attacks and spells: there must be a clear
  line between the two tokens.
- **Spells**: *Touch* means next to the target, *Self (15-foot cone)* means
  within 15 ft, an area spell reaches its range plus its radius. A spell that
  cannot reach is refused before a slot is spent.
- **Conditions** change the roll by themselves: a blinded, poisoned,
  frightened, restrained or prone attacker has disadvantage; a blinded,
  paralyzed, restrained, stunned or unconscious target gives advantage; a
  prone target gives advantage up close and disadvantage from afar; a hit on
  a paralyzed or unconscious creature within 5 ft is a critical. Someone
  stunned, paralyzed, unconscious or at 0 hit points cannot attack or cast.
- **Movement** in a fight is the character's speed per turn (the squares in
  reach are tinted on the phone's map). *Dash* doubles it. Grappled,
  restrained, stunned or paralyzed means no movement.

The phone shows the distance to each target and greys out attacks that
cannot reach. Every number comes from the sheet, so anything that changes
speed, reach or range there (a level, a feat, an item, a DM modifier with
target `speed`, `attack.reach`, `attack.range` or `spell.range`) changes
what is allowed. Tokens that are not on the map are not checked.

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
| Whole list | Cleric, druid, paladin, artificer | cantrips only | from the whole class list |
| Known | Bard, ranger, sorcerer, warlock | cantrips + `spells_known` per level | nothing: all are ready |

Spells a feature grants (a cleric's domain spells, a paladin's oath, a
Land druid's terrain, racial magic) are marked **always** and never count
against the limits. The DM can always add any spell to a character by hand;
those stay.

## Rules the apps enforce

Everything a character has is checked against what it qualifies for:
subclass level, class options with prerequisites (an invocation that needs
Pact of the Blade), feat prerequisites, multiclass ability scores, spells
above the castable level or off the list, the number of cantrips, known and
prepared spells, class skill picks, ability score improvements at the right
levels, and armor the character is proficient with and strong enough for.

- **DM**: anything goes. The Build tab starts with a *Rules check* list of
  what is broken, and each offending pick has a red outline with the reason
  on hover. Drop-downs show options the character doesn't qualify for in
  red.
- **Players**: red-outlined options can't be picked; tapping one says what
  it needs. The server refuses a character or spell choice that breaks a
  rule.
- **Allowing it**: Dashboard → *Players may break the rules* (everyone), or
  *Let this character's player pick options they don't qualify for* on a
  character's Build tab. The picks still show in red, and the player's
  Spells tab gains *More spells your DM allows*.

## Help for new players

Every stat, skill, condition, section heading, class, subclass, species,
background, feat, item and spell has a small **(i)**. Hover it on the PC for
a short explanation; tap it on the phone for the full text in a popup (tap
elsewhere to close). The wording lives in `crates/dnd-core/src/glossary.rs`
(game terms) and is built from the content files for everything else.

## Trying it alone

```text
cargo run --release -p dnd-player --example demo
```

opens the player app against a small fight running inside the same program
(nothing is opened to the network). `DND_PLAYER_TAB=spells` starts on a tab;
`DND_PLAYER_CREATE="dwarf,cleric,acolyte"` opens the character builder with
those picks. See `docs/DEVELOPING.md` for the rest.
