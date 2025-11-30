use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::prelude::{AppExtStates, MessageReader, State, StateTransitionEvent, States};
use bevy::reflect::Reflect;
use std::fmt::Display;

/// A plugin that introduces and manages the main application states.
pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
  fn build(&self, app: &mut App) {
    app
      .init_state::<AppState>()
      .register_type::<State<AppState>>()
      .add_systems(Update, log_app_state_transitions_system);
  }
}

fn name_from<T: ToString>(state: Option<T>) -> String {
  match state {
    Some(state_name) => state_name.to_string(),
    None => "None".to_string(),
  }
}

/// The main application states for this application. Drives the overall flow of the game.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum AppState {
  /// The initialisation state which loads shared resources. Runs at application start and after before entering the
  /// registering state.
  #[default]
  Initialising,
  /// The state used for menus. Time may be paused.
  Preparing,
  /// The state where players can register to join the game.
  Registering,
  /// The main gameplay state.
  Playing,
  /// The state after a game has finished. Time may be paused.
  GameOver,
}

impl AppState {
  pub fn name() -> &'static str {
    "AppState"
  }
}

impl Display for AppState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", format!("{:?}", self))
  }
}

fn log_app_state_transitions_system(mut app_state_messages: MessageReader<StateTransitionEvent<AppState>>) {
  for message in app_state_messages.read() {
    info!(
      "Transitioning [{}] from [{}] to [{}]",
      AppState::name(),
      name_from(message.exited),
      name_from(message.entered)
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bevy::app::App;
  use bevy::log::LogPlugin;
  use bevy::prelude::*;
  use bevy::state::app::StatesPlugin;

  #[test]
  fn app_state_plugin_initialises_states() {
    let mut app = App::new();
    app.add_plugins((LogPlugin::default(), StatesPlugin));
    app.add_plugins(AppStatePlugin);

    let state = app.world().get_resource::<State<AppState>>();
    assert!(state.is_some());
    assert_eq!(state.unwrap(), &AppState::Initialising);
  }

  #[test]
  fn app_state_name_returns_correct_value() {
    assert_eq!(AppState::name(), "AppState");
  }

  #[test]
  fn app_state_display_formats_correctly() {
    assert_eq!(AppState::Initialising.to_string(), "Initialising");
    assert_eq!(AppState::Registering.to_string(), "Registering");
    assert_eq!(AppState::Playing.to_string(), "Playing");
    assert_eq!(AppState::GameOver.to_string(), "GameOver");
  }

  #[test]
  fn name_from_handles_some_state() {
    let state_name = name_from(Some(AppState::Playing));
    assert_eq!(state_name, "Playing");
  }

  #[test]
  fn name_from_handles_none_state() {
    let state_name = name_from::<AppState>(None);
    assert_eq!(state_name, "None");
  }
}
