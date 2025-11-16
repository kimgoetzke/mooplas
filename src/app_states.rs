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

fn name_from<T: ToString>(state: Option<T>) -> String {
  match state {
    Some(state_name) => state_name.to_string(),
    None => "None".to_string(),
  }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum AppState {
  /// The initialisation state which loads shared resources. Only runs once at application start.
  #[default]
  Initialising,
  /// The state where players can register to join the game.
  Registering,
  /// The main gameplay state.
  Playing,
  /// The state after a game has finished.
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
