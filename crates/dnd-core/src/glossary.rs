//! Plain-language explanations of game terms for new players. The apps
//! show them behind a small (i) icon next to the thing they explain.
//! Keys are lower-case with underscores; [`explain`] also accepts spaces,
//! hyphens and capitals ("Armor Class", "sleight-of-hand").

/// The explanation for a term, if there is one.
pub fn explain(key: &str) -> Option<&'static str> {
    let k = key.trim().to_lowercase().replace([' ', '-', '/'], "_").replace('\'', "");
    ENTRIES.iter().find(|(n, _)| *n == k).map(|(_, t)| *t)
}

/// The explanation for a section heading: "Choices (fighting style…)" and
/// "Proficiencies & languages" find "choices" and "proficiencies_and_languages".
pub fn explain_heading(title: &str) -> Option<&'static str> {
    let t = title.split(" (").next().unwrap_or(title).replace(" & ", " and ").replace(['?', '…'], "");
    explain(&t).or_else(|| explain(t.trim_end_matches('s')))
}

/// Every term the glossary knows, for a "help" page or tests.
pub fn all() -> &'static [(&'static str, &'static str)] {
    ENTRIES
}

const ENTRIES: &[(&str, &str)] = &[
    // ---------------------------------------------------------- abilities
    ("abilities", "Six numbers that describe what your character is naturally good at. The big number is the score; the small +/- number is the modifier, which is what you actually add to dice rolls. 10 is average (+0); every 2 points above adds +1."),
    ("ability_score", "A score from about 3 to 20 (heroes can reach 24 or more). The modifier is (score − 10) ÷ 2, rounded down: 8 → −1, 10 → +0, 14 → +2, 18 → +4."),
    ("ability_modifier", "The bonus or penalty that comes from an ability score. You add it to checks, saving throws and attacks that use that ability."),
    ("str", "Strength: raw physical power. Used for melee weapon attacks and damage (swords, axes), climbing, jumping, swimming, shoving and how much you can carry."),
    ("dex", "Dexterity: agility, reflexes and balance. Used for ranged and finesse weapon attacks, your initiative, your Armor Class in light armor, sneaking and dodging traps."),
    ("con", "Constitution: health and stamina. Adds to your hit points every level and helps you hold concentration on spells and resist poison."),
    ("int", "Intelligence: memory, reasoning and learning. Wizards cast spells with it; it covers knowledge skills like Arcana, History and Investigation."),
    ("wis", "Wisdom: awareness, intuition and willpower. Clerics, druids and rangers cast with it; it covers Perception, Insight and resisting mind control."),
    ("cha", "Charisma: force of personality. Bards, paladins, sorcerers and warlocks cast with it; it covers persuading, lying to and intimidating people."),
    ("strength", "Strength: raw physical power. Used for melee weapon attacks and damage, climbing, jumping, swimming, shoving and carrying."),
    ("dexterity", "Dexterity: agility and reflexes. Used for ranged and finesse attacks, initiative, Armor Class in light armor and sneaking."),
    ("constitution", "Constitution: health and stamina. Adds hit points every level and helps hold concentration and resist poison."),
    ("intelligence", "Intelligence: memory and reasoning. Wizards cast with it; covers Arcana, History, Investigation, Nature and Religion."),
    ("wisdom", "Wisdom: awareness and willpower. Clerics, druids and rangers cast with it; covers Perception, Insight, Medicine and Survival."),
    ("charisma", "Charisma: force of personality. Bards, paladins, sorcerers and warlocks cast with it; covers Persuasion, Deception, Intimidation and Performance."),

    // ---------------------------------------------------------- core numbers
    ("ac", "Armor Class: how hard you are to hit. An attacker rolls a d20 plus their bonus; if the total equals or beats your AC, they hit. Armor, shields, Dexterity and magic raise it."),
    ("armor_class", "How hard you are to hit. An attacker rolls a d20 plus their bonus; if the total equals or beats your AC, they hit."),
    ("attack", "Spell attack bonus: what you add to the d20 when a spell asks for an attack roll (proficiency bonus + spellcasting modifier)."),
    ("gold", "Your gold pieces (gp), the main coin. 10 silver = 1 gold; 1 platinum = 10 gold."),
    ("armor", "Armor Class: how hard you are to hit. Attack rolls must equal or beat this number to hit you."),
    ("initiative", "Decides the turn order when a fight starts. Everyone rolls a d20 and adds this bonus (usually your Dexterity modifier); highest goes first."),
    ("init", "Initiative: your bonus to the d20 roll that decides turn order at the start of a fight. Highest goes first."),
    ("speed", "How many feet you can move on your turn. On the map one square is usually 5 feet, so 30 ft = 6 squares. Difficult terrain costs double."),
    ("proficiency_bonus", "A bonus you add to anything you're trained in: attacks with weapons you know, skills you're proficient in, your spell attacks and save DC, and some saving throws. It grows from +2 at level 1 to +6 at level 17."),
    ("prof", "Proficiency bonus: added to rolls for things you're trained in (your weapons, skills, spells and some saves). +2 at level 1, up to +6."),
    ("passive_perception", "How much you notice without trying: 10 + your Perception bonus. The DM compares it to an enemy's Stealth to see if you spot them sneaking up."),
    ("passive_investigation", "10 + your Investigation bonus. Used when the DM checks whether you notice clues or hidden details without actively searching."),
    ("passive_insight", "10 + your Insight bonus. Used when the DM checks whether you sense that someone is lying or hiding their mood."),
    ("hit_points", "HP: how much harm you can take. Damage lowers it; healing and rest restore it. At 0 you fall unconscious and start making death saving throws."),
    ("hp", "Hit points: how much harm you can take before you fall unconscious. At 0 HP you make death saves."),
    ("temp_hp", "Temporary hit points: a buffer that soaks up damage before your real HP. They don't stack (keep the higher amount), can't be healed back, and vanish after a long rest."),
    ("temp", "Temporary hit points: lost first when you take damage. They don't add together; you keep the bigger amount."),
    ("hit_dice", "Dice you can spend during a short rest to heal: roll one, add your Constitution modifier, and regain that many hit points. You get back half of them after a long rest."),
    ("hit_die", "The die your class uses for hit points: d6 for fragile casters up to d12 for barbarians. Bigger means more HP each level."),
    ("dead", "Three failed death saves (or massive damage) and the character dies. Resting and ordinary healing don't help: only magic like revivify (within a minute), raise dead or resurrection brings them back, or the DM's decision."),
    ("death_saves", "When you're at 0 HP, roll a d20 at the start of each turn: 10 or higher is a success, below 10 a failure. Three successes and you stabilize; three failures and you die. A natural 20 brings you back with 1 HP; a 1 counts as two failures. Any healing wakes you up."),
    ("inspiration", "A reward from the DM for good roleplaying. Spend it to get advantage on one attack roll, saving throw or ability check. You either have it or you don't."),
    ("level", "How experienced your character is, from 1 to 20. Levels give more hit points, new class features and, for casters, stronger spells."),
    ("xp", "Experience points: earned by overcoming challenges. Enough XP raises your level (300 for level 2, 900 for level 3, ...). Some tables level up by milestones instead."),
    ("darkvision", "You can see in darkness as if it were dim light, out to this many feet, but only in shades of gray."),
    ("carrying_capacity", "How much weight you can carry: your Strength score × 15 pounds."),

    // ---------------------------------------------------------- rolls
    ("saving_throws", "A roll to resist something bad happening to you: a trap, a spell, poison. Roll a d20 + that ability's save bonus and try to meet the DC. A filled circle means you're proficient and add your proficiency bonus."),
    ("saves", "Saving throws: d20 rolls to resist harm like spells, poison and traps. Filled circle = proficient (you add your proficiency bonus)."),
    ("skills", "Specific things your character can try, each tied to an ability. Roll a d20 + the bonus when the DM asks for a check. Filled circle = proficient, star = expertise (double proficiency). Tap a skill to roll it."),
    ("ability_check", "When you try something with an uncertain outcome, roll a d20 + the relevant bonus. Meet or beat the DC the DM sets and you succeed."),
    ("dc", "Difficulty Class: the number you need to reach. 10 is easy, 15 is medium, 20 is hard, 25+ is very hard."),
    ("proficient", "You're trained in this, so you add your proficiency bonus to the roll."),
    ("expertise", "Extra training: you add double your proficiency bonus to this skill."),
    ("advantage", "Roll two d20s and use the higher one. You get it when circumstances help you (an ally distracts the enemy, the target can't see you...)."),
    ("disadvantage", "Roll two d20s and use the lower one. You get it when circumstances hurt you (you can't see the target, you're poisoned...). Advantage and disadvantage cancel out."),
    ("roll_mode", "Normal rolls one d20. Advantage rolls two and keeps the higher; disadvantage rolls two and keeps the lower. Use what the DM or your features say."),
    ("attack_roll", "Roll a d20 + your attack bonus. If the total equals or beats the target's Armor Class, you hit and roll damage. A natural 20 always hits and is a critical hit; a natural 1 always misses."),
    ("attacks", "Your weapon attacks. 'Hit' is what you add to the d20 to try to beat the target's Armor Class; 'Damage' is what you roll if you hit. Pick a target and tap Attack."),
    ("damage_roll", "Rolled after you hit: the weapon or spell's dice plus your modifier. The target loses that many hit points."),
    ("critical_hit", "Rolling a 20 on the d20 for an attack. It always hits, and you roll all the damage dice twice."),
    ("dice", "Dice are written like 2d6 + 3: roll two six-sided dice and add 3. d20 is the big one for attacks, checks and saves. '4d6kh3' means roll four, keep the highest three."),
    ("dice_notation", "XdY means roll X dice with Y sides and add them: 1d8 is one eight-sided die, 3d6 is three six-sided dice. Add or subtract numbers after it. 'kh' keeps the highest dice, 'kl' the lowest."),

    // ---------------------------------------------------------- turns and actions
    ("turn", "In a fight, everyone takes a turn in initiative order. On your turn you can move up to your speed and take one action, and maybe a bonus action. One round of everyone's turns is about 6 seconds."),
    ("round", "One full cycle of turns where everyone in the fight acts once. About 6 seconds of game time."),
    ("action", "The main thing you do on your turn: Attack, Cast a Spell, Dash (double move), Disengage (move safely away), Dodge, Help, Hide, Ready or Use an Object."),
    ("bonus_action", "A quick extra thing you can do on your turn, but only if a feature or spell says it uses a bonus action. One per turn."),
    ("reaction", "Something you do instantly in response to a trigger, even on someone else's turn, like an opportunity attack when an enemy walks away from you. One per round."),
    ("opportunity_attack", "When an enemy you can reach moves out of your reach, you can use your reaction to make one melee attack against it. Taking the Disengage action avoids this."),
    ("end_turn", "Tell the table you're done. The tracker moves on to the next creature in initiative order."),
    ("target", "The creature you're attacking or casting at. You can only pick creatures your character can see."),
    ("reach", "How far away you can hit with a melee attack: usually 5 feet (the squares next to you), 10 feet with a reach weapon like a glaive or whip."),
    ("range", "How far a ranged weapon or spell reaches. '80/320' means normal range 80 feet; up to 320 feet you attack with disadvantage."),
    ("cover", "Something between you and an attacker. Half cover (a low wall) gives +2 AC and Dex saves; three-quarters cover gives +5; full cover means you can't be targeted directly."),

    // ---------------------------------------------------------- rests
    ("short_rest", "At least an hour of downtime. You can spend hit dice to heal, and some features recharge (Second Wind, Action Surge, warlock spell slots...)."),
    ("long_rest", "About 8 hours of sleep or light activity, once a day. You regain all hit points, spell slots, half your hit dice and most features."),
    ("rest", "Short rest: an hour; spend hit dice, some features come back. Long rest: 8 hours; everything comes back. You can only benefit from one long rest per day."),
    ("uses", "How many times you can use this before it needs a rest to recharge. The recharge says which rest brings it back."),
    ("recharge", "When a used-up feature comes back: 'short rest' means after any rest of an hour or more, 'long rest' only after a full night's rest."),

    // ---------------------------------------------------------- spells
    ("spellcasting", "Your magic. Spell attack is what you add to a d20 when a spell asks for an attack roll; save DC is the number enemies must reach to resist your spells."),
    ("spell_attack", "What you add to the d20 when a spell asks for an attack roll: your proficiency bonus + your spellcasting ability modifier."),
    ("spell_save_dc", "The number a target must reach on its saving throw to resist your spell: 8 + your proficiency bonus + your spellcasting ability modifier."),
    ("save_dc", "The number a target must reach on its saving throw to resist your spell: 8 + proficiency bonus + spellcasting modifier."),
    ("spell_slots", "Your daily 'fuel' for spells. Casting a spell of 1st level or higher uses up a slot of that level or higher. Slots come back after a long rest (warlocks: after a short rest too)."),
    ("slots", "Spell slots: each leveled spell you cast spends one slot of its level or higher. They return after a long rest."),
    ("pact_slots", "Warlock spell slots: few, all the same level, and they come back after a short rest."),
    ("cantrip", "A level 0 spell you can cast as often as you like, without using a spell slot. Damage cantrips get stronger at levels 5, 11 and 17."),
    ("cantrips", "Level 0 spells: cast them as often as you like, no spell slot needed. They grow stronger at character levels 5, 11 and 17."),
    ("spell_level", "How powerful a spell is, from 1 to 9 (cantrips are 0). You need a spell slot of at least that level to cast it. Not the same as your character level."),
    ("prepared", "The spells you have ready today. Clerics, druids, paladins and wizards choose them after each long rest; you can only cast prepared spells (plus 'always' ones)."),
    ("prepare", "Tick the spells you want ready today. You can change the list after a long rest. The number you can prepare comes from your level and spellcasting ability."),
    ("known", "Spells you've learned. Bards, rangers, sorcerers and warlocks always have their known spells ready; wizards keep theirs in a spellbook and prepare some each day."),
    ("always_prepared", "Granted by a feature (your domain, oath, circle or race): always ready, and doesn't count against how many spells you can prepare."),
    ("spellbook", "A wizard's book of spells. You prepare your daily spells from it, and can copy new spells into it when you find them."),
    ("concentration", "Some spells last only while you focus on them. You can concentrate on one spell at a time; taking damage forces a Constitution save (DC 10 or half the damage) or the spell ends."),
    ("ritual", "A ritual spell can be cast without a spell slot if you take 10 extra minutes to cast it. Handy for things like Detect Magic outside combat."),
    ("upcast", "Casting a spell using a higher-level slot than it needs. Many spells get stronger: more damage, more targets, longer duration."),
    ("components", "What casting needs: V = speak words, S = gestures with a free hand, M = materials (a component pouch or spellcasting focus covers ones without a gold cost)."),
    ("casting_time", "How long the spell takes: most take your action; some a bonus action or reaction; rituals and big spells take minutes or hours."),
    ("duration", "How long the spell lasts. 'Instantaneous' happens and is done; 'Concentration' ends early if your concentration breaks."),
    ("school", "The kind of magic: abjuration (protection), conjuration (summoning), divination (knowledge), enchantment (minds), evocation (energy), illusion (tricks), necromancy (life and death), transmutation (change)."),
    ("spell_choice", "Pick the spells your character learns. The numbers show how many cantrips and leveled spells you can still choose."),

    // ---------------------------------------------------------- building a character
    ("species", "Your character's people (also called race): elf, dwarf, human... It gives ability bonuses, speed, senses like darkvision and special traits."),
    ("class", "Your character's calling: fighter, wizard, rogue... It decides your hit points, what you're trained in, your features and whether and how you cast spells."),
    ("subclass", "A specialty within your class (a cleric's domain, a fighter's archetype...). You choose it at the level your class says, and it adds extra features."),
    ("background", "What your character did before adventuring (acolyte, soldier...). It gives two skills, maybe tools or languages, and a small feature."),
    ("ability_method", "How you get your six scores. Standard array: assign 15, 14, 13, 12, 10, 8. Point buy: spend 27 points on scores from 8 to 15. Rolled: the server rolls 4d6 (keep the best 3) six times for you."),
    ("standard_array", "Six fixed scores — 15, 14, 13, 12, 10 and 8 — that you assign to the abilities however you like. Put the highest where your class needs it."),
    ("point_buy", "Every score starts at 8 and you spend 27 points to raise them, up to 15. Higher scores cost more points."),
    ("rolled", "The server rolls 4d6 six times and keeps the best three dice each time; you assign the results to your abilities."),
    ("improvements", "At certain levels your class lets you raise one ability by 2, or two abilities by 1 each (to a maximum of 20), or take a feat instead."),
    ("ability_score_improvement", "At certain class levels, raise one ability by 2 or two abilities by 1 (max 20), or take a feat instead."),
    ("feat", "A special talent that gives a new ability or bonus, such as Tough (more hit points) or Alert (better initiative). Taken instead of an ability score improvement."),
    ("choices", "Some features let you pick an option: a fighting style, warlock invocations, a dragon ancestry... Each pick's description says what it does."),
    ("features", "Special abilities from your class, subclass, species, background and feats. Some can only be used a few times before a rest."),
    ("traits", "Special abilities that come from your species, like darkvision, resistance to poison or extra proficiencies."),
    ("proficiencies", "What you're trained to use: armor (you need it to wear armor well), weapons (add your proficiency bonus to attacks), tools and languages."),
    ("languages", "The languages your character can speak, read and write."),
    ("suggested_characteristics", "Ideas from your background for who your character is: personality traits (pick two), an ideal (what you believe), a bond (who or what you care about) and a flaw (what gets you in trouble). Tap one to use it, roll for a random one, or write your own."),
    ("personality", "How your character acts and talks: a couple of habits or quirks other people notice."),
    ("ideals", "What your character believes in and fights for. The word in brackets is the alignment it suggests."),
    ("bonds", "The people, places or things your character cares about most."),
    ("flaws", "A weakness or vice that gets your character into trouble. Good roleplaying of flaws can earn inspiration."),
    ("starting_equipment", "The gear your class and background give you at the start. Where a line says 'or', pick one option."),
    ("multiclass", "Taking levels in a second class. You need 13 or more in the key abilities of both classes."),
    ("player_name", "The real person who plays this character."),
    ("alignment", "Your character's moral outlook, like lawful good or chaotic neutral. A roleplaying guide, not a rule."),

    // ---------------------------------------------------------- items
    ("equipped", "Being worn or held. Only equipped armor and weapons count toward your Armor Class and attacks."),
    ("attuned", "Some magic items only work after you spend a short rest bonding with them. You can be attuned to at most three items at once."),
    ("attunement", "Some magic items only work after you spend a short rest bonding with them. You can be attuned to at most three items at once."),
    ("weight", "How heavy your gear is in pounds. Carrying more than your capacity (Strength × 15) isn't allowed."),
    ("coins", "Money: 10 copper (cp) = 1 silver (sp); 10 silver = 1 gold (gp); 1 platinum (pp) = 10 gold. Electrum (ep) is worth half a gold piece."),
    ("currency", "Money: 10 cp = 1 sp; 10 sp = 1 gp; 1 pp = 10 gp; 1 ep = half a gp."),
    ("rarity", "How rare and powerful a magic item is: common, uncommon, rare, very rare, legendary or artifact."),
    ("finesse", "You can use Strength or Dexterity, whichever is better, for attack and damage rolls with this weapon."),
    ("versatile", "Can be used with one or two hands; two hands deal the larger damage shown."),
    ("ammunition", "Needs ammunition (arrows, bolts, bullets, needles) to make a ranged attack; each attack uses one. After a fight you can recover half of what you shot. Loading it is part of the attack."),
    ("heavy", "Big and bulky: Small and Tiny creatures attack with it at disadvantage."),
    ("light", "Small and easy to handle. Holding a light melee weapon in each hand lets you attack with the second one as a bonus action (without adding your modifier to its damage, unless it's negative)."),
    ("loading", "Slow to reload: you fire it only once per action, bonus action or reaction, even if you could normally attack more than once."),
    ("thrown", "You can throw it to make a ranged attack, using the same ability as for a melee attack with it (Strength, or Dexterity if it's also finesse)."),
    ("two_handed", "Takes both hands to attack with."),
    ("special", "Has its own rules, given in the weapon's description (lance, net, double-bladed scimitar)."),
    ("weapon_properties", "Tags on a weapon that change how it works: finesse, light, heavy, reach, thrown, two-handed, versatile, ammunition, loading. Tap one to read what it does."),

    // ---------------------------------------------------------- conditions
    ("conditions", "Temporary states that change what a creature can do: poisoned, prone, stunned... Tap one to see what it means."),
    ("blinded", "Can't see: automatically fails checks that need sight. Attacks against it have advantage; its attacks have disadvantage."),
    ("charmed", "Can't attack the charmer or target it with harmful magic, and the charmer has advantage on social checks with it."),
    ("deafened", "Can't hear and automatically fails checks that need hearing."),
    ("frightened", "Disadvantage on checks and attacks while it can see the source of its fear, and it can't willingly move closer to it."),
    ("grappled", "Speed becomes 0. It ends if the grappler is incapacitated or the creature is moved out of reach."),
    ("incapacitated", "Can't take actions or reactions."),
    ("invisible", "Can't be seen without magic. Attacks against it have disadvantage; its attacks have advantage."),
    ("paralyzed", "Incapacitated and can't move or speak. Fails Strength and Dexterity saves; attacks against it have advantage, and hits from within 5 feet are critical hits."),
    ("petrified", "Turned to stone: incapacitated, can't move or speak, resists all damage, immune to poison and disease."),
    ("poisoned", "Disadvantage on attack rolls and ability checks."),
    ("prone", "Lying on the ground. It can only crawl, or spend half its movement to stand. Its attacks have disadvantage; melee attacks against it have advantage, ranged attacks disadvantage."),
    ("restrained", "Speed 0. Attacks against it have advantage; its attacks have disadvantage, and it has disadvantage on Dexterity saves."),
    ("stunned", "Incapacitated, can't move and can barely speak. Fails Strength and Dexterity saves; attacks against it have advantage."),
    ("unconscious", "Knocked out: drops what it holds and falls prone. Fails Strength and Dexterity saves; attacks against it have advantage, and hits from within 5 feet are critical hits."),
    ("exhaustion", "Built-up fatigue in six levels: 1 disadvantage on checks; 2 speed halved; 3 disadvantage on attacks and saves; 4 HP maximum halved; 5 speed 0; 6 death. A long rest removes one level."),
    ("concentrating", "Holding a concentration spell. Taking damage means a Constitution save to keep it going."),

    // ---------------------------------------------------------- damage
    ("damage_types", "Damage comes in kinds — slashing, piercing, bludgeoning, fire, cold, lightning, thunder, acid, poison, radiant, necrotic, force, psychic. Resistance halves a type, vulnerability doubles it, immunity ignores it."),
    ("resistance", "Takes half damage of that type."),
    ("vulnerability", "Takes double damage of that type."),
    ("immunity", "Takes no damage of that type (or isn't affected by that condition)."),

    // ---------------------------------------------------------- the table
    ("encounter", "A fight or tense scene run in turns. The DM adds the creatures, everyone rolls initiative, then each acts in order."),
    ("initiative_order", "The list of who acts when. Highest initiative goes first; the marker shows whose turn it is."),
    ("distance_rule", "How diagonal moves are counted on the grid. Chebyshev: every square is 5 ft, diagonals too (the 5e default). Alternating: diagonals cost 5, then 10, then 5... Euclidean: true straight-line distance."),
    ("area_of_effect", "The shape a spell covers: sphere (radius from a point), cube, cone (spreads from you), line or cylinder. Everyone inside is affected."),
    ("fog", "Fog of war: players only see the parts of the map their characters can see or have already explored."),
    ("map", "The battle map. Drag your token to move (up to your speed); doors open with a tap. You only see what your character can see."),
    ("table_feed", "Everything that happens at the table: rolls, attacks, spells and turns, so everyone sees the same results."),
    ("dm_notes", "Private notes only the DM sees. Never sent to players."),
    ("hidden", "Hidden from players: their app doesn't show it at all, but it still works (a hidden magic ring still protects its wearer)."),
    ("modifiers", "Bonuses and penalties added on top of the normal rules: +1 AC from a blessing, a house rule, a curse. Each says what it changes and by how much."),
    ("join_code", "A password players type to join the game from their app. Leave it empty to let anyone on your Wi-Fi join."),
    ("dm_code", "A password that lets a second DM app make DM-only changes over the network. 'off' disables remote DM changes."),

    // ---------------------------------------------------------- section headings
    ("ability_scores", "Your six abilities: Strength, Dexterity, Constitution, Intelligence, Wisdom and Charisma. Species, feats and improvements add to the base scores you choose here."),
    ("advancements", "Ability score improvements or feats taken at the levels your class allows (usually 4, 8, 12, 16 and 19)."),
    ("bonus_feats", "Feats the character has that didn't come from an ability score improvement, for example a gift from the DM."),
    ("skill_proficiencies", "The skills your character is trained in. Your class lets you pick some; your background and species may give more."),
    ("proficiencies_and_languages", "What your character is trained to use — armor, weapons, tools — and the languages they know."),
    ("other_proficiencies", "Tools and languages beyond what your class, species and background already give."),
    ("features_and_traits", "Everything special your character can do, from class, subclass, species, background and feats. Numbers like '2/short rest' mean limited uses."),
    ("limited_uses", "Features you can only use a few times before a rest. Tick them off as you use them; a rest brings them back."),
    ("inventory", "Everything you carry. Equip armor and weapons to use them; magic items that need attunement must be attuned to work."),
    ("spells", "Your spells, grouped by level. Cantrips are free; leveled spells need a spell slot. Tap a spell to read it, then Cast."),
    ("spell_list", "Every spell your class can use. Known/Prep columns show which ones the character has learned and prepared."),
    ("spell_slots_used", "How many spell slots of each level have been spent since the last long rest."),
    ("starting_gear", "The equipment your class and background give you. Where there's a choice, tap the option you want."),
    ("hit_dice_spent", "Hit dice used for healing since the last long rest. A long rest gives back half of your total."),
    ("selected_combatant", "The creature picked in the tracker. Change its hit points, conditions and position here, or make it attack."),
    ("add_combatants", "Put player characters and monsters into the fight. Then roll initiative and start."),
    ("distance", "The distance in feet between two creatures on the grid, using the distance rule you picked."),
    ("within_radius", "Lists every creature within this many feet of the chosen one — handy for auras and explosions."),
    ("at_the_table", "The shared feed: rolls, attacks, spells and turns everyone at the table can see."),
    ("roll", "Type dice like 1d20+5 or 2d6 and roll them; the result goes to the table feed."),
    ("identity", "Who the character is: name, player, alignment, experience, species and background."),
    ("classes", "The character's class (or classes, if multiclassing), subclass and level in each."),
    ("custom_modifiers", "Bonuses the DM or a house rule adds by hand: a blessing, a curse, a magic boon. They change the sheet like any other modifier."),
    ("modifier_trace", "Shows every bonus that went into each number and where it came from, so you can see why your AC or a save is what it is."),
    ("always_on_modifiers", "Bonuses this entry always gives, such as +2 Dexterity for a species."),
    ("modifiers_while_equipped", "Bonuses this item gives only while it is equipped (and attuned, if it needs attunement)."),
    ("weapon_stats", "Damage dice and type, simple or martial, melee or ranged, and properties like finesse or reach."),
    ("armor_stats", "Base AC, armor category (light, medium, heavy, shield), Dexterity cap and Strength requirement."),
    ("library", "How many of each kind of content the loaded packs contain: species, classes, spells, monsters..."),
    ("content_packs", "Folders of TOML files with species, classes, spells and monsters. Base is the SRD; Homebrew holds your own additions and changes."),
    ("players_connect_to", "Start the server so players' phones on the same Wi-Fi can join. Share the join code if you set one."),
    ("players_at_the_table", "Phones that have joined. Give each player their character from the drop-down."),
    ("send_to_the_dm", "When your character is ready, send it to the DM's app. The DM's server checks it follows the rules."),
    ("between_sessions", "Make or rebuild your character here while no game is running. During a game the DM assigns characters."),
    ("who_are_you", "Type your name so the DM can see who's at the table and give you your character."),
    ("find_the_dm_on_this_wi_fi", "Your phone looks for the DM's computer on the same Wi-Fi network. Tap it when it appears."),
    ("maps", "The campaign's maps. The one marked ▶ is in play; switch to another to move the party there."),

    // ---------------------------------------------------------- skills
    ("acrobatics", "Acrobatics (Dexterity): keeping your balance, tumbling, flips and wriggling out of a grab."),
    ("animal_handling", "Animal Handling (Wisdom): calming, controlling or reading an animal; staying on a spooked horse."),
    ("arcana", "Arcana (Intelligence): knowledge of magic, spells, magic items, planes and magical creatures."),
    ("athletics", "Athletics (Strength): climbing, jumping, swimming, grappling and shoving."),
    ("deception", "Deception (Charisma): lying convincingly, disguises, misleading people."),
    ("history", "History (Intelligence): knowledge of past events, kingdoms, wars and legends."),
    ("insight", "Insight (Wisdom): reading people — telling whether someone is lying or what they really want."),
    ("intimidation", "Intimidation (Charisma): getting your way through threats and scary presence."),
    ("investigation", "Investigation (Intelligence): searching for clues, working out how something works, finding hidden details."),
    ("medicine", "Medicine (Wisdom): stabilizing a dying friend and diagnosing illness."),
    ("nature", "Nature (Intelligence): knowledge of plants, animals, terrain and weather."),
    ("perception", "Perception (Wisdom): noticing things — spotting a hidden enemy, hearing footsteps, seeing a trap."),
    ("performance", "Performance (Charisma): entertaining with music, dance, acting or storytelling."),
    ("persuasion", "Persuasion (Charisma): convincing people honestly — negotiating, charming, inspiring."),
    ("religion", "Religion (Intelligence): knowledge of gods, rites, holy symbols and cults."),
    ("sleight_of_hand", "Sleight of Hand (Dexterity): picking pockets, palming objects, hand tricks."),
    ("stealth", "Stealth (Dexterity): sneaking and hiding without being noticed."),
    ("survival", "Survival (Wisdom): tracking, hunting, finding your way and surviving in the wild."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups_are_forgiving_and_keys_unique() {
        assert!(explain("Armor Class").is_some());
        assert!(explain("sleight-of-hand").is_some());
        assert!(explain("nonsense").is_none());
        let mut keys: Vec<&str> = all().iter().map(|(k, _)| *k).collect();
        keys.sort();
        let n = keys.len();
        keys.dedup();
        assert_eq!(n, keys.len(), "duplicate glossary key");
        for (k, t) in all() {
            assert!(!t.is_empty() && k.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{k}");
        }
    }

    #[test]
    fn headings_the_apps_use_are_explained() {
        for h in [
            "Saving throws", "Skills", "Proficiencies & languages", "Attacks", "Limited uses", "Conditions", "Rest", "Spell slots",
            "Spellcasting", "Ability scores (base, before species/feat bonuses)", "Choices (fighting style, invocations, ancestry…)",
            "Improvements", "Species", "Class", "Background", "Starting gear", "Inventory", "Coins", "Area of effect", "Within radius",
            "Distance", "Add combatants", "Selected combatant", "Who are you?", "Find the DM on this Wi-Fi", "Cantrips",
        ] {
            assert!(explain_heading(h).is_some(), "heading {h}");
        }
    }

    #[test]
    fn every_skill_condition_and_ability_is_explained() {
        let rules = crate::model::Rules::default();
        for s in &rules.skills {
            assert!(explain(&s.key).is_some(), "skill {}", s.key);
        }
        for c in &rules.conditions {
            assert!(explain(c).is_some(), "condition {c}");
        }
        for a in crate::model::ABILITY_KEYS {
            assert!(explain(a).is_some(), "ability {a}");
        }
    }
}
