use crate::app_states::AppState;
use crate::prelude::{PlayerId, RegisteredPlayer, RegisteredPlayers, SnakeHead, SnakeTail, WinnerInfo};
use avian2d::prelude::Collisions;
use bevy::app::{App, Plugin};
use bevy::ecs::entity::Entity;
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
      .add_systems(OnEnter(AppState::GameOver), pause_game)
      .add_systems(OnExit(AppState::GameOver), (unpause_game, despawn_players_system));
  }
}

fn reset_for_lobby_system(mut registered: ResMut<RegisteredPlayers>, mut winner: ResMut<WinnerInfo>) {
  registered.players.clear();
  winner.winner = None;
}

fn check_snake_collisions_system(
  mut registered_players: ResMut<RegisteredPlayers>,
  collisions: Collisions,
  snake_head_query: Query<&PlayerId, With<SnakeHead>>,
  player_id_query: Query<&PlayerId>,
  parent_query: Query<&ChildOf>,
) {
  let resolve_player_id = |start: Entity| -> Option<PlayerId> {
    let mut current = start;
    loop {
      if let Ok(pid) = player_id_query.get(current) {
        return Some(*pid);
      }
      match parent_query.get(current) {
        Ok(parent) => current = parent.0,
        Err(_) => return None,
      }
    }
  };

  for contact_pair in collisions.iter() {
    let a = contact_pair.collider1;
    let b = contact_pair.collider2;

    let mut process_pair = |this_entity: Entity, other_entity: Entity| {
      if let Some(this_player_id) = resolve_player_id(this_entity) {
        if let Ok(_) = snake_head_query.get(this_entity) {
          if let Some(player) = registered_players.players.iter_mut().find(|p| p.id == this_player_id) {
            if let Some(other_player_id) = resolve_player_id(other_entity) {
              if other_player_id.0 != this_player_id.0 {
                debug!("Player [{:?}] collided with player [{:?}]", player.id, other_player_id);
              } else {
                debug!("Player [{:?}] collided with themselves", player.id);
              }
              player.alive = false;
            } else {
              error!(
                "Player [{:?}] collided with non-tail entity [{:?}]",
                player.id, other_entity
              );
            }
          } else {
            error!("Cannot find alive player for head entity [{:?}]", this_entity);
            return;
          }
        }
      }
    };

    process_pair(a, b);
    process_pair(b, a);
  }
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

fn pause_game(mut time: ResMut<Time<Virtual>>) {
  time.pause();
}

fn unpause_game(mut time: ResMut<Time<Virtual>>) {
  time.unpause();
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
