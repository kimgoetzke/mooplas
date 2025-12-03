use crate::app_states::AppState;
use crate::prelude::{
  AvailablePlayerConfigs, ContinueMessage, NetworkRole, PlayerId, PlayerRegistrationMessage, RegisteredPlayer,
  RegisteredPlayers, SnakeHead, WinnerInfo, has_registered_players,
};
use crate::shared::{InputAction, Player};
use avian2d::prelude::Collisions;
use bevy::app::{App, Plugin};
use bevy::ecs::entity::Entity;
use bevy::prelude::*;

/// A plugin that manages the main game loop.
pub struct GameLoopPlugin;

impl Plugin for GameLoopPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        handle_continue_message
          .run_if(in_state(AppState::Registering))
          .run_if(has_registered_players)
          .run_if(|role: Res<NetworkRole>| role.is_server() || role.is_none()),
      )
      .add_systems(
        Update,
        player_registration_system.run_if(in_state(AppState::Registering)),
      )
      .add_systems(
        Update,
        (check_snake_collisions_system, transition_to_game_over_system).run_if(in_state(AppState::Playing)),
      )
      .add_systems(OnEnter(AppState::GameOver), pause_game_system)
      .add_systems(
        Update,
        game_over_to_initialising_transition_system.run_if(in_state(AppState::GameOver)),
      )
      .add_systems(
        OnExit(AppState::GameOver),
        (unpause_game_system, despawn_players_system, reset_for_lobby_system),
      );
  }
}

/// Resets the registered players and winner information when entering the lobby/registering state.
fn reset_for_lobby_system(mut registered: ResMut<RegisteredPlayers>, mut winner: ResMut<WinnerInfo>) {
  registered.players.clear();
  winner.clear();
}

/// Handles player registration messages to add or remove players from the registered players list.
fn player_registration_system(
  mut input_action_messages: MessageReader<InputAction>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut player_registration_message: MessageWriter<PlayerRegistrationMessage>,
  network_role: Res<NetworkRole>,
) {
  for input_action in input_action_messages.read() {
    if let InputAction::Action(player_id) = input_action {
      let Some(available_config) = available_configs.configs.iter().find(|config| config.id == *player_id) else {
        warn!("Received registration action for unknown player ID [{:?}]", player_id);
        continue;
      };

      let is_now_registered = if let Some(position) = registered_players
        .players
        .iter()
        .position(|p| p.id == available_config.id)
      {
        // Unregister
        registered_players.players.remove(position);
        debug!("[Player {}] has unregistered", available_config.id.0);
        false
      } else {
        // Register
        registered_players.players.push(RegisteredPlayer {
          id: available_config.id,
          input: available_config.input.clone(),
          colour: available_config.colour,
          alive: true,
        });
        debug!("[Player {}] has registered", available_config.id.0);
        true
      };

      player_registration_message.write(PlayerRegistrationMessage {
        player_id: available_config.id,
        has_registered: is_now_registered,
        is_anyone_registered: !registered_players.players.is_empty(),
        network_audience: (*network_role).into(),
      });
    }
  }
}

/// Transitions the game from the registration/lobby state to the running state.
fn handle_continue_message(
  mut continue_messages: MessageReader<ContinueMessage>,
  mut next_app_state: ResMut<NextState<AppState>>,
) {
  let messages = continue_messages.read().collect::<Vec<&ContinueMessage>>();
  if messages.is_empty() {
    return;
  }
  next_app_state.set(AppState::Playing);
}

