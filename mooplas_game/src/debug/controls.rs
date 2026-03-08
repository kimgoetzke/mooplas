use crate::prelude::{DebugStateMessage, GeneralSettings, Settings};
use bevy::app::{App, Plugin, Update};
use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::prelude::{KeyCode, MessageWriter, Res, ResMut};

/// A plugin that manages debug related controls that do not exist in release mode.
pub struct DebugControlsPlugin;

impl Plugin for DebugControlsPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, debug_controls_system);
  }
}

/// A system that handles debug related controls such as toggling gizmos.
fn debug_controls_system(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  mut settings: ResMut<Settings>,
  mut general_settings: ResMut<GeneralSettings>,
  mut debug_state_message: MessageWriter<DebugStateMessage>,
) {
  if keyboard_input.just_pressed(KeyCode::F9) {
    settings.general.display_player_gizmos = !settings.general.display_player_gizmos;
    general_settings.display_player_gizmos = settings.general.display_player_gizmos;
    info!(
      "[F9] Set display player gizmos to [{}]",
      settings.general.display_player_gizmos
    );
    debug_state_message.write(DebugStateMessage {
      display_player_gizmos: settings.general.display_player_gizmos,
    });
  }
}
