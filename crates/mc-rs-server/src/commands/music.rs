//! Port de PMMP `src/command/defaults/MusicCommand.php` — voir
//! .reference/PocketMine-MP/src/command/defaults/ pour la sémantique vanilla.

use mc_rs_command::{
    hard_enum_param, param, usage, CommandDefinition, CommandInvocation, CommandOverload,
    ParamType, PermissionDefault, PermissionRegistry,
};

use super::{register_command, ServerCommandMap, ServerCommandRuntime};

pub(super) fn register(permissions: &mut PermissionRegistry, map: &mut ServerCommandMap) {
    let mut music = CommandDefinition::new("music", "Control music playback");
    music.usage = "/music play|stop|volume <track|amount> [volume]".into();
    music.permissions = vec!["server.command.music".into()];
    music.overloads.push(CommandOverload {
        parameters: vec![
            hard_enum_param("action", "music_action", &["play", "stop", "volume"], false),
            param("arg", ParamType::String, true),
            param("volume", ParamType::Float, true),
        ],
    });
    register_command(
        permissions,
        map,
        music,
        PermissionDefault::Op,
        |runtime: &mut dyn ServerCommandRuntime, invocation: &CommandInvocation| {
            let Some(action) = invocation.arg(0) else {
                return usage("Usage: /music play|stop|volume <track|amount> [volume]");
            };
            let action = action.to_ascii_lowercase();
            // Broadcast (pas de target select pour /music vanilla — tous les
            // joueurs du serveur).
            let center = if runtime.sender_is_player() {
                runtime.sender_position()
            } else {
                [0.0, 64.0, 0.0]
            };
            match action.as_str() {
                "play" => {
                    let Some(track) = invocation.arg(1) else {
                        return usage("Usage: /music play <track> [volume]");
                    };
                    let volume: f32 = invocation
                        .arg(2)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                    // Préfixe `music.` si absent — convention Bedrock.
                    let sound = if track.contains('.') {
                        track.to_string()
                    } else {
                        format!("music.{track}")
                    };
                    runtime.play_sound(&[], &sound, center, volume, 1.0);
                    runtime.send_feedback(&format!("Now playing: {sound}"));
                }
                "stop" => {
                    // Stop tout son côté client (paramètre None = stop_all).
                    runtime.stop_sound(&[], None);
                    runtime.send_feedback("Stopped music.");
                }
                "volume" => {
                    // Pas de mécanisme vanilla pour ajuster un son déjà en
                    // cours — on accepte la commande mais on ne change rien.
                    runtime.send_feedback(
                        "Music volume command accepted (no-op — re-play to change volume).",
                    );
                }
                _ => return usage("Action must be play, stop, or volume."),
            }
            Ok(())
        },
    );
}