/// Checks for collisions involving snake heads and marks players as dead if they collide.
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
                debug!("[{:?}] collided with [{:?}]", player.id, other_player_id);
              } else {
                debug!("[{:?}] collided with themselves", player.id);
              }
              player.alive = false;
            } else {
              error!("[{:?}] collided with non-tail entity [{:?}]", player.id, other_entity);
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

/// Transitions to the next game state if there are no alive players or only one alive player remaining.
fn transition_to_game_over_system(
  registered_players: ResMut<RegisteredPlayers>,
  mut winner: ResMut<WinnerInfo>,
  mut next: ResMut<NextState<AppState>>,
) {
  let alive_players: Vec<&RegisteredPlayer> = registered_players.players.iter().filter(|p| p.alive).collect();
  match (registered_players.players.len(), alive_players.len()) {
    (_, 0) => {
      winner.clear();
      next.set(AppState::GameOver);
      info!("Game over: No winner this round.");
    }
    (registered_players, 1) if registered_players > 1 => {
      winner.set(alive_players[0].id);
      next.set(AppState::GameOver);
      info!("Game over: [{:?}] wins the round", alive_players[0].id);
    }
    _ => {}
  }
}

/// Pauses the game time when called. Intended to be called when entering the game over state.
fn pause_game_system(mut time: ResMut<Time<Virtual>>) {
  time.pause();
}

/// Unpauses the game time when called. Intended to be called when exiting the game over state.
fn unpause_game_system(mut time: ResMut<Time<Virtual>>) {
  time.unpause();
}

fn game_over_to_initialising_transition_system(
  mut continue_messages: MessageReader<ContinueMessage>,
  mut next_app_state: ResMut<NextState<AppState>>,
) {
  debug_once!("Waiting for message to continue...");
  let messages = continue_messages.read().collect::<Vec<&ContinueMessage>>();
  if messages.is_empty() {
    return;
  }
  next_app_state.set(AppState::Initialising);
}

/// Despawns all player entities. Intended to be called when exiting the game over state.
fn despawn_players_system(mut commands: Commands, players_query: Query<Entity, With<Player>>) {
  for entity in &players_query {
    commands.entity(entity).despawn();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::{AvailablePlayerConfig, PlayerInput, SharedMessagesPlugin, SharedResourcesPlugin};
  use bevy::state::app::StatesPlugin;

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Add logging and state machinery so systems that require NextState<AppState> work in tests
    app.add_plugins((StatesPlugin, crate::app_states::AppStatePlugin));
    // Add shared messages and resources as they are required by the game loop systems
    app.add_plugins((SharedMessagesPlugin, SharedResourcesPlugin));
    app
  }

  #[test]
  fn player_registration_registers_and_unregisters() {
    let mut app = setup();

    // Prepare an available player config
    let mut available_configs = app
      .world_mut()
      .get_resource_mut::<AvailablePlayerConfigs>()
      .expect("AvailablePlayerConfigs resource missing");
    available_configs.configs.push(AvailablePlayerConfig {
      id: PlayerId(0),
      input: PlayerInput::new(PlayerId(0), KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD),
      colour: Color::WHITE,
    });
    drop(available_configs);

    // Send an input action to register the player
    app
      .world_mut()
      .write_message(InputAction::Action(PlayerId(0)))
      .expect("Failed to write InputAction message");

    // Add and run the registration system once
    app.add_systems(Update, player_registration_system);
    app.update();

    // Player should now be registered
    let registered_players = app
      .world()
      .get_resource::<RegisteredPlayers>()
      .expect("RegisteredPlayers missing");
    assert_eq!(registered_players.players.len(), 1);
    assert_eq!(registered_players.players[0].id, PlayerId(0));

    // Send action again to unregister
    app
      .world_mut()
      .write_message(InputAction::Action(PlayerId(0)))
      .expect("Failed to write InputAction message");
    app.update();

    let registered_players = app
      .world()
      .get_resource::<RegisteredPlayers>()
      .expect("RegisteredPlayers missing");
    assert!(registered_players.players.is_empty());
  }

  #[cfg(not(feature = "online"))]
  #[test]
  fn handle_continue_message_transitions_to_playing() {
    let mut app = setup();

    // Ensure we start in the initialising state
    let state = app
      .world()
      .get_resource::<State<AppState>>()
      .expect("AppState State resource missing");
    assert_eq!(state, &AppState::Initialising);

    // Send the message
    app
      .world_mut()
      .write_message(ContinueMessage)
      .expect("Failed to write ContinueMessage");
    app.add_systems(Update, handle_continue_message);

    // Run two updates to ensure state transition occurs
    app.update();
    app.update();

    let state = app
      .world()
      .get_resource::<State<AppState>>()
      .expect("AppState State resource missing");
    assert_eq!(state, &AppState::Playing);
  }

  #[test]
  fn transition_to_game_over_sets_winner_when_one_alive_remains() {
    let mut app = setup();

    // Register two players, one alive and one dead
    let mut registered_players = app
      .world_mut()
      .get_resource_mut::<RegisteredPlayers>()
      .expect("RegisteredPlayers missing");
    registered_players.players = vec![
      RegisteredPlayer {
        id: PlayerId(0),
        input: PlayerInput::new(PlayerId(0), KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD),
        colour: Color::WHITE,
        alive: false,
      },
      RegisteredPlayer {
        id: PlayerId(1),
        input: PlayerInput::new(PlayerId(1), KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyL),
        colour: Color::BLACK,
        alive: true,
      },
    ];
    drop(registered_players);

    // Add and run the transition to game over system
    app.add_systems(Update, transition_to_game_over_system);
    app.update();

    // Verify that we now have a winner set
    let winner = app.world().get_resource::<WinnerInfo>().expect("WinnerInfo missing");
    assert_eq!(winner.get(), Some(PlayerId(1)));
  }
}
