use crate::app_states::AppState;
use crate::player::SnakeTail;
use crate::prelude::{PlayerId, RegisteredPlayer, RegisteredPlayers, SnakeHead, WinnerInfo};
use bevy::app::{App, Plugin};
use bevy::prelude::*;

/// A plugin that manages the main game loop.
pub struct GameLoopPlugin;

impl Plugin for GameLoopPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(OnEnter(AppState::Registering), reset_for_lobby_system)
      .add_systems(
        Update,
        (check_snake_collisions_system, transition_to_game_over_system).run_if(in_state(AppState::Playing)),
      )
      .add_systems(OnExit(AppState::Playing), despawn_players_system);
  }
}

fn reset_for_lobby_system(mut registered: ResMut<RegisteredPlayers>, mut winner: ResMut<WinnerInfo>) {
  registered.players.clear();
  winner.winner = None;
}

fn check_snake_collisions_system(
  mut registered: ResMut<RegisteredPlayers>,
  heads: Query<(&PlayerId, &Transform), With<SnakeHead>>,
  tails: Query<(&PlayerId, &Transform), With<SnakeTail>>,
) {
  // TODO: Replace with actual collision detection
  let _ = (&mut *registered, &heads, &tails);
}

fn transition_to_game_over_system(
  registered_players: ResMut<RegisteredPlayers>,
  mut winner: ResMut<WinnerInfo>,
  mut next: ResMut<NextState<AppState>>,
) {
  let alive_players: Vec<&RegisteredPlayer> = registered_players.players.iter().filter(|p| p.alive).collect();
  match (registered_players.players.len(), alive_players.len()) {
    (_, 0) => {
      winner.winner = None;
      next.set(AppState::GameOver);
    }
    (registered_players, 1) if registered_players > 1 => {
      winner.winner = Some(alive_players[0].id);
      next.set(AppState::GameOver);
    }
    _ => {}
  }
}

fn despawn_players_system(
  mut commands: Commands,
  heads: Query<Entity, With<SnakeHead>>,
  tails: Query<Entity, With<SnakeTail>>,
) {
  for entity in &heads {
    commands.entity(entity).despawn();
  }
  for entity in &tails {
    commands.entity(entity).despawn();
  }
}
