//! Sound library — comprehensive sound event list.

/// Major sound categories.
pub fn all_sound_events() -> &'static [&'static str] {
    &[
        // Blocks
        "dig.stone", "dig.wood", "dig.gravel", "dig.grass", "dig.sand", "dig.cloth", "dig.glass", "dig.snow",
        "step.stone", "step.wood", "step.gravel", "step.grass", "step.sand", "step.cloth", "step.snow",
        "random.door_open", "random.door_close", "random.door_bump",
        "random.click", "random.chest.open", "random.chest.close",
        "random.fizz", "random.fuse", "random.explode", "random.orb",
        "random.levelup", "random.eat", "random.burp", "random.drink",
        "random.anvil_break", "random.anvil_land", "random.anvil_use",
        // Mobs
        "mob.zombie.say", "mob.zombie.hurt", "mob.zombie.death", "mob.zombie.attack_door",
        "mob.skeleton.say", "mob.skeleton.hurt", "mob.skeleton.death",
        "mob.creeper.say", "mob.creeper.death",
        "mob.spider.say", "mob.spider.hurt", "mob.spider.death",
        "mob.enderman.idle", "mob.enderman.hit", "mob.enderman.death", "mob.enderman.portal",
        "mob.villager.idle", "mob.villager.hurt", "mob.villager.death", "mob.villager.hit",
        "mob.pig.say", "mob.pig.hurt", "mob.pig.death",
        "mob.sheep.say", "mob.sheep.hurt", "mob.sheep.death",
        "mob.cow.say", "mob.cow.hurt", "mob.cow.death",
        "mob.chicken.say", "mob.chicken.hurt", "mob.chicken.death", "mob.chicken.plop",
        "mob.blaze.breathe", "mob.blaze.hit", "mob.blaze.death", "mob.blaze.fire",
        "mob.ghast.moan", "mob.ghast.scream", "mob.ghast.death", "mob.ghast.fireball",
        "mob.wolf.bark", "mob.wolf.howl", "mob.wolf.hurt", "mob.wolf.death", "mob.wolf.shake",
        "mob.cat.meow", "mob.cat.purr", "mob.cat.hiss",
        "mob.horse.idle", "mob.horse.hurt", "mob.horse.death", "mob.horse.armor",
        "mob.dolphin.eat", "mob.dolphin.splash", "mob.dolphin.play",
        // Environment
        "ambient.weather.rain", "ambient.weather.thunder", "ambient.weather.lightning",
        "ambient.cave", "ambient.nether",
        // Player
        "game.player.hurt", "game.player.die", "game.player.swim", "game.player.splash",
        "game.player.attack.nodamage", "game.player.attack.strong", "game.player.attack.weak",
        "game.player.hurt.fall.small", "game.player.hurt.fall.big",
        // Items
        "random.bow", "random.bowhit", "random.arrow.hit", "random.break",
        "note.bass", "note.hat", "note.snare", "note.bassattack", "note.harp",
        "random.toast", "random.eating",
    ]
}

/// Sound category grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCategory {
    Master,
    Music,
    Record,
    Weather,
    Block,
    Hostile,
    Neutral,
    Player,
    Ambient,
    Voice,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_list_non_empty() {
        assert!(!all_sound_events().is_empty());
    }
}
