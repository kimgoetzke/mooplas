use crate::app_states::AppState;
use bevy::prelude::{App, NextState, Plugin, ResMut, Startup};

/// A plugin responsible for loading shared assets and transitioning to the next app state. This plugin is intended to
/// be run once at the start of the application.
pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, change_state_system);
  }
}

// TODO: Add asset loading, possibly loading screen
fn change_state_system(mut next_state: ResMut<NextState<AppState>>) {
  next_state.set(AppState::Preparing);
}
