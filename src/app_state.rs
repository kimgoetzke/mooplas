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
  /// The state in which the application starts, used to load initial resources that do not change throughout the
  /// application lifetime.
  #[default]
  Loading,
  /// The state used for menus. Time may be paused.
  Preparing,
  /// A state which runs after a game mode has been selected and initialises resources such as spawn points.
  /// Automatically transitions to the next state once complete.
  Initialising,
  /// The state where players can register to join the game.
  Registering,
  /// The main gameplay state.
  Playing,
  /// The state after a game has finished. Time may be paused.
  GameOver,
  /// A placeholder state for undefined states. Should never occur.
  Error,
}

impl AppState {
  pub fn name() -> &'static str {
    "AppState"
  }

  /// Returns true if the current state is considered to be restricted. This includes states that the application
  /// automatically transitions to. Used to stop the server in a multiplayer context from causing an inconsistent state.
  #[cfg(feature = "online")]
  pub fn is_restricted(&self) -> bool {
    matches!(self, AppState::Initialising)
  }
}

impl Display for AppState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", format!("{:?}", self))
  }
}

impl From<&String> for AppState {
  fn from(state_name: &String) -> Self {
    match state_name.as_str() {
      "Loading" => AppState::Loading,
      "Preparing" => AppState::Preparing,
      "Initialising" => AppState::Initialising,
      "Registering" => AppState::Registering,
      "Playing" => AppState::Playing,
      "GameOver" => AppState::GameOver,
      _ => AppState::Error,
    }
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
    assert_eq!(state.unwrap(), &AppState::Loading);
  }

  #[test]
  fn app_state_name_returns_correct_value() {
    assert_eq!(AppState::name(), "AppState");
  }

  #[test]
  fn app_state_display_formats_correctly() {
    assert_eq!(AppState::Loading.to_string(), "Loading");
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

  #[test]
  fn from_string_converts_valid_state_names() {
    assert_eq!(AppState::from(&"Loading".to_string()), AppState::Loading);
    assert_eq!(AppState::from(&"Initialising".to_string()), AppState::Initialising);
    assert_eq!(AppState::from(&"Preparing".to_string()), AppState::Preparing);
    assert_eq!(AppState::from(&"Registering".to_string()), AppState::Registering);
    assert_eq!(AppState::from(&"Playing".to_string()), AppState::Playing);
    assert_eq!(AppState::from(&"GameOver".to_string()), AppState::GameOver);
  }

  #[test]
  fn from_string_defaults_to_error_for_invalid_state_names() {
    assert_eq!(AppState::from(&"InvalidState".to_string()), AppState::Error);
    assert_eq!(AppState::from(&"".to_string()), AppState::Error);
    assert_eq!(AppState::from(&"123".to_string()), AppState::Error);
  }
}
