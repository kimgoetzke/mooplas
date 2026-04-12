#[cfg(feature = "online")]
use crate::prelude::LocalPlayerRegistrationRequestMessage;
use crate::prelude::constants::{RESOLUTION_HEIGHT, RESOLUTION_WIDTH};
use crate::prelude::{
  AppState, AvailableControlSchemes, ContinueMessage, ControlSchemeId, ExitLobbyMessage, PlayerId,
  PlayerRegistrationMessage, RegisteredPlayer, RegisteredPlayers, SnakeHead, WinnerInfo, colour_for_player_id,
  has_registered_players,
};
use crate::shared::{InputMessage, Player};
use avian2d::prelude::Collisions;
use bevy::app::{App, Plugin};
use bevy::ecs::entity::Entity;
use bevy::prelude::*;
use mooplas_networking::prelude::NetworkRole;

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
        handle_exit_lobby_message
          .run_if(in_state(AppState::Registering))
          .run_if(|role: Res<NetworkRole>| !role.is_server()),
      )
      .add_systems(
        Update,
        (
          check_snake_collisions_system,
          check_screen_bounds_collisions_system,
          transition_to_game_over_system,
        )
          .run_if(in_state(AppState::Playing))
          .run_if(|role: Res<NetworkRole>| role.is_server() || role.is_none()),
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

/// Transitions the game from the registration/lobby state to the running state.
fn handle_continue_message(
  mut messages: MessageReader<ContinueMessage>,
  mut next_app_state: ResMut<NextState<AppState>>,
) {
  if messages.read().collect::<Vec<&ContinueMessage>>().is_empty() {
    return;
  }
  next_app_state.set(AppState::Playing);
}

/// Handles exit lobby messages by transitioning back to the menu.
fn handle_exit_lobby_message(
  mut messages: MessageReader<ExitLobbyMessage>,
  mut next_app_state: ResMut<NextState<AppState>>,
) {
  if messages.read().collect::<Vec<&ExitLobbyMessage>>().is_empty() {
    return;
  }
  next_app_state.set(AppState::Preparing);
}

/// Handles player registration messages to add or remove players from the registered players list.
fn player_registration_system(
  mut input_messages: MessageReader<InputMessage>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_control_schemes: Res<AvailableControlSchemes>,
  #[cfg(feature = "online")] mut local_player_registration_request_message: MessageWriter<
    LocalPlayerRegistrationRequestMessage,
  >,
  mut player_registration_message: MessageWriter<PlayerRegistrationMessage>,
  _network_role: Res<NetworkRole>,
) {
  for input_action in input_messages.read() {
    if let InputMessage::Action(player_id) = input_action {
      let control_scheme_id = ControlSchemeId(player_id.0);
      let Some(control_scheme) = available_control_schemes.find_by_id(control_scheme_id) else {
        warn!(
          "Received registration action for unknown control scheme [{:?}]",
          control_scheme_id
        );
        continue;
      };

      #[cfg(feature = "online")]
      if _network_role.is_client() || _network_role.is_server() {
        let is_registered_locally = registered_players
          .players
          .iter()
          .any(|registered_player| registered_player.is_local() && registered_player.input.id == control_scheme_id);
        local_player_registration_request_message.write(LocalPlayerRegistrationRequestMessage {
          control_scheme_id,
          has_registered: !is_registered_locally,
        });
        continue;
      }

      let affected_player_id = if let Some(registered_player_id) = registered_players
        .players
        .iter()
        .find(|registered_player| registered_player.id == *player_id)
        .map(|registered_player| registered_player.id)
      {
        // Unregister
        match registered_players.unregister_mutable(registered_player_id) {
          Ok(_) => debug!("[Player {}] has unregistered", player_id.0),
          Err(e) => {
            warn!("Failed to unregister [Player {}]: {}", player_id.0, e);
            continue;
          }
        }

        registered_player_id
      } else {
        // Register — in local mode, ControlSchemeId maps implicitly to PlayerId
        let colour = colour_for_player_id(*player_id);
        match registered_players.register(RegisteredPlayer::new_mutable(
          *player_id,
          control_scheme.clone(),
          colour,
        )) {
          Ok(_) => debug!("[Player {}] has registered", player_id.0),
          Err(e) => {
            warn!("Failed to register [Player {}]: {}", player_id.0, e);
            continue;
          }
        }

        *player_id
      };

      player_registration_message.write(PlayerRegistrationMessage {
        player_id: affected_player_id,
        control_scheme_id: Some(control_scheme_id),
        is_anyone_registered: !registered_players.players.is_empty(),
      });
    }
  }
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

/// Checks whether any snake head is "touching" the bounds. If so, mark the corresponding player as dead.
fn check_screen_bounds_collisions_system(
  mut registered_players: ResMut<RegisteredPlayers>,
  snake_head_query: Query<(&GlobalTransform, &PlayerId), With<SnakeHead>>,
) {
  let half_width = RESOLUTION_WIDTH as f32 / 2.;
  let half_height = RESOLUTION_HEIGHT as f32 / 2.;

  for (global_transform, player_id) in snake_head_query.iter() {
    let position = global_transform.translation();
    if position.x.abs() > half_width || position.y.abs() > half_height {
      if let Some(player) = registered_players.players.iter_mut().find(|p| p.id == *player_id) {
        debug!(
          "Player [{:?}] left bounds at position {:?} and is eliminated",
          player.id, position
        );
        player.alive = false;
      }
    }
  }
}

/// Pauses the game time when called. Intended to be called when entering the game over state.
fn pause_game_system(mut time: ResMut<Time<Virtual>>) {
  time.pause();
}

/// Transitions to the next game state if there are no alive players or only one alive player remaining.
fn transition_to_game_over_system(
  registered_players: ResMut<RegisteredPlayers>,
  mut winner: ResMut<WinnerInfo>,
  mut next_app_state: ResMut<NextState<AppState>>,
) {
  let alive_players: Vec<&RegisteredPlayer> = registered_players.players.iter().filter(|p| p.alive).collect();
  match (registered_players.count(), alive_players.len()) {
    (_, 0) => {
      winner.clear();
      next_app_state.set(AppState::GameOver);
      info!("Game over: No winner this round.");
    }
    (registered_players, 1) if registered_players > 1 => {
      winner.set(alive_players[0].id);
      next_app_state.set(AppState::GameOver);
      info!("Game over: [{:?}] wins the round", alive_players[0].id);
    }
    _ => {}
  }
}

/// Unpauses the game time when called. Intended to be called when exiting the game over state.
fn unpause_game_system(mut time: ResMut<Time<Virtual>>) {
  time.unpause();
}

fn game_over_to_initialising_transition_system(
  mut continue_messages: MessageReader<ContinueMessage>,
  mut next_app_state: ResMut<NextState<AppState>>,
) {
  debug_once!("Waiting for message or host to continue...");
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

/// Resets the registered players and winner information when entering the lobby/registering state.
fn reset_for_lobby_system(mut registered: ResMut<RegisteredPlayers>, mut winner: ResMut<WinnerInfo>) {
  registered.players.clear();
  winner.clear();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::{
    AvailableControlSchemes, ControlScheme, ControlSchemeId, SharedMessagesPlugin, SharedResourcesPlugin,
  };
  use bevy::state::app::StatesPlugin;

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Add logging and state machinery so systems that require NextState<AppState> work in tests
    app.add_plugins((StatesPlugin, crate::app_state::AppStatePlugin));
    // Add shared messages and resources as they are required by the game loop systems
    app.add_plugins((SharedMessagesPlugin, SharedResourcesPlugin));
    // Add a NetworkRole resource so systems that require it can run
    app.init_resource::<NetworkRole>();
    app
  }

  #[test]
  fn player_registration_registers_and_unregisters() {
    let mut app = setup();

    // Prepare an available control scheme
    {
      let mut available_schemes = app
        .world_mut()
        .get_resource_mut::<AvailableControlSchemes>()
        .expect("AvailableControlSchemes resource missing");
      available_schemes.schemes.push(ControlScheme::new(
        ControlSchemeId(0),
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
      ));
    }

    // Send an input action to register the player
    app
      .world_mut()
      .write_message(InputMessage::Action(PlayerId(0)))
      .expect("Failed to write InputAction message");

    // Add and run the registration system once
    app.add_systems(Update, player_registration_system);
    app.update();

    {
      let mut registration_messages = app
        .world_mut()
        .get_resource_mut::<Messages<PlayerRegistrationMessage>>()
        .expect("Messages<PlayerRegistrationMessage> missing");
      let queued_registration_messages: Vec<_> =
        registration_messages.iter_current_update_messages().copied().collect();
      assert_eq!(queued_registration_messages.len(), 1);
      assert_eq!(
        queued_registration_messages[0],
        PlayerRegistrationMessage {
          player_id: PlayerId(0),
          control_scheme_id: Some(ControlSchemeId(0)),
          is_anyone_registered: true,
        }
      );
      registration_messages.drain().for_each(drop);
    }

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
      .write_message(InputMessage::Action(PlayerId(0)))
      .expect("Failed to write InputAction message");
    app.update();

    {
      let mut registration_messages = app
        .world_mut()
        .get_resource_mut::<Messages<PlayerRegistrationMessage>>()
        .expect("Messages<PlayerRegistrationMessage> missing");
      let queued_registration_messages: Vec<_> =
        registration_messages.iter_current_update_messages().copied().collect();
      assert_eq!(queued_registration_messages.len(), 1);
      assert_eq!(
        queued_registration_messages[0],
        PlayerRegistrationMessage {
          player_id: PlayerId(0),
          control_scheme_id: Some(ControlSchemeId(0)),
          is_anyone_registered: false,
        }
      );
      registration_messages.drain().for_each(drop);
    }

    let registered_players = app
      .world()
      .get_resource::<RegisteredPlayers>()
      .expect("RegisteredPlayers missing");
    assert!(registered_players.players.is_empty());
  }

  #[cfg(feature = "online")]
  #[test]
  fn player_registration_in_client_mode_does_not_register_locally() {
    let mut app = setup();
    *app.world_mut().resource_mut::<NetworkRole>() = NetworkRole::Client;

    {
      let mut available_schemes = app
        .world_mut()
        .get_resource_mut::<AvailableControlSchemes>()
        .expect("AvailableControlSchemes resource missing");
      available_schemes.schemes.push(ControlScheme::new(
        ControlSchemeId(0),
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
      ));
    }

    app
      .world_mut()
      .write_message(InputMessage::Action(PlayerId(0)))
      .expect("Failed to write InputAction message");

    app.add_systems(Update, player_registration_system);
    app.update();

    let registered_players = app
      .world()
      .get_resource::<RegisteredPlayers>()
      .expect("RegisteredPlayers missing");
    assert!(registered_players.players.is_empty());

    let registration_messages = app
      .world_mut()
      .get_resource_mut::<Messages<PlayerRegistrationMessage>>()
      .expect("Messages<PlayerRegistrationMessage> missing");
    assert_eq!(registration_messages.iter_current_update_messages().count(), 0);

    let local_registration_requests = app
      .world_mut()
      .get_resource_mut::<Messages<LocalPlayerRegistrationRequestMessage>>()
      .expect("Messages<LocalPlayerRegistrationRequestMessage> missing");
    assert_eq!(local_registration_requests.iter_current_update_messages().count(), 1);
  }

  #[cfg(not(feature = "online"))]
  #[test]
  fn handle_continue_message_transitions_to_playing() {
    let mut app = setup();

    // Ensure we start in the loading state.
    let state = app
      .world()
      .get_resource::<State<AppState>>()
      .expect("AppState State resource missing");
    assert_eq!(state, &AppState::Loading);

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
      RegisteredPlayer::new_mutable_dead(
        PlayerId(0),
        ControlScheme::new(ControlSchemeId(0), KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD),
        Color::WHITE,
      ),
      RegisteredPlayer::new_mutable(
        PlayerId(1),
        ControlScheme::new(ControlSchemeId(1), KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyL),
        Color::BLACK,
      ),
    ];
    let _ = registered_players;

    // Add and run the transition to game over system
    app.add_systems(Update, transition_to_game_over_system);
    app.update();

    // Verify that we now have a winner set
    let winner = app.world().get_resource::<WinnerInfo>().expect("WinnerInfo missing");
    assert_eq!(winner.get(), Some(PlayerId(1)));
  }
}
